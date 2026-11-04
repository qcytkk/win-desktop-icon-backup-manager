use std::fs;

use tauri::AppHandle;

use crate::desktop::{apply, display, guard, icons, layout, registry};
use crate::models::{AppSettings, BackupRecord, DisplayInfo, IconItem, SettingsView};
use crate::storage::{backup, settings};

#[tauri::command]
pub fn ping() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    let settings = settings::load(&app);
    settings::view(&app, settings)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<SettingsView, String> {
    settings::save(&app, &settings)?;
    // 开关类设置需要同步到系统（开机启动、桌面右键菜单）。
    let _ = crate::storage::autostart::sync(settings.auto_start);
    let _ = crate::storage::context_menu::sync(settings.context_menu, &settings.language);
    Ok(settings::view(&app, settings))
}

fn build_backup(app: &AppHandle, name: Option<String>) -> Result<BackupRecord, String> {
    let current = settings::load(app);
    let root = settings::root(app, &current);
    let icon_dir = backup::icons_dir(&root);

    fs::create_dir_all(backup::backups_dir(&root))
        .map_err(|e| format!("无法创建备份目录：{}", e))?;
    fs::create_dir_all(&icon_dir).map_err(|e| format!("无法创建图标缓存目录：{}", e))?;

    let display_info = display::read_display();
    let snapshot = registry::snapshot_bags()?;
    let layout_bytes = registry::icon_layouts_bytes(&snapshot).unwrap_or_default();
    let registry_json =
        serde_json::to_string(&snapshot).map_err(|e| format!("序列化注册表快照失败：{}", e))?;

    let (items, source, mut warnings) = match icons::collect(&icon_dir) {
        Ok(collected) => (collected.icons, collected.source, collected.warnings),
        Err(err) => {
            let names = registry::icon_layouts_bytes(&snapshot)
                .ok()
                .and_then(|bytes| layout::parse_names(&bytes).ok());
            let Some(parsed) = names else {
                return Err(format!("无法读取桌面图标：{}", err));
            };
            let items = build_items_from_names(&parsed.names);
            (
                items,
                "name_only".to_string(),
                vec![format!(
                    "未取到图标坐标，预览按名称网格排列（{}）",
                    err
                )],
            )
        }
    };

    if items.is_empty() {
        warnings.push("当前桌面没有图标".into());
    }

    let now = chrono::Local::now();
    let final_name = match name {
        Some(value) if !value.trim().is_empty() => value.trim().chars().take(40).collect(),
        _ => backup::default_name_from(&now.to_rfc3339()),
    };

    backup::create(
        &root,
        &final_name,
        display_info,
        items,
        &source,
        warnings,
        &layout_bytes,
        &registry_json,
    )
}

#[tauri::command]
pub async fn create_backup(app: AppHandle, name: Option<String>) -> Result<BackupRecord, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || build_backup(&handle, name))
        .await
        .map_err(|e| format!("备份任务异常：{}", e))?
}

/// 右键菜单触发的同步备份，复用与界面一致的备份流程。
pub fn create_backup_headless(app: &AppHandle) -> Result<BackupRecord, String> {
    build_backup(app, None)
}

fn build_items_from_names(names: &[String]) -> Vec<IconItem> {
    names
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let cleaned = layout::clean_name(raw);
            let display_name = layout::clsid_display_name(&cleaned)
                .map(|s| s.to_string())
                .unwrap_or_else(|| cleaned.clone());
            IconItem {
                index: index as i32,
                raw_name: raw.clone(),
                display_name,
                kind: layout::kind_from_name(&cleaned).to_string(),
                path: cleaned,
                x: -1,
                y: -1,
                icon_key: None,
            }
        })
        .collect()
}

#[tauri::command]
pub fn get_current_display() -> DisplayInfo {
    display::read_display()
}

#[tauri::command]
pub async fn apply_backup(
    app: AppHandle,
    id: String,
    scale_mode: String,
) -> Result<apply::ApplyResult, String> {
    let current_settings = settings::load(&app);
    let root = settings::root(&app, &current_settings);
    let record = backup::get(&root, &id)?;

    let mode = scale_mode.clone();
    let result = tauri::async_runtime::spawn_blocking(move || apply_record(&record, &mode))
        .await
        .map_err(|e| format!("应用布局任务异常：{}", e))??;

    // 记录活动布局，供守护进程在登录/显示变化后恢复。
    guard::remember(&app, &id, &scale_mode);
    Ok(result)
}

/// 按备份记录和缩放策略应用布局；界面命令与守护进程共用。
pub fn apply_record(record: &BackupRecord, scale_mode: &str) -> Result<apply::ApplyResult, String> {
    let current_display = display::read_display();
    let backup_scale = if record.display.scale_factor > 0.0 {
        record.display.scale_factor
    } else {
        1.0
    };
    let ratio = if scale_mode == "backup" {
        1.0
    } else {
        current_display.scale_factor / backup_scale
    };
    apply::apply(record, &current_display, ratio)
}

#[tauri::command]
pub fn list_backups(app: AppHandle) -> Result<Vec<BackupRecord>, String> {
    let current = settings::load(&app);
    let root = settings::root(&app, &current);
    backup::list(&root)
}

#[tauri::command]
pub fn get_backup(app: AppHandle, id: String) -> Result<BackupRecord, String> {
    let current = settings::load(&app);
    let root = settings::root(&app, &current);
    backup::get(&root, &id)
}

#[tauri::command]
pub fn rename_backup(app: AppHandle, id: String, name: String) -> Result<BackupRecord, String> {
    let current = settings::load(&app);
    let root = settings::root(&app, &current);
    backup::rename(&root, &id, &name)
}

#[tauri::command]
pub fn delete_backup(app: AppHandle, id: String) -> Result<(), String> {
    let current = settings::load(&app);
    let root = settings::root(&app, &current);
    backup::delete(&root, &id)
}

#[tauri::command]
pub fn load_icons(app: AppHandle, id: String) -> Result<std::collections::HashMap<String, String>, String> {
    let current = settings::load(&app);
    let root = settings::root(&app, &current);
    backup::load_icons(&root, &id)
}
