pub mod commands;
pub mod desktop;
pub mod models;
pub mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
