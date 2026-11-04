use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};

fn labels(language: &str) -> (&'static str, &'static str, &'static str) {
    match language {
        "zh-TW" => ("顯示主介面", "結束", "桌面圖示備份"),
        "en" => ("Show main window", "Quit", "Desktop Icon Backup"),
        "fr" => ("Afficher la fenêtre", "Quitter", "Sauvegarde des icônes"),
        "ko" => ("주 창 열기", "종료", "바탕화면 아이콘 백업"),
        "ja" => ("メイン画面を表示", "終了", "デスクトップアイコン配置バックアップ"),
        _ => ("显示主界面", "退出", "桌面图标备份"),
    }
}

pub fn init(app: &AppHandle, language: &str) -> tauri::Result<()> {
    let (show_text, quit_text, tooltip) = labels(language);
    let show = MenuItem::with_id(app, "show", show_text, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_text, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .or_else(|| Image::from_bytes(include_bytes!("../icons/32x32.png")).ok());

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip(tooltip)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle().clone());
            }
        });

    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

pub fn show_main_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    // Tauri 文档提示：在同步事件回调里直接创建窗口可能死锁，放到异步任务中执行。
    tauri::async_runtime::spawn(async move {
        let _ = create_main_window(&app);
    });
}

pub fn create_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("桌面图标备份")
        .inner_size(1080.0, 720.0)
        .min_inner_size(900.0, 560.0)
        .center()
        .build()?;
    let _ = window.set_focus();
    Ok(())
}
