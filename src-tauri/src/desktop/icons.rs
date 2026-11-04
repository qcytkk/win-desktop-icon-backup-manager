use std::ffi::c_void;
use std::path::Path;
use std::thread;

use sha1::{Digest, Sha1};
use windows::core::{GUID, Interface};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, SelectObject, BITMAP,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IServiceProvider, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::SystemServices::{SFGAO_FLAGS, SFGAO_FOLDER};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};
use windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellFolder, IShellItem, IShellItemImageFactory, IShellView,
    IShellWindows, SHCreateItemWithParent, SHGDN_INFOLDER, SHGetDesktopFolder,
    SID_STopLevelBrowser, SIIGBF_ICONONLY, SIGDN_DESKTOPABSOLUTEPARSING, SVGIO_ALLVIEW,
    SWC_DESKTOP, SWFO_NEEDDISPATCH, StrRetToBufW,
};

use crate::models::IconItem;

const CLSID_SHELL_WINDOWS: GUID = GUID::from_u128(0x9BA05972_F6A8_11CF_A442_00A0C90A8F39);

pub struct Collected {
    pub icons: Vec<IconItem>,
    pub source: String,
    pub warnings: Vec<String>,
}

pub fn collect(icon_dir: &Path) -> Result<Collected, String> {
    let dir = icon_dir.to_path_buf();
    thread::spawn(move || unsafe { collect_sta(&dir) })
        .join()
        .map_err(|_| "桌面图标采集线程异常".to_string())?
}

unsafe fn collect_sta(icon_dir: &Path) -> Result<Collected, String> {
    let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let initialized = init.is_ok();

    let result = collect_inner(icon_dir);
    if initialized {
        CoUninitialize();
    }
    result
}

pub struct DesktopView {
    pub folder_view: IFolderView,
    pub desktop: IShellFolder,
}

pub unsafe fn open_desktop_view() -> Result<DesktopView, String> {
    let desktop: IShellFolder =
        SHGetDesktopFolder().map_err(|e| format!("无法获取桌面文件夹：{}", e))?;

    let shell_windows: IShellWindows = CoCreateInstance(&CLSID_SHELL_WINDOWS, None, CLSCTX_ALL)
        .map_err(|e| format!("无法创建 ShellWindows：{}", e))?;

    let empty = VARIANT::default();
    let mut hwnd = 0i32;
    let dispatch = shell_windows
        .FindWindowSW(
            &empty,
            &empty,
            SWC_DESKTOP,
            &mut hwnd,
            SWFO_NEEDDISPATCH,
        )
        .map_err(|e| format!("找不到桌面 Shell 窗口：{}", e))?;

    let provider: IServiceProvider = dispatch
        .cast()
        .map_err(|e| format!("桌面对象不支持 IServiceProvider：{}", e))?;

    let browser: IShellBrowser = provider
        .QueryService(&SID_STopLevelBrowser as *const GUID)
        .map_err(|e| format!("无法获取桌面浏览器：{}", e))?;

    let view: IShellView = browser
        .QueryActiveShellView()
        .map_err(|e| format!("无法获取桌面视图：{}", e))?;

    let folder_view: IFolderView = view
        .cast()
        .map_err(|e| format!("桌面视图不支持 IFolderView：{}", e))?;

    Ok(DesktopView { folder_view, desktop })
}

