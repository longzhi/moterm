use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub font: FontConfig,
    pub window: WindowConfig,
    pub cursor: CursorConfig,
    pub colors: ColorConfig,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct FontConfig {
    pub family: Option<String>,
    pub size: f32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct CursorConfig {
    pub style: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct ColorConfig {
    pub background: String,
    pub foreground: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: FontConfig::default(),
            window: WindowConfig::default(),
            cursor: CursorConfig::default(),
            colors: ColorConfig::default(),
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: None,
            size: 14.0,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 960,
            height: 600,
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: "block".to_string(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            background: "#1e1e2e".to_string(),
            foreground: "#cdd6f4".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if !path.exists() {
            eprintln!("配置文件不存在，使用默认配置: {}", path.display());
            return Config::default();
        }
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => {
                    eprintln!("已加载配置: {}", path.display());
                    cfg
                }
                Err(e) => {
                    eprintln!("配置文件解析失败: {e}，使用默认配置");
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("读取配置失败: {e}，使用默认配置");
                Config::default()
            }
        }
    }

    pub fn initial_cursor_style(&self) -> crate::terminal::CursorStyle {
        match self.cursor.style.as_str() {
            "beam" | "bar" => crate::terminal::CursorStyle::Beam,
            "underline" => crate::terminal::CursorStyle::Underline,
            _ => crate::terminal::CursorStyle::Block,
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("moterm")
        .join("config.toml")
}
