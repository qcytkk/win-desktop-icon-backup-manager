use std::ffi::c_void;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW,
    RegisterClassW, SetWindowLongPtrW, TranslateMessage, GWLP_USERDATA, MSG, SPI_SETWORKAREA,
    WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSW, WNDCLASS_STYLES, WM_DISPLAYCHANGE, WM_DPICHANGED,
    WM_SETTINGCHANGE,
};

use crate::commands;
use crate::desktop::{display, icons};
use crate::storage::{backup, settings};

const LOGIN_DELAY: Duration = Duration::from_secs(8);
const DEBOUNCE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveLayout {
    pub backup_id: String,
    pub scale_mode: String,
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("guard.json"))
}

/// 应用布局成功后记录活动布局，供守护进程在登录/显示变化后重放。
pub fn remember(app: &AppHandle, backup_id: &str, scale_mode: &str) {
    let Some(path) = state_path(app) else { return };
    let state = ActiveLayout {
        backup_id: backup_id.to_string(),
        scale_mode: scale_mode.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = fs::write(path, json);
    }
}

fn read_active(app: &AppHandle) -> Option<ActiveLayout> {
    let path = state_path(app)?;
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn log_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("guard.log"))
}

pub fn log(app: &AppHandle, text: &str) {
    let Some(path) = log_path(app) else { return };
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > 1024 * 1024 {
            let _ = fs::remove_file(&path);
        }
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), text);
    }
}

/// 启动守护线程。`initial_restore` 为 true 时执行一次登录后的恢复。
pub fn start(app: AppHandle, initial_restore: bool) {
    let (tx, rx) = mpsc::channel::<()>();
    spawn_listener(tx, app.clone());
    thread::spawn(move || worker(app, rx, initial_restore));
}

fn worker(app: AppHandle, rx: Receiver<()>, initial_restore: bool) {
    log(&app, "守护线程启动");
    if initial_restore {
        wait_desktop_ready();
        thread::sleep(LOGIN_DELAY);
        restore(&app, "登录");
    }

    loop {
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(_) => {
                // 等待事件安静下来，避免一次显示切换触发多次恢复。
                loop {
                    match rx.recv_timeout(DEBOUNCE) {
                        Ok(_) => continue,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                restore(&app, "显示变化");
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn wait_desktop_ready() {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if icons::read_positions().is_ok() {
            return;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn restore(app: &AppHandle, reason: &str) {
    let cfg = settings::load(app);
    if !cfg.auto_start {
        return;
    }
    let Some(state) = read_active(app) else {
        log(app, &format!("{}：没有活动布局记录，跳过", reason));
        return;
    };
    let root = settings::root(app, &cfg);
    let record = match backup::get(&root, &state.backup_id) {
        Ok(record) => record,
        Err(err) => {
            log(app, &format!("{}：读取活动布局失败：{}", reason, err));
            return;
        }
    };
    if record.icons.is_empty() {
        return;
    }

    for attempt in 1..=3 {
        match commands::apply_record(&record, &state.scale_mode) {
            Ok(result) => {
                thread::sleep(Duration::from_secs(1));
                if verify(&record, &state.scale_mode) {
                    log(
                        app,
                        &format!(
                            "{}：第 {} 次恢复成功，{} 个图标",
                            reason, attempt, result.matched
                        ),
                    );
                    return;
                }
                log(app, &format!("{}：第 {} 次恢复校验未通过", reason, attempt));
            }
            Err(err) => log(app, &format!("{}：第 {} 次恢复失败：{}", reason, attempt, err)),
        }
        thread::sleep(Duration::from_secs(3));
    }
    log(app, &format!("{}：重试 3 次仍未恢复成功", reason));
}

fn verify(record: &crate::models::BackupRecord, scale_mode: &str) -> bool {
    let current = display::read_display();
    let backup_scale = if record.display.scale_factor > 0.0 {
        record.display.scale_factor
    } else {
        1.0
    };
    let ratio = if scale_mode == "backup" {
        1.0
    } else {
        current.scale_factor / backup_scale
    };
    let Ok(positions) = icons::read_positions() else {
        return false;
    };
    let max_x = current.virtual_bounds.width.max(1) as i32;
    let max_y = current.virtual_bounds.height.max(1) as i32;

    let mut expected = 0usize;
    let mut matched = 0usize;
    for icon in &record.icons {
        if icon.x < 0 || icon.y < 0 {
            continue;
        }
        let Some((_, actual)) = positions
            .iter()
            .find(|(path, _)| path.eq_ignore_ascii_case(&icon.path))
        else {
            continue;
        };
        expected += 1;
        let mut x = (icon.x as f64 * ratio).round() as i32;
        let mut y = (icon.y as f64 * ratio).round() as i32;
        if x > max_x - 10 {
            x = max_x - 10;
        }
        if y > max_y - 10 {
            y = max_y - 10;
        }
        if (actual.x - x).abs() <= 3 && (actual.y - y).abs() <= 3 {
            matched += 1;
        }
    }
    expected > 0 && matched * 10 >= expected * 9
}

struct ListenerState {
    tx: Sender<()>,
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DISPLAYCHANGE | WM_DPICHANGED => {
            notify_listener(hwnd);
            LRESULT(0)
        }
        WM_SETTINGCHANGE => {
            if wparam.0 == SPI_SETWORKAREA.0 as usize {
                notify_listener(hwnd);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn notify_listener(hwnd: HWND) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const ListenerState;
    if !ptr.is_null() {
        let _ = (*ptr).tx.send(());
    }
}

fn spawn_listener(tx: Sender<()>, app: AppHandle) {
    thread::spawn(move || unsafe {
        let class_name = windows::core::HSTRING::from("DesktopIconBackupGuardWindow");
        let window_name = windows::core::HSTRING::from("DesktopIconBackupGuard");
        let Ok(hinstance) = GetModuleHandleW(None) else {
            log(&app, "显示变化监听失败：无法获取模块句柄");
            return;
        };

        let wc = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };
        let _ = RegisterClassW(&wc);

        let state = Box::into_raw(Box::new(ListenerState { tx }));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            Some(state as *const c_void),
        );
        let Ok(hwnd) = hwnd else {
            drop(Box::from_raw(state));
            log(&app, "显示变化监听失败：无法创建隐藏窗口");
            return;
        };
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
        log(&app, "显示变化监听已就绪");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
