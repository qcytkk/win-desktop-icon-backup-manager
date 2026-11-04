use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use windows::Win32::System::SystemInformation::GetTickCount64;

use crate::storage::settings;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GuideState {
    last_boot_at: u64,
}

fn state_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join("onboarding.json"))
}

/// 本次开机的近似时间戳（秒）。同一启动周期内多次计算误差不超过几秒。
fn boot_epoch() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let uptime = unsafe { GetTickCount64() } / 1000;
    now.saturating_sub(uptime)
}

/// 是否应该显示引导：开启引导开关，且本次开机还没有显示过。
pub fn should_show(app: &AppHandle) -> bool {
    let cfg = settings::load(app);
    if !cfg.guide {
        return false;
    }
    let Some(path) = state_path(app) else {
        return false;
    };
    let current = boot_epoch();
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<GuideState>(&raw) {
            // 允许几秒误差，避免同一启动周期被误判成新的开机。
            if current.abs_diff(state.last_boot_at) <= 30 {
                return false;
            }
        }
    }
    let state = GuideState {
        last_boot_at: current,
    };
    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = fs::write(path, json);
    }
    true
}
