//! Настройки программы: хранятся в settings.json в папке программы.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Горячая клавиша, например "CommandOrControl+Shift+Space"
    pub hotkey: String,
    /// "hold" — держать клавишу, пока говоришь; "toggle" — нажал начать, нажал закончить
    pub mode: String,
    /// "auto", "ru", "en"
    pub language: String,
    /// id модели: "base", "small", "turbo"
    pub model: String,
    pub autostart: bool,
    /// Прошёл ли человек приветствие
    pub onboarded: bool,
    /// Показывать/вписывать текст сразу, пока говоришь
    pub live: bool,
    /// Обновляться автоматически
    pub auto_update: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "CommandOrControl+Shift+Space".into(),
            mode: "hold".into(),
            language: "auto".into(),
            model: "small".into(),
            autostart: false,
            onboarded: false,
            live: true,
            auto_update: true,
        }
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

pub fn load(dir: &Path) -> Settings {
    std::fs::read_to_string(path(dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, s: &Settings) {
    let _ = std::fs::create_dir_all(dir);
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(path(dir), json);
    }
}
