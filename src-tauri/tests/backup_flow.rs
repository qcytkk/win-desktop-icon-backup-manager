use desktop_icon_backup_lib::desktop::{display, icons, registry};
use desktop_icon_backup_lib::storage::backup;

#[test]
fn backup_pipeline_reads_real_desktop() {
    let root = std::env::temp_dir().join(format!("dcib-flow-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(backup::icons_dir(&root)).unwrap();

    let display_info = display::read_display();
    assert!(display_info.width > 0 && display_info.height > 0);

    let snapshot = registry::snapshot_bags().expect("读取桌面布局注册表失败");
    let layout_bytes = registry::icon_layouts_bytes(&snapshot).unwrap_or_default();

    let collected = icons::collect(&backup::icons_dir(&root)).expect("读取桌面图标失败");
    assert!(!collected.icons.is_empty(), "桌面图标为空");
    assert!(
        collected.icons.iter().any(|icon| icon.x >= 0 && icon.y >= 0),
        "未采集到任何坐标"
    );
    assert!(
        collected.icons.iter().filter(|icon| icon.icon_key.is_some()).count() > 10,
        "图标位图过少"
    );

    let record = backup::create(
        &root,
        "集成测试",
        display_info,
        collected.icons,
        &collected.source,
        collected.warnings,
        &layout_bytes,
        "{}",
    )
    .expect("创建备份失败");

    assert_eq!(record.icon_count, 131);

    let listed = backup::list(&root).unwrap();
    assert_eq!(listed.len(), 1);

    let loaded = backup::load_icons(&root, &record.id).unwrap();
    assert!(loaded.len() > 10, "图标缓存读取失败");

    let _ = std::fs::remove_dir_all(&root);
}
