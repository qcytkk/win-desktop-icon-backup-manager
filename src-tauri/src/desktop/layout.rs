use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutNames {
    pub count: usize,
    pub names: Vec<String>,
    pub name_table_end: usize,
    pub total_len: usize,
}

pub fn parse_names(bytes: &[u8]) -> Result<LayoutNames, String> {
    if bytes.len() < 0x1c {
        return Err("IconLayouts 数据过短".into());
    }

    let count = read_u32(bytes, 0x18)? as usize;
    if count == 0 || count > 4096 {
        return Err(format!("IconLayouts 名称数量异常：{}", count));
    }

    let mut pos = 0x20;
    let mut names = Vec::with_capacity(count);

    for index in 0..count {
        if pos + 8 > bytes.len() {
            return Err(format!("IconLayouts 第 {} 项越界", index));
        }
        let chars = read_u32(bytes, pos)? as usize;
        let byte_len = chars * 2;
        if chars == 0 || pos + 8 + byte_len > bytes.len() {
            return Err(format!("IconLayouts 第 {} 项名称长度异常", index));
        }
        let start = pos + 8;
        let mut utf16 = Vec::with_capacity(chars);
        for i in 0..chars {
            utf16.push(read_u16(bytes, start + i * 2)?);
        }
        let raw = String::from_utf16_lossy(&utf16);
        let trimmed = match raw.find('\0') {
            Some(p) => &raw[..p],
            None => &raw[..],
        };
        names.push(trimmed.to_string());
        pos = start + byte_len;
    }

    Ok(LayoutNames {
        count,
        names,
        name_table_end: pos,
        total_len: bytes.len(),
    })
}

pub fn clean_name(raw: &str) -> String {
    let mut s = raw.trim_end();
    if let Some(stripped) = s.strip_suffix('>') {
        s = stripped.trim_end();
    }
    s.to_string()
}

pub fn kind_from_name(name: &str) -> &'static str {
    if name.starts_with("::") {
        return "system";
    }
    let lower = name.to_lowercase();
    if lower.ends_with(".lnk") {
        "shortcut"
    } else if lower.ends_with(".url") {
        "url"
    } else {
        "file"
    }
}

pub fn clsid_display_name(name: &str) -> Option<&'static str> {
    let id = name.trim_start_matches("::");
    let id = id.trim_start_matches('{').trim_end_matches('}').to_uppercase();
    match id.as_str() {
        "645FF040-5081-101B-9F08-00AA002F954E" => Some("回收站"),
        "20D04FE0-3AEA-1069-A2D8-08002B30309D" => Some("此电脑"),
        "F02C1A0D-BE21-4350-88B0-7367FC96EF3C" => Some("网络"),
        "5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0" => Some("控制面板"),
        "59031A47-3F72-44A7-89C5-5595FE6B30EE" => Some("用户文件夹"),
        "031E4825-7B94-4DC3-B131-E946B44C8DD5" => Some("库"),
        _ => None,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "读取 u32 越界".to_string())?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| "读取 u16 越界".to_string())?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_sample(names: &[&str]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x20];
        bytes[0x18..0x1c].copy_from_slice(&(names.len() as u32).to_le_bytes());
        for name in names {
            let text = format!("{}>  \0", name);
            let chars = text.encode_utf16().count() as u32;
            bytes.extend_from_slice(&chars.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn parses_name_table() {
        let bytes = build_sample(&["Krita.lnk", "::{645FF040-5081-101B-9F08-00AA002F954E}"]);
        let parsed = parse_names(&bytes).unwrap();
        assert_eq!(parsed.count, 2);
        assert_eq!(parsed.names[0], "Krita.lnk>  ");
        assert_eq!(clean_name(&parsed.names[0]), "Krita.lnk");
        assert_eq!(
            clsid_display_name(&clean_name(&parsed.names[1])),
            Some("回收站")
        );
        assert_eq!(parsed.name_table_end, bytes.len());
    }
}
