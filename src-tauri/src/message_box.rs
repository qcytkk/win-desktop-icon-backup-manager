use windows::core::HSTRING;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK};

pub fn info(title: &str, text: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(title),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

pub fn error(title: &str, text: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(title),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn backup_result(language: &str, result: Result<usize, String>) {
    let (title, ok_text, err_text) = match language {
        "zh-TW" => (
            "桌面圖示備份",
            "已備份目前桌面配置（{n} 個圖示）",
            "備份失敗：{msg}",
        ),
        "en" => (
            "Desktop Icon Backup",
            "Desktop layout backed up ({n} icons)",
            "Backup failed: {msg}",
        ),
        "fr" => (
            "Sauvegarde des icônes",
            "Disposition du bureau sauvegardée ({n} icônes)",
            "Échec de la sauvegarde : {msg}",
        ),
        "ko" => (
            "바탕화면 아이콘 백업",
            "바탕화면 배치를 백업했습니다({n}개 아이콘)",
            "백업 실패: {msg}",
        ),
        "ja" => (
            "デスクトップアイコン配置バックアップ",
            "デスクトップ配置をバックアップしました（{n} 個）",
            "バックアップ失敗：{msg}",
        ),
        _ => (
            "桌面图标备份",
            "已备份当前桌面布局（{n} 个图标）",
            "备份失败：{msg}",
        ),
    };

    match result {
        Ok(count) => info(title, &ok_text.replace("{n}", &count.to_string())),
        Err(err) => error(title, &err_text.replace("{msg}", &err)),
    }
}
