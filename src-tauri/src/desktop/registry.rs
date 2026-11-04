use std::collections::BTreeMap;

use serde::Serialize;
use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_SET_VALUE};
use winreg::{RegKey, RegValue};

const BAGS_PATH: &str = r"Software\Microsoft\Windows\Shell\Bags\1\Desktop";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawValue {
    pub vtype: u32,
    pub bytes_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySnapshot {
    pub values: BTreeMap<String, RawValue>,
    pub icon_layouts: Option<String>,
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

pub fn snapshot_bags() -> Result<RegistrySnapshot, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(BAGS_PATH)
        .map_err(|e| format!("无法读取桌面布局注册表：{}", e))?;

    let mut values = BTreeMap::new();
    for entry in key.enum_values() {
        let (name, value) = entry.map_err(|e| format!("读取注册表值失败：{}", e))?;
        values.insert(
            name,
            RawValue {
                vtype: value.vtype as u32,
                bytes_hex: to_hex(&value.bytes),
            },
        );
    }

    let icon_layouts = values
        .get("IconLayouts")
        .map(|v| v.bytes_hex.clone());

    Ok(RegistrySnapshot {
        values,
        icon_layouts,
    })
}

pub fn icon_layouts_bytes(snapshot: &RegistrySnapshot) -> Result<Vec<u8>, String> {
    let hex = snapshot
        .icon_layouts
        .as_ref()
        .ok_or_else(|| "注册表中没有 IconLayouts".to_string())?;
    hex_to_bytes(hex)
}

/// 把完整的 IconLayouts 二进制写回桌面布局注册表。
pub fn write_icon_layouts(bytes: &[u8]) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey_with_flags(BAGS_PATH, KEY_SET_VALUE)
        .map_err(|e| format!("无法打开桌面布局注册表：{}", e))?;
    let value = RegValue {
        bytes: bytes.to_vec(),
        vtype: RegType::REG_BINARY,
    };
    key.set_raw_value("IconLayouts", &value)
        .map_err(|e| format!("写入 IconLayouts 失败：{}", e))
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("二进制数据长度异常".into());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = (bytes[i] as char)
            .to_digit(16)
            .ok_or_else(|| "二进制数据格式异常".to_string())?;
        let lo = (bytes[i + 1] as char)
            .to_digit(16)
            .ok_or_else(|| "二进制数据格式异常".to_string())?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}
