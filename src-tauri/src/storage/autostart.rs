use std::env;
use std::io::ErrorKind;
use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "DesktopIconBackup";

pub fn current_exe() -> Option<String> {
    env::current_exe()
        .ok()
        .map(|p| p.display().to_string())
}

fn command_line(exe: &Path) -> String {
    format!("\"{}\" --autostart", exe.display())
}

fn read_value() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ).ok()?;
    key.get_value::<String, _>(VALUE_NAME).ok()
}

/// 让 Run 键与设置保持一致：开启时写入当前 exe 的 --autostart 命令，关闭时删除。
pub fn sync(enabled: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let exe = env::current_exe().map_err(|e| format!("无法定位程序路径：{}", e))?;
        let desired = command_line(&exe);
        if read_value().as_deref() == Some(desired.as_str()) {
            return Ok(());
        }
        let key = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_WRITE)
            .map_err(|e| format!("无法打开开机启动注册表：{}", e))?;
        key.set_value(VALUE_NAME, &desired)
            .map_err(|e| format!("写入开机启动失败：{}", e))
    } else {
        if read_value().is_none() {
            return Ok(());
        }
        let key = hkcu
            .open_subkey_with_flags(RUN_KEY, KEY_WRITE)
            .map_err(|e| format!("无法打开开机启动注册表：{}", e))?;
        match key.delete_value(VALUE_NAME) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("删除开机启动失败：{}", err)),
        }
    }
}

pub fn is_enabled() -> bool {
    read_value().is_some()
}