pub unsafe fn pidl_display_name(
    desktop: &IShellFolder,
    pidl: *const ITEMIDLIST,
) -> Option<String> {
    let mut name_buf = [0u16; 520];
    let mut strret = STRRET::default();
    desktop
        .GetDisplayNameOf(pidl, SHGDN_INFOLDER, &mut strret)
        .ok()?;
    StrRetToBufW(&mut strret, Some(pidl), &mut name_buf).ok()?;
    let name = String::from_utf16_lossy(&name_buf)
        .trim_end_matches('\0')
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub unsafe fn pidl_path(desktop: &IShellFolder, pidl: *const ITEMIDLIST) -> Option<String> {
    let item: IShellItem = SHCreateItemWithParent(None, desktop, pidl).ok()?;
    get_parsing_name(&item)
}

unsafe fn collect_inner(icon_dir: &Path) -> Result<Collected, String> {
    let view = open_desktop_view()?;
    let folder_view = &view.folder_view;
    let desktop = &view.desktop;

    let count = folder_view
        .ItemCount(SVGIO_ALLVIEW)
        .map_err(|e| format!("无法统计桌面图标：{}", e))?;

    let mut icons = Vec::with_capacity(count.max(0) as usize);
    let mut warnings = Vec::new();
    let mut position_failures = 0usize;
    let mut index = 0i32;

    for i in 0..count {
        let pidl = match folder_view.Item(i) {
            Ok(p) => p,
            Err(_) => {
                warnings.push(format!("第 {} 个图标无法读取", i));
                continue;
            }
        };

        let display = match pidl_display_name(desktop, pidl as *const ITEMIDLIST) {
            Some(name) => name,
            None => {
                CoTaskMemFree(Some(pidl as *const c_void));
                continue;
            }
        };
        if display.eq_ignore_ascii_case("desktop.ini") {
            CoTaskMemFree(Some(pidl as *const c_void));
            continue;
        }

        let positioned = match folder_view.GetItemPosition(pidl) {
            Ok(point) => Some(point),
            Err(_) => {
                position_failures += 1;
                None
            }
        };

        let item: Option<IShellItem> = SHCreateItemWithParent(None, desktop, pidl).ok();
        let path = item
            .as_ref()
            .and_then(|it| get_parsing_name(it))
            .unwrap_or_else(|| display.clone());
        let hidden = item
            .as_ref()
            .map(|it| is_hidden(it))
            .unwrap_or(false);
        if hidden {
            CoTaskMemFree(Some(pidl as *const c_void));
            continue;
        }
        let kind = item
            .as_ref()
            .map(|it| kind_of(it, &path))
            .unwrap_or_else(|| "file".to_string());

        let raw_name = display.clone();
        let cleaned = crate::desktop::layout::clean_name(&raw_name);
        let display_name = crate::desktop::layout::clsid_display_name(&cleaned)
            .map(|s| s.to_string())
            .unwrap_or_else(|| cleaned.clone());

        let icon_key = item
            .as_ref()
            .and_then(|it| extract_icon(it, icon_dir, &path));

        icons.push(IconItem {
            index,
            raw_name,
            display_name,
            kind,
            path,
            x: positioned.map(|p| p.x).unwrap_or(-1),
            y: positioned.map(|p| p.y).unwrap_or(-1),
            icon_key,
        });
        index += 1;

        CoTaskMemFree(Some(pidl as *const c_void));
    }

    if position_failures > 0 {
        warnings.push(format!(
            "{} 个图标未取到位置，预览中按名称网格排列",
            position_failures
        ));
    }

    Ok(Collected {
        icons,
        source: "folder_view".into(),
        warnings,
    })
}

unsafe fn get_parsing_name(item: &IShellItem) -> Option<String> {
    let pw = item.GetDisplayName(SIGDN_DESKTOPABSOLUTEPARSING).ok()?;
    let name = pw.to_string().ok();
    CoTaskMemFree(Some(pw.0 as *const c_void));
    name
}

unsafe fn is_hidden(item: &IShellItem) -> bool {
    let attributes = item.GetAttributes(SFGAO_HIDDEN).unwrap_or(SFGAO_FLAGS(0));
    attributes.0 & SFGAO_HIDDEN.0 != 0
}

use windows::Win32::System::SystemServices::SFGAO_HIDDEN;

unsafe fn kind_of(item: &IShellItem, path: &str) -> String {
    if path.starts_with("::") {
        return "system".into();
    }
    let attributes = item.GetAttributes(SFGAO_FOLDER).unwrap_or(SFGAO_FLAGS(0));
    if attributes.0 & SFGAO_FOLDER.0 != 0 {
        return "folder".into();
    }
    let lower = path.to_lowercase();
    if lower.ends_with(".lnk") {
        "shortcut".into()
    } else if lower.ends_with(".url") {
        "url".into()
    } else {
        "file".into()
    }
}

fn icon_hash(path: &str, name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(path.as_bytes());
    hasher.update(b"|");
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(40);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

unsafe fn extract_icon(item: &IShellItem, icon_dir: &Path, path: &str) -> Option<String> {
    let hash = icon_hash(path, path);
    let key = format!("icons/{}.png", hash);
    let file_path = icon_dir.join(format!("{}.png", hash));
    if file_path.exists() {
        return Some(key);
    }

    let factory: IShellItemImageFactory = item.cast().ok()?;
    let bitmap = factory.GetImage(SIZE { cx: 48, cy: 48 }, SIIGBF_ICONONLY).ok()?;
    let (width, height, pixels) = hbitmap_to_rgba(bitmap)?;
    let image = image::RgbaImage::from_raw(width, height, pixels)?;
    if std::fs::create_dir_all(icon_dir).is_err() {
        return None;
    }
    if image.save(&file_path).is_ok() {
        return Some(key);
    }
    None
}

unsafe fn hbitmap_to_rgba(bitmap: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut bmp = BITMAP::default();
    if GetObjectW(
        HGDIOBJ(bitmap.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut BITMAP as *mut c_void),
    ) == 0
    {
        return None;
    }
    if bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
        return None;
    }
    let width = bmp.bmWidth as u32;
    let height = bmp.bmHeight as u32;

    let hdc: HDC = CreateCompatibleDC(None);
    if hdc.0.is_null() {
        return None;
    }
    let old = SelectObject(hdc, HGDIOBJ(bitmap.0));
    let mut info = BITMAPINFO::default();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: bmp.bmWidth,
        biHeight: -bmp.bmHeight,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
    };

    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let lines = GetDIBits(
        hdc,
        bitmap,
        0,
        height,
        Some(buffer.as_mut_ptr() as *mut c_void),
        &mut info,
        DIB_RGB_COLORS,
    );
    SelectObject(hdc, old);
    let _ = DeleteDC(hdc);
    let _ = DeleteObject(HGDIOBJ(bitmap.0));

    if lines == 0 {
        return None;
    }

    let total = (width * height) as usize;
    let mut rgba = Vec::with_capacity(total * 4);
    for i in 0..total {
        let b = buffer[i * 4];
        let g = buffer[i * 4 + 1];
        let r = buffer[i * 4 + 2];
        let a = buffer[i * 4 + 3];
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(a);
    }

    Some((width, height, rgba))
}

