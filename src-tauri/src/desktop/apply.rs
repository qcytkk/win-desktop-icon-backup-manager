use std::collections::HashSet;
use std::ffi::c_void;
use std::thread;

use serde::Serialize;
use windows::core::Interface;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{FWF_AUTOARRANGE, IFolderView2, SVGIO_ALLVIEW, SVSI_POSITIONITEM};

use crate::desktop::icons::{open_desktop_view, pidl_display_name, pidl_path};
use crate::models::{BackupRecord, DisplayInfo};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub matched: usize,
    pub missing: Vec<String>,
    pub new_placed: usize,
    pub auto_arrange_disabled: bool,
    pub warnings: Vec<String>,
}

struct CurrentItem {
    pidl: *mut ITEMIDLIST,
    name: String,
    path: String,
}

pub fn apply(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let record = record.clone();
    let current = current.clone();
    thread::spawn(move || unsafe { apply_sta(&record, &current, ratio) })
        .join()
        .map_err(|_| "应用布局线程异常".to_string())?
}

unsafe fn apply_sta(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let initialized = init.is_ok();

    let result = apply_inner(record, current, ratio);
    if initialized {
        CoUninitialize();
    }
    result
}

unsafe fn apply_inner(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let view = open_desktop_view()?;
    let folder_view = &view.folder_view;
    let desktop = &view.desktop;

    let count = folder_view
        .ItemCount(SVGIO_ALLVIEW)
        .map_err(|e| format!("无法统计桌面图标：{}", e))?;

    let mut items: Vec<CurrentItem> = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        let pidl = match folder_view.Item(i) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let name = match pidl_display_name(desktop, pidl as *const ITEMIDLIST) {
            Some(name) => name,
            None => {
                CoTaskMemFree(Some(pidl as *const c_void));
                continue;
            }
        };
        if name.eq_ignore_ascii_case("desktop.ini") {
            CoTaskMemFree(Some(pidl as *const c_void));
            continue;
        }
        let path = pidl_path(desktop, pidl as *const ITEMIDLIST).unwrap_or_else(|| name.clone());
        items.push(CurrentItem { pidl, name, path });
    }

    let mut used = vec![false; items.len()];
    let mut moves: Vec<(usize, POINT)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut matched = 0usize;
    let mut unpositioned = 0usize;

    for icon in &record.icons {
        if icon.x < 0 || icon.y < 0 {
            unpositioned += 1;
            continue;
        }
        let mut found: Option<usize> = None;
        for (i, item) in items.iter().enumerate() {
            if used[i] {
                continue;
            }
            if item.path.eq_ignore_ascii_case(&icon.path) {
                found = Some(i);
                break;
            }
        }
        if found.is_none() {
            let target_name = icon.display_name.to_lowercase();
            for (i, item) in items.iter().enumerate() {
                if used[i] {
                    continue;
                }
                if item.name.to_lowercase() == target_name {
                    found = Some(i);
                    break;
                }
            }
        }

        match found {
            Some(i) => {
                used[i] = true;
                matched += 1;
                moves.push((
                    i,
                    POINT {
                        x: (icon.x as f64 * ratio).round() as i32,
                        y: (icon.y as f64 * ratio).round() as i32,
                    },
                ));
            }
            None => missing.push(icon.display_name.clone()),
        }
    }

    let mut warnings: Vec<String> = Vec::new();
    if unpositioned > 0 {
        warnings.push(format!("{} 个备份图标没有坐标，已跳过", unpositioned));
    }
    let max_x = current.virtual_bounds.width.max(1) as i32;
    let max_y = current.virtual_bounds.height.max(1) as i32;
    let mut clamped = 0usize;
    for (_, point) in moves.iter_mut() {
        if point.x > max_x - 10 {
            point.x = max_x - 10;
            clamped += 1;
        }
        if point.y > max_y - 10 {
            point.y = max_y - 10;
            clamped += 1;
        }
    }
    if clamped > 0 {
        warnings.push(format!(
            "{} 个图标超出当前屏幕范围，已贴近边缘放置",
            clamped
        ));
    }

    let new_indices: Vec<usize> = (0..items.len()).filter(|i| !used[*i]).collect();
    let mut new_placed = 0usize;
    if !new_indices.is_empty() {
        let mut spacing = POINT::default();
        let _ = folder_view.GetSpacing(&mut spacing);
        if spacing.x <= 0 || spacing.y <= 0 {
            spacing = folder_view
                .GetDefaultSpacing()
                .unwrap_or(POINT { x: 75, y: 91 });
        }
        if spacing.x <= 0 || spacing.y <= 0 {
            spacing = POINT { x: 75, y: 91 };
        }

        let base_x = moves.iter().map(|(_, p)| p.x).min().unwrap_or(18);
        let base_y = moves.iter().map(|(_, p)| p.y).min().unwrap_or(2);
        let max_col = ((max_x - base_x) / spacing.x).max(1);
        let max_row = ((max_y - base_y) / spacing.y).max(1);
        let capacity = max_col * max_row;

        let mut occupied: HashSet<(i32, i32)> = HashSet::new();
        for (_, point) in &moves {
            let col = ((point.x - base_x) as f64 / spacing.x as f64).round() as i32;
            let row = ((point.y - base_y) as f64 / spacing.y as f64).round() as i32;
            occupied.insert((col, row));
        }

        let mut slot = 0i32;
        for index in new_indices {
            let mut placed = false;
            while slot < capacity {
                let col = slot / max_row;
                let row = slot % max_row;
                slot += 1;
                if occupied.contains(&(col, row)) {
                    continue;
                }
                occupied.insert((col, row));
                moves.push((
                    index,
                    POINT {
                        x: base_x + col * spacing.x,
                        y: base_y + row * spacing.y,
                    },
                ));
                new_placed += 1;
                placed = true;
                break;
            }
            if !placed {
                warnings.push("桌面空位不足，部分新增图标未移动".into());
                break;
            }
        }
    }

    let folder_view2: Option<IFolderView2> = folder_view.cast().ok();
    let mut auto_arrange_disabled = false;
    if let Some(view2) = &folder_view2 {
        let flags = view2.GetCurrentFolderFlags().unwrap_or(0);
        if flags & (FWF_AUTOARRANGE.0 as u32) != 0 {
            match view2.SetCurrentFolderFlags(FWF_AUTOARRANGE.0 as u32, 0) {
                Ok(_) => {
                    auto_arrange_disabled = true;
                    warnings.push("已关闭“自动排列图标”，否则布局无法保持".into());
                }
                Err(_) => warnings.push("“自动排列图标”处于开启状态，布局可能无法保持".into()),
            }
        }
    }

    let mut move_error: Option<String> = None;
    if !moves.is_empty() {
        let apidl: Vec<*const ITEMIDLIST> = moves
            .iter()
            .map(|(index, _)| items[*index].pidl as *const ITEMIDLIST)
            .collect();
        let points: Vec<POINT> = moves.iter().map(|(_, point)| *point).collect();
        if let Err(err) = folder_view.SelectAndPositionItems(
            apidl.len() as u32,
            apidl.as_ptr(),
            Some(points.as_ptr()),
            SVSI_POSITIONITEM.0 as u32,
        ) {
            move_error = Some(format!("写入图标位置失败：{}", err));
        }
    }

    for item in &items {
        CoTaskMemFree(Some(item.pidl as *const c_void));
    }

    if let Some(err) = move_error {
        return Err(err);
    }

    // SelectAndPositionItems 只更新当前桌面视图的内存状态，必须让 Shell 保存视图状态，
    // 否则新的坐标不会写入 Bags\1\Desktop\IconLayouts，重启后会恢复成旧布局。
    if let Err(err) = view.shell_view.SaveViewState() {
        warnings.push(format!(
            "布局已应用，但无法写入系统持久化状态，重启后可能恢复原布局：{}",
            err
        ));
    }

    Ok(ApplyResult {
        matched,
        missing,
        new_placed,
        auto_arrange_disabled,
        warnings,
    })
}
