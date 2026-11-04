use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use chrono::{DateTime, Local};

use crate::models::{BackupRecord, IconItem};

pub fn backups_dir(root: &Path) -> PathBuf {
    root.join("backups")
}

pub fn icons_dir(root: &Path) -> PathBuf {
    root.join("icons")
}

pub fn create(
    root: &Path,
    name: &str,
    display: crate::models::DisplayInfo,
    icons: Vec<IconItem>,
    layout_source: &str,
    warnings: Vec<String>,
    layout_bytes: &[u8],
    registry_json: &str,
) -> Result<BackupRecord, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = backups_dir(root).join(&id);
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建备份目录：{}", e))?;

    let created_at = Local::now().to_rfc3339();
    let record = BackupRecord {
        id: id.clone(),
        name: name.to_string(),
        created_at,
        display,
        icon_count: icons.len(),
        layout_source: layout_source.to_string(),
        warnings,
        icons,
    };

    let meta = serde_json::to_string_pretty(&record).map_err(|e| format!("序列化备份失败：{}", e))?;
    fs::write(dir.join("meta.json"), meta).map_err(|e| format!("写入备份失败：{}", e))?;
    fs::write(dir.join("layout.bin"), layout_bytes).map_err(|e| format!("写入布局数据失败：{}", e))?;
    fs::write(dir.join("registry.json"), registry_json)
        .map_err(|e| format!("写入注册表快照失败：{}", e))?;

    Ok(record)
}

pub fn list(root: &Path) -> Result<Vec<BackupRecord>, String> {
    let dir = backups_dir(root);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("读取备份目录失败：{}", e))?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let meta_path = entry.path().join("meta.json");
        let raw = match fs::read_to_string(&meta_path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Ok(record) = serde_json::from_str::<BackupRecord>(&raw) {
            records.push(record);
        }
    }

    records.sort_by(|a, b| {
        let ta = DateTime::parse_from_rfc3339(&a.created_at)
            .map(|t| t.timestamp_millis())
            .unwrap_or(0);
        let tb = DateTime::parse_from_rfc3339(&b.created_at)
            .map(|t| t.timestamp_millis())
            .unwrap_or(0);
        tb.cmp(&ta)
    });

    Ok(records)
}

pub fn dir_of(root: &Path, id: &str) -> PathBuf {
    backups_dir(root).join(id)
}

pub fn get(root: &Path, id: &str) -> Result<BackupRecord, String> {
    let meta_path = dir_of(root, id).join("meta.json");
    let raw = fs::read_to_string(&meta_path).map_err(|_| "找不到该备份".to_string())?;
    serde_json::from_str::<BackupRecord>(&raw).map_err(|e| format!("备份数据损坏：{}", e))
}

pub fn rename(root: &Path, id: &str, name: &str) -> Result<BackupRecord, String> {
    let mut record = get(root, id)?;
    let trimmed: String = name.trim().chars().take(40).collect();
    record.name = if trimmed.is_empty() {
        default_name_from(&record.created_at)
    } else {
        trimmed
    };

    let meta = serde_json::to_string_pretty(&record).map_err(|e| format!("序列化备份失败：{}", e))?;
    fs::write(dir_of(root, id).join("meta.json"), meta)
        .map_err(|e| format!("保存名称失败：{}", e))?;
    Ok(record)
}

pub fn delete(root: &Path, id: &str) -> Result<(), String> {
    let dir = dir_of(root, id);
    if !dir.exists() {
        return Err("找不到该备份".into());
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("删除备份失败：{}", e))
}

pub fn load_icons(root: &Path, id: &str) -> Result<HashMap<String, String>, String> {
    let record = get(root, id)?;
    let mut out = HashMap::new();
    for icon in record.icons {
        let Some(key) = icon.icon_key else { continue };
        if out.contains_key(&key) {
            continue;
        }
        let file = root.join(&key);
        let bytes = match fs::read(&file) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        out.insert(key, format!("data:image/png;base64,{}", encoded));
    }
    Ok(out)
}

pub fn default_name_from(created_at: &str) -> String {
    let time = DateTime::parse_from_rfc3339(created_at)
        .map(|t| t.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| Local::now().format("%Y-%m-%d %H:%M").to_string());
    format!("备份 {}", time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DisplayInfo, IconItem, MonitorInfo, Rect};

    fn sample_display() -> DisplayInfo {
        DisplayInfo {
            width: 1920,
            height: 1080,
            dpi: 96,
            scale_factor: 1.0,
            virtual_bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            monitors: vec![MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                primary: true,
            }],
        }
    }

    fn sample_icon() -> IconItem {
        IconItem {
            index: 0,
            raw_name: "Krita.lnk>  ".into(),
            display_name: "Krita".into(),
            kind: "shortcut".into(),
            path: r"C:\Users\liang\Desktop\Krita.lnk".into(),
            x: 18,
            y: 20,
            icon_key: None,
        }
    }

    #[test]
    fn create_list_rename_delete() {
        let root = std::env::temp_dir().join(format!("dcib-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(icons_dir(&root)).unwrap();

        let record = create(
            &root,
            "测试布局",
            sample_display(),
            vec![sample_icon()],
            "folder_view",
            vec![],
            &[1u8, 2, 3],
            "{}",
        )
        .unwrap();
        assert_eq!(record.icon_count, 1);
        assert!(dir_of(&root, &record.id).join("layout.bin").exists());

        let items = list(&root).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "测试布局");

        let renamed = rename(&root, &record.id, "  新名字  ").unwrap();
        assert_eq!(renamed.name, "新名字");

        let fallback = rename(&root, &record.id, "   ").unwrap();
        assert!(fallback.name.starts_with("备份 "));

        delete(&root, &record.id).unwrap();
        assert!(list(&root).unwrap().is_empty());

        let _ = fs::remove_dir_all(&root);
    }
}
