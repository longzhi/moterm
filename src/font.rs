use std::fs;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};

pub fn load_monospace_font() -> Result<(Font, PathBuf), String> {
    // Prefer Nerd Font (has icons for starship/powerlevel10k prompts)
    let home = std::env::var("HOME").unwrap_or_default();
    let nerd_font = format!("{}/Library/Fonts/FiraCodeNerdFontMono-Regular.ttf", home);

    let candidates = [
        nerd_font.as_str(),
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/Library/Fonts/Courier New.ttf",
        "/System/Library/Fonts/Monaco.ttf",
    ];

    for p in candidates {
        let path = Path::new(p);
        if !path.exists() {
            continue;
        }
        match fs::read(path) {
            Ok(bytes) => {
                if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                    return Ok((font, path.to_path_buf()));
                }
            }
            Err(_) => continue,
        }
    }

    let embedded: &[u8] = include_bytes!("embedded_fallback_font.bin");
    if !embedded.is_empty() {
        if let Ok(font) = Font::from_bytes(embedded, FontSettings::default()) {
            return Ok((font, PathBuf::from("<embedded>")));
        }
    }

    Err("无法加载系统等宽字体；当前仓库未提供可用嵌入字体。".to_string())
}
