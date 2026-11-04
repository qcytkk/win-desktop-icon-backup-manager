use std::mem::size_of;

use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, SetProcessDpiAwarenessContext, MDT_EFFECTIVE_DPI,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

use crate::models::{DisplayInfo, MonitorInfo, Rect};

pub fn ensure_dpi_awareness() {
    // 进程若已设置过 DPI 感知会返回错误，这里忽略即可。
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

struct Accum {
    monitors: Vec<MonitorInfo>,
    dpi: u32,
}

unsafe extern "system" fn enum_monitor(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let acc = &mut *(data.0 as *mut Accum);

    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if !GetMonitorInfoW(hmonitor, &mut info.monitorInfo).as_bool() {
        return TRUE;
    }

    let r = info.monitorInfo.rcMonitor;
    let primary = info.monitorInfo.dwFlags & 1 != 0; // MONITORINFOF_PRIMARY
    acc.monitors.push(MonitorInfo {
        x: r.left,
        y: r.top,
        width: (r.right - r.left).max(0) as u32,
        height: (r.bottom - r.top).max(0) as u32,
        primary,
    });

    if primary {
        let mut dpi_x = 96u32;
        let mut dpi_y = 96u32;
        if GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
            acc.dpi = dpi_x;
        }
    }

    TRUE
}

pub fn read_display() -> DisplayInfo {
    let mut acc = Accum {
        monitors: Vec::new(),
        dpi: 96,
    };

    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_monitor),
            LPARAM(&mut acc as *mut Accum as isize),
        );
    }

    if acc.monitors.is_empty() {
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) }.max(1) as u32;
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) }.max(1) as u32;
        acc.monitors.push(MonitorInfo {
            x: 0,
            y: 0,
            width,
            height,
            primary: true,
        });
    }

    let primary = acc
        .monitors
        .iter()
        .find(|m| m.primary)
        .cloned()
        .unwrap_or_else(|| acc.monitors[0].clone());

    let min_x = acc.monitors.iter().map(|m| m.x).min().unwrap_or(0);
    let min_y = acc.monitors.iter().map(|m| m.y).min().unwrap_or(0);
    let max_x = acc
        .monitors
        .iter()
        .map(|m| m.x + m.width as i32)
        .max()
        .unwrap_or(primary.width as i32);
    let max_y = acc
        .monitors
        .iter()
        .map(|m| m.y + m.height as i32)
        .max()
        .unwrap_or(primary.height as i32);

    let dpi = if acc.dpi > 0 { acc.dpi } else { 96 };

    DisplayInfo {
        width: primary.width,
        height: primary.height,
        dpi,
        scale_factor: dpi as f64 / 96.0,
        virtual_bounds: Rect {
            x: min_x,
            y: min_y,
            width: (max_x - min_x).max(1) as u32,
            height: (max_y - min_y).max(1) as u32,
        },
        monitors: acc.monitors,
    }
}
