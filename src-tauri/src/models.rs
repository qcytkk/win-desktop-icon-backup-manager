use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub scale_factor: f64,
    pub virtual_bounds: Rect,
    pub monitors: Vec<MonitorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconItem {
    pub index: i32,
    pub raw_name: String,
    pub display_name: String,
    pub kind: String,
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub icon_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub display: DisplayInfo,
    pub icon_count: usize,
    pub layout_source: String,
    pub warnings: Vec<String>,
    pub icons: Vec<IconItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub font_size: String,
    pub storage_mode: String,
    pub custom_path: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            language: "zh-CN".into(),
            font_size: "normal".into(),
            storage_mode: "documents".into(),
            custom_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathsInfo {
    pub documents: String,
    pub app: String,
    pub current: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: AppSettings,
    pub paths: PathsInfo,
}
