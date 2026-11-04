use std::env;
use std::fs;

use desktop_icon_backup_lib::desktop::{display, icons, layout, registry};

fn main() {
    display::ensure_dpi_awareness();

    let display_info = display::read_display();
    let icon_dir = env::temp_dir().join("desktop-icon-probe-icons");
    let collected = icons::collect(&icon_dir);
    let snapshot = registry::snapshot_bags();

    let mut output = serde_json::json!({
        "display": display_info,
        "iconDir": icon_dir.display().to_string(),
    });

    match &collected {
        Ok(result) => {
            output["source"] = serde_json::json!(result.source);
            output["iconCount"] = serde_json::json!(result.icons.len());
            output["warnings"] = serde_json::json!(result.warnings);
            output["icons"] = serde_json::json!(result.icons);
        }
        Err(err) => {
            output["source"] = serde_json::json!("failed");
            output["error"] = serde_json::json!(err);
        }
    }

    match &snapshot {
        Ok(snap) => {
            let mut values = Vec::new();
            for (name, value) in &snap.values {
                values.push(serde_json::json!({
                    "name": name,
                    "type": value.vtype,
                    "bytes": value.bytes_hex.len() / 2,
                }));
            }
            output["registry"] = serde_json::json!({
                "values": values,
                "hasIconLayouts": snap.icon_layouts.is_some(),
            });

            if let Ok(bytes) = registry::icon_layouts_bytes(snap) {
                let dump_path = env::temp_dir().join("desktop-icon-layouts.bin");
                let _ = fs::write(&dump_path, &bytes);
                output["layoutDump"] = serde_json::json!(dump_path.display().to_string());

                match layout::parse_names(&bytes) {
                    Ok(parsed) => {
                        output["layoutNames"] = serde_json::json!(parsed);
                    }
                    Err(err) => {
                        output["layoutNamesError"] = serde_json::json!(err);
                    }
                }
            }
        }
        Err(err) => {
            output["registryError"] = serde_json::json!(err);
        }
    }

    let text = serde_json::to_string_pretty(&output).unwrap();
    let out_path = env::temp_dir().join("desktop-icon-probe-output.json");
    let _ = fs::write(&out_path, &text);
    println!("{}", text);
    eprintln!("written to {}", out_path.display());
}
