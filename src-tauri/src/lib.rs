pub mod commands;
pub mod desktop;
pub mod message_box;
pub mod models;
pub mod storage;
pub mod tray;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(backup_now: bool, autostart: bool) {
    let mut builder = tauri::Builder::default();
    if !backup_now {
        // 单实例：重复启动时唤起已有实例的主界面。
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main_window(app.clone());
        }));
    }

    builder
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
