use std::env;
use std::path::Path;

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const MENU_KEY: &str = r"Software\Classes\DesktopBackground\Shell\DesktopIconBackup";

pub fn menu_text(language: &str) -> &'static str {
    match language {
        "zh-TW" => "備份目前桌面配置",
        "en" => "Back up current desktop layout",
        "fr" => "Sauvegarder la disposition actuelle du bureau",
        "ko" => "현재 바탕화면 배치 백업",
        "ja" => "現在のデスクトップ配置をバックアップ",
        _ => "备份当前桌面布局",
    }
}

/// 注册或移除桌面空白处右键菜单。
pub fn sync(enabled: bool, language: &str) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if enabled {
        let exe = env::current_exe().map_err(|e| format!("无法定位程序路径：{}", e))?;
        let (key, _) = hkcu
            .create_subkey(MENU_KEY)
            .map_err(|e| format!("无法创建右键菜单注册表：{}", e))?;
        key.set_value("", &menu_text(language))
            .map_err(|e| format!("写入菜单文案失败：{}", e))?;
        key.set_value("Icon", &exe.display().to_string())
            .map_err(|e| format!("写入菜单图标失败：{}", e))?;
        key.set_value("Position", &"Bottom")
            .map_err(|e| format!("写入菜单位置失败：{}", e))?;
        let (command, _) = key
            .create_subkey("command")
            .map_err(|e| format!("无法创建菜单命令：{}", e))?;
        command
            .set_value("", &command_line(&exe))
            .map_err(|e| format!("写入菜单命令失败：{}", e))
    } else {
        match hkcu.delete_subkey_all(MENU_KEY) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("移除右键菜单失败：{}", err)),
        }
    }
}

fn command_line(exe: &Path) -> String {
    format!("\"{}\" --backup-now", exe.display())
}
