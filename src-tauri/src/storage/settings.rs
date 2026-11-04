use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::models::{AppSettings, PathsInfo, SettingsView};

fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法定位配置目录：{}", e))?;
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建配置目录：{}", e))?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> AppSettings {
    let path = match config_file(app) {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };
    serde_json::from_str::<AppSettings>(&raw).unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = config_file(app)?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| format!("序列化设置失败：{}", e))?;
    fs::write(&path, json).map_err(|e| format!("保存设置失败：{}", e))
}

pub fn documents_dir(app: &AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().document_dir() {
        return dir.join("DesktopIconBackup");
    }
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Documents").join("DesktopIconBackup")
}

pub fn app_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("backups")
}

pub fn paths(app: &AppHandle, settings: &AppSettings) -> PathsInfo {
    let documents = documents_dir(app);
    let app_path = app_dir();
    let current = match settings.storage_mode.as_str() {
        "app" => app_path.clone(),
        "custom" => {
            let custom = settings.custom_path.trim();
            if custom.is_empty() {
                documents.clone()
            } else {
                PathBuf::from(custom)
            }
        }
        _ => documents.clone(),
    };

    PathsInfo {
        documents: documents.display().to_string(),
        app: app_path.display().to_string(),
        current: current.display().to_string(),
    }
}

pub fn view(app: &AppHandle, settings: AppSettings) -> SettingsView {
    let paths = paths(app, &settings);
    SettingsView { settings, paths }
}

pub fn root(app: &AppHandle, settings: &AppSettings) -> PathBuf {
    let documents = documents_dir(app);
    match settings.storage_mode.as_str() {
        "app" => app_dir(),
        "custom" => {
            let custom = settings.custom_path.trim();
            if custom.is_empty() {
                documents
            } else {
                PathBuf::from(custom)
            }
        }
        _ => documents,
    }
}
