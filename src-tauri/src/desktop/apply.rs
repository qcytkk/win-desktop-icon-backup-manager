use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::path::Path;
use std::thread;

use serde::Serialize;
use windows::core::Interface;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{FWF_AUTOARRANGE, IFolderView2, SVGIO_ALLVIEW, SVSI_POSITIONITEM};

use crate::desktop::icons::{open_desktop_view, pidl_display_name, pidl_path};
use crate::desktop::{layout, registry};
use crate::models::{BackupRecord, DisplayInfo};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub matched: usize,
    pub missing: Vec<String>,
    pub new_placed: usize,
    pub auto_arrange_disabled: bool,
    /// 是否已把新坐标直接写回注册表 IconLayouts
    pub registry_synced: bool,
    pub warnings: Vec<String>,
}

struct CurrentItem {
    pidl: *mut ITEMIDLIST,
    name: String,
    path: String,
    /// 用于和注册表名称表匹配的键（文件名或 CLSID）
    base: String,
    old_pos: Option<POINT>,
}

struct RegistryLayout {
    bytes: Vec<u8>,
    names: Vec<String>,
    table_offset: usize,
    /// 按名称索引给出坐标记录在 bytes 中的偏移
    records: Vec<Option<usize>>,
}

fn base_key(path: &str) -> String {
    if path.starts_with("::") {
        return path.trim().to_lowercase();
    }
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_lowercase())
        .unwrap_or_else(|| path.to_lowercase())
}

fn consensus_origin(values: &[f64], tolerance: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let median = sorted[sorted.len() / 2];
    let agree = values.iter().filter(|v| (*v - median).abs() <= tolerance).count();
    if agree * 5 < values.len() * 4 {
        return None;
    }
    Some(median)
}

fn load_registry_layout() -> Option<RegistryLayout> {
    let snapshot = registry::snapshot_bags().ok()?;
    let bytes = registry::icon_layouts_bytes(&snapshot).ok()?;
    let parsed = layout::parse_names(&bytes).ok()?;
    let table_offset = layout::position_table_offset(&bytes, parsed.count)?;
    let records = layout::record_offset_by_name_index(&bytes, table_offset, parsed.count)?;
    Some(RegistryLayout {
        bytes,
        names: parsed.names,
        table_offset,
        records,
    })
}

/// 用旧坐标和注册表网格坐标拟合「原点 + 间距」。要求样本数量和一致性达标，
/// 否则说明注册表布局与当前视图不同步，放弃直写。
fn fit_axis(points: &[(f64, f64)]) -> Option<(f64, f64)> {
    if points.len() < 3 {
        return None;
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    let mut slopes = Vec::new();
    for pair in sorted.windows(2) {
        let delta = pair[1].0 - pair[0].0;
        if delta.abs() > 0.01 {
            slopes.push((pair[1].1 - pair[0].1) / delta);
        }
    }
    if slopes.len() < 2 {
        return None;
    }
    let mut sorted_slopes = slopes.clone();
    sorted_slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let spacing = sorted_slopes[sorted_slopes.len() / 2];
    if !(5.0..=1000.0).contains(&spacing) {
        return None;
    }
    let agree = slopes.iter().filter(|v| (*v - spacing).abs() <= 0.6).count();
    if agree * 5 < slopes.len() * 4 {
        return None;
    }

    let spacing = spacing.round();
    let origins: Vec<f64> = sorted
        .iter()
        .map(|(cell, pixel)| pixel - spacing * cell)
        .collect();
    Some((consensus_origin(&origins, 3.0)?, spacing))
}

fn derive_grid(
    items: &[CurrentItem],
    name_indexes: &[Option<usize>],
    reg: &RegistryLayout,
) -> Option<(f64, f64, f64, f64)> {
    let mut xs: Vec<(f64, f64)> = Vec::new();
    let mut ys: Vec<(f64, f64)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let Some(name_index) = name_indexes[i] else { continue };
        let Some(record) = reg.records.get(name_index).copied().flatten() else { continue };
        let Some(old) = item.old_pos else { continue };
        let rec_index = (record - reg.table_offset) / 10;
        let Some(pos) = layout::read_grid_pos(&reg.bytes, reg.table_offset, rec_index) else { continue };
        xs.push((pos.col as f64, old.x as f64));
        ys.push((pos.row as f64, old.y as f64));
    }
    if xs.len() < 6 || ys.len() < 6 {
        return None;
    }
    let (origin_x, spacing_x) = fit_axis(&xs)?;
    let (origin_y, spacing_y) = fit_axis(&ys)?;
    Some((origin_x, origin_y, spacing_x, spacing_y))
}


