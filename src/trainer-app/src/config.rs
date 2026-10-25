//! 控制软件自己的设置，存在 %APPDATA%\CyanBrainTrainer\settings.ini（key=value，UTF-8）。

use std::fs;
use std::path::PathBuf;

use crate::protocol::DEFAULT_PORT;

pub struct AppConfig {
    pub game_dir: Option<PathBuf>,
    pub port: u16,
}

impl AppConfig {
    fn file_path() -> Option<PathBuf> {
        let base = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(base).join("CyanBrainTrainer").join("settings.ini"))
    }

    pub fn load() -> Self {
        let mut config = AppConfig { game_dir: None, port: DEFAULT_PORT };
        let Some(text) = Self::file_path().and_then(|p| fs::read_to_string(p).ok()) else {
            return config;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match key.trim() {
                "game_dir" if !value.trim().is_empty() => config.game_dir = Some(PathBuf::from(value.trim())),
                "port" => config.port = value.trim().parse().unwrap_or(DEFAULT_PORT),
                _ => {}
            }
        }
        config
    }

    pub fn save(&self) {
        let Some(path) = Self::file_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let game_dir = self.game_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        let _ = fs::write(path, format!("game_dir={game_dir}\nport={}\n", self.port));
    }
}
