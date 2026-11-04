pub mod commands;
pub mod desktop;
pub mod message_box;
pub mod models;
pub mod storage;
pub mod tray;

use tauri::{Emitter, Manager, WindowEvent};

fn run_backup_now(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        desktop::guard::log(&app, "收到右键菜单备份请求");
        let result = commands::create_backup_headless(&app).map(|record| record.icon_count);
        // 先通知主界面刷新，再弹提示，避免用户关闭提示前列表仍是旧数据。
        let _ = app.emit("backups-changed", ());
        desktop::guard::log(
            &app,
            &match &result {
                Ok(count) => format!("右键菜单备份完成：{} 个图标", count),
                Err(err) => format!("右键菜单备份失败：{}", err),
            },
        );
        let language = storage::settings::load(&app).language;
        message_box::backup_result(&language, result);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(backup_now: bool, autostart: bool) {
    tauri::Builder::default()
        // 单实例：重复启动时唤起已有实例；右键菜单备份交由已有实例处理。
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|arg| arg == "--backup-now") {
                run_backup_now(app.clone());
            } else {
                tray::show_main_window(app.clone());
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let handle = app.handle().clone();

            if backup_now {
                let result = commands::create_backup_headless(&handle)
                    .map(|record| record.icon_count);
                let language = storage::settings::load(&handle).language;
                message_box::backup_result(&language, result);
                std::process::exit(0);
            }

            let settings = storage::settings::load(&handle);
            if let Err(err) = storage::autostart::sync(settings.auto_start) {
                desktop::guard::log(&handle, &format!("同步开机启动失败：{}", err));
            }
            if let Err(err) =
                storage::context_menu::sync(settings.context_menu, &settings.language)
            {
                desktop::guard::log(&handle, &format!("同步右键菜单失败：{}", err));
            }

            tray::init(&handle, &settings.language)?;
            desktop::guard::start(handle.clone(), autostart && settings.auto_start);

            if !(autostart && settings.auto_start) {
                tray::create_main_window(&handle)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let settings = storage::settings::load(app);
                if settings.auto_start {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::get_settings,
            commands::save_settings,
            commands::create_backup,
            commands::get_current_display,
            commands::apply_backup,
            commands::list_backups,
            commands::get_backup,
            commands::rename_backup,
            commands::delete_backup,
            commands::load_icons,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
