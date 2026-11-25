use crate::models::{AppConfig, HistoryEntry};
use std::fs;
use std::path::PathBuf;

fn app_data_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("RustTube");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

fn history_path() -> PathBuf {
    app_data_dir().join("history.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config(cfg: &AppConfig) {
    let path = config_path();
    if let Ok(text) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, text);
    }
}

pub fn load_history() -> Vec<HistoryEntry> {
    let path = history_path();
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&text) {
            return entries;
        }
    }
    Vec::new()
}

pub fn save_history(entries: &[HistoryEntry]) {
    let path = history_path();
    if let Ok(text) = serde_json::to_string_pretty(entries) {
        let _ = fs::write(path, text);
    }
}
