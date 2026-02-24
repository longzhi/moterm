use std::fs;
use std::path::{Path, PathBuf};

use fontdue::{Font, FontSettings};

use crate::config::Config;

pub fn load_monospace_font(cfg: &Config) -> Result<(Font, PathBuf), String> {
    // If user specified a font family in config, try to find it
    let home = std::env::var("HOME").unwrap_or_default();

    let mut custom_paths: Vec<String> = Vec::new();
    if let Some(ref family) = cfg.font.family {
        // Try common locations with the family name
        let clean = family.replace(' ', "");
        custom_paths.push(format!("{}/Library/Fonts/{}-Regular.ttf", home, clean));
        custom_paths.push(format!("{}/Library/Fonts/{}.ttf", home, clean));
        custom_paths.push(format!("/Library/Fonts/{}-Regular.ttf", clean));
        custom_paths.push(format!("/Library/Fonts/{}.ttf", clean));
    }

    // Default: prefer Nerd Font
    let nerd_font = format!("{}/Library/Fonts/FiraCodeNerdFontMono-Regular.ttf", home);

    let system_fonts = [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/Library/Fonts/Courier New.ttf",
        "/System/Library/Fonts/Monaco.ttf",
    ];

    let mut all_candidates: Vec<&str> = custom_paths.iter().map(|s| s.as_str()).collect();
    all_candidates.push(nerd_font.as_str());
    all_candidates.extend(system_fonts.iter());

    for p in all_candidates {
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

/// Load CJK fallback fonts from the system
pub fn load_fallback_fonts() -> Vec<Font> {
    let cjk_candidates = [
        // macOS CJK fonts
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/Supplemental/Songti.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        // Symbols
        "/System/Library/Fonts/Apple Color Emoji.ttc",
    ];

    let mut fonts = Vec::new();
    for p in cjk_candidates {
        let path = Path::new(p);
        if !path.exists() {
            continue;
        }
        if let Ok(bytes) = fs::read(path) {
            if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
                eprintln!("回退字体: {}", p);
                fonts.push(font);
                if fonts.len() >= 2 {
                    break; // 2 fallbacks is enough
                }
            }
        }
    }
    fonts
}
