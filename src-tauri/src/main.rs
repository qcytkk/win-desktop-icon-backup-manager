#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let backup_now = args.iter().any(|arg| arg == "--backup-now");
    let autostart = args.iter().any(|arg| arg == "--autostart");
    desktop_icon_backup_lib::run(backup_now, autostart);
}