pub fn apply(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let record = record.clone();
    let current = current.clone();
    thread::spawn(move || unsafe { apply_sta(&record, &current, ratio) })
        .join()
        .map_err(|_| "应用布局线程异常".to_string())?
}

unsafe fn apply_sta(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let initialized = init.is_ok();

    let result = apply_inner(record, current, ratio);
    if initialized {
        CoUninitialize();
    }
    result
}

unsafe fn apply_inner(
    record: &BackupRecord,
    current: &DisplayInfo,
    ratio: f64,
) -> Result<ApplyResult, String> {
    let view = open_desktop_view()?;
    let folder_view = &view.folder_view;
    let desktop = &view.desktop;

    let count = folder_view
        .ItemCount(SVGIO_ALLVIEW)
        .map_err(|e| format!("无法统计桌面图标：{}", e))?;

    let mut items: Vec<CurrentItem> = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        let pidl = match folder_view.Item(i) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let name = match pidl_display_name(desktop, pidl as *const ITEMIDLIST) {
            Some(name) => name,
            None => {
                CoTaskMemFree(Some(pidl as *const c_void));
                continue;
            }
        };
        if name.eq_ignore_ascii_case("desktop.ini") {
            CoTaskMemFree(Some(pidl as *const c_void));
            continue;
        }
        let path = pidl_path(desktop, pidl as *const ITEMIDLIST).unwrap_or_else(|| name.clone());
        let old_pos = folder_view.GetItemPosition(pidl).ok();
        let base = base_key(&path);
        items.push(CurrentItem {
            pidl,
            name,
            path,
            base,
            old_pos,
        });
    }

    // 读取当前注册表布局，准备在移动完成后把坐标直接写回 IconLayouts。
    // Windows 只保证“当前视图”被改写，重启/显示模式变化时可能重新排布，
    // 直接写盘可以让注册表里至少留有一份与本次应用一致的布局。
    let reg_layout = load_registry_layout();
    let mut name_indexes: Vec<Option<usize>> = vec![None; items.len()];
    let mut name_candidates: HashMap<String, Vec<usize>> = HashMap::new();
    if let Some(reg) = &reg_layout {
        for (index, raw_name) in reg.names.iter().enumerate() {
            name_candidates
                .entry(layout::name_base(raw_name).to_lowercase())
                .or_default()
                .push(index);
        }
    }
    for (i, item) in items.iter().enumerate() {
        if let Some(list) = name_candidates.get(&item.base) {
            if list.len() == 1 {
                name_indexes[i] = Some(list[0]);
            }
        }
    }

    let mut spacing = POINT::default();
    let _ = folder_view.GetSpacing(&mut spacing);
    if spacing.x <= 0 || spacing.y <= 0 {
        spacing = folder_view
            .GetDefaultSpacing()
            .unwrap_or(POINT { x: 75, y: 91 });
    }
    if spacing.x <= 0 || spacing.y <= 0 {
        spacing = POINT { x: 75, y: 91 };
    }

    let grid = match &reg_layout {
        Some(reg) => derive_grid(&items, &name_indexes, reg),
        None => None,
    };
    if let (Some((origin_x, origin_y, spacing_x, spacing_y)), Some(reg)) = (grid, &reg_layout) {
        // 同名图标（如公共桌面和用户桌面下的同名快捷方式）用旧网格位置消歧。
        for (i, item) in items.iter().enumerate() {
            if name_indexes[i].is_some() {
                continue;
            }
            let (Some(list), Some(old)) = (name_candidates.get(&item.base), item.old_pos) else {
                continue;
            };
            if list.len() <= 1 {
                continue;
            }
            let old_col = (old.x as f64 - origin_x) / spacing_x;
            let old_row = (old.y as f64 - origin_y) / spacing_y;
            let best = list
                .iter()
                .filter_map(|&name_index| {
                    let record = reg.records.get(name_index).copied().flatten()?;
                    let rec_index = (record - reg.table_offset) / 10;
                    let pos = layout::read_grid_pos(&reg.bytes, reg.table_offset, rec_index)?;
                    let distance = (pos.col as f64 - old_col).powi(2)
                        + (pos.row as f64 - old_row).powi(2);
                    Some((distance, name_index))
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            if let Some((distance, name_index)) = best {
                if distance.sqrt() < 2.0 {
                    name_indexes[i] = Some(name_index);
                }
            }
        }
    }

    let mut used = vec![false; items.len()];
    let mut moves: Vec<(usize, POINT)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    let mut matched = 0usize;
    let mut unpositioned = 0usize;

    for icon in &record.icons {
        if icon.x < 0 || icon.y < 0 {
            unpositioned += 1;
            continue;
        }
        let mut found: Option<usize> = None;
        for (i, item) in items.iter().enumerate() {
            if used[i] {
                continue;
            }
            if item.path.eq_ignore_ascii_case(&icon.path) {
                found = Some(i);
                break;
            }
        }
        if found.is_none() {
            let target_name = icon.display_name.to_lowercase();
            for (i, item) in items.iter().enumerate() {
                if used[i] {
                    continue;
                }
                if item.name.to_lowercase() == target_name {
                    found = Some(i);
                    break;
                }
            }
        }

        match found {
            Some(i) => {
                used[i] = true;
                matched += 1;
                moves.push((
                    i,
                    POINT {
                        x: (icon.x as f64 * ratio).round() as i32,
                        y: (icon.y as f64 * ratio).round() as i32,
                    },
                ));
            }
            None => missing.push(icon.display_name.clone()),
        }
    }

    let mut warnings: Vec<String> = Vec::new();
    if unpositioned > 0 {
        warnings.push(format!("{} 个备份图标没有坐标，已跳过", unpositioned));
    }
    let max_x = current.virtual_bounds.width.max(1) as i32;
    let max_y = current.virtual_bounds.height.max(1) as i32;
    let mut clamped = 0usize;
    for (_, point) in moves.iter_mut() {
        if point.x > max_x - 10 {
            point.x = max_x - 10;
            clamped += 1;
        }
        if point.y > max_y - 10 {
            point.y = max_y - 10;
            clamped += 1;
        }
    }
    if clamped > 0 {
        warnings.push(format!(
            "{} 个图标超出当前屏幕范围，已贴近边缘放置",
            clamped
        ));
    }

    let new_indices: Vec<usize> = (0..items.len()).filter(|i| !used[*i]).collect();
    let mut new_placed = 0usize;
    if !new_indices.is_empty() {
        let base_x = moves.iter().map(|(_, p)| p.x).min().unwrap_or(18);
        let base_y = moves.iter().map(|(_, p)| p.y).min().unwrap_or(2);
        let max_col = ((max_x - base_x) / spacing.x).max(1);
        let max_row = ((max_y - base_y) / spacing.y).max(1);
        let capacity = max_col * max_row;

        let mut occupied: HashSet<(i32, i32)> = HashSet::new();
        for (_, point) in &moves {
            let col = ((point.x - base_x) as f64 / spacing.x as f64).round() as i32;
            let row = ((point.y - base_y) as f64 / spacing.y as f64).round() as i32;
            occupied.insert((col, row));
        }

        let mut slot = 0i32;
        for index in new_indices {
            let mut placed = false;
            while slot < capacity {
                let col = slot / max_row;
                let row = slot % max_row;
                slot += 1;
                if occupied.contains(&(col, row)) {
                    continue;
                }
                occupied.insert((col, row));
                moves.push((
                    index,
                    POINT {
                        x: base_x + col * spacing.x,
                        y: base_y + row * spacing.y,
                    },
                ));
                new_placed += 1;
                placed = true;
                break;
            }
            if !placed {
                warnings.push("桌面空位不足，部分新增图标未移动".into());
                break;
            }
        }
    }

    let folder_view2: Option<IFolderView2> = folder_view.cast().ok();
    let mut auto_arrange_disabled = false;
    if let Some(view2) = &folder_view2 {
        let flags = view2.GetCurrentFolderFlags().unwrap_or(0);
        if flags & (FWF_AUTOARRANGE.0 as u32) != 0 {
            match view2.SetCurrentFolderFlags(FWF_AUTOARRANGE.0 as u32, 0) {
                Ok(_) => {
                    auto_arrange_disabled = true;
                    warnings.push("已关闭“自动排列图标”，否则布局无法保持".into());
                }
                Err(_) => warnings.push("“自动排列图标”处于开启状态，布局可能无法保持".into()),
            }
        }
    }

    let mut move_error: Option<String> = None;
    if !moves.is_empty() {
        let apidl: Vec<*const ITEMIDLIST> = moves
            .iter()
            .map(|(index, _)| items[*index].pidl as *const ITEMIDLIST)
            .collect();
        let points: Vec<POINT> = moves.iter().map(|(_, point)| *point).collect();
        if let Err(err) = folder_view.SelectAndPositionItems(
            apidl.len() as u32,
            apidl.as_ptr(),
            Some(points.as_ptr()),
            SVSI_POSITIONITEM.0 as u32,
        ) {
            move_error = Some(format!("写入图标位置失败：{}", err));
        }
    }

    if let Some(err) = move_error {
        for item in &items {
            CoTaskMemFree(Some(item.pidl as *const c_void));
        }
        return Err(err);
    }

    // SelectAndPositionItems 只更新当前桌面视图的内存状态，必须让 Shell 保存视图状态，
    // 否则新的坐标不会写入 Bags\1\Desktop\IconLayouts，重启后会恢复成旧布局。
    if let Err(err) = view.shell_view.SaveViewState() {
        warnings.push(format!(
            "布局已应用，但无法写入系统持久化状态，重启后可能恢复原布局：{}",
            err
        ));
    }

    // 兜底：按名称索引把本次坐标直接写回注册表 IconLayouts。
    // Explorer 在显示模式变化或启动时可能重新排布并回写，这里至少保证盘上留有一份新布局。
    let mut registry_synced = false;
    if let (Some((origin_x, origin_y, spacing_x, spacing_y)), Some(reg)) = (grid, reg_layout.as_ref()) {
        let targets: HashMap<usize, POINT> = moves.iter().copied().collect();
        let mut bytes = reg.bytes.clone();
        let mut patched = 0usize;
        for (i, _) in items.iter().enumerate() {
            let Some(name_index) = name_indexes[i] else { continue };
            let Some(record) = reg.records.get(name_index).copied().flatten() else { continue };
            let Some(target) = targets.get(&i).copied() else { continue };
            let col = ((target.x as f64 - origin_x) / spacing_x).round() as f32;
            let row = ((target.y as f64 - origin_y) / spacing_y).round() as f32;
            let rec_index = (record - reg.table_offset) / 10;
            if layout::write_grid_pos(&mut bytes, reg.table_offset, rec_index, col, row) {
                patched += 1;
            }
        }
        if patched > 0 {
            match registry::write_icon_layouts(&bytes) {
                Ok(_) => registry_synced = true,
                Err(err) => warnings.push(format!("布局已应用，但写回注册表失败：{}", err)),
            }
        } else {
            warnings.push("没有匹配到注册表坐标记录，已跳过直写".into());
        }
    } else if reg_layout.is_some() {
        warnings.push("注册表布局与当前桌面不同步，已跳过直写".into());
    }

    for item in &items {
        CoTaskMemFree(Some(item.pidl as *const c_void));
    }

    Ok(ApplyResult {
        matched,
        missing,
        new_placed,
        auto_arrange_disabled,
        registry_synced,
        warnings,
    })
}
