use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LauncherConfig {
    pub nickname: String,
    pub ram_gb: f32,
    pub java_version: String,
    pub java_path: Option<String>,
    pub window_width: u32,
    pub window_height: u32,
    pub fullscreen: bool,
    pub vsync: bool,
    pub keep_launcher_open: bool,
    pub debug_info: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            nickname: String::from("Player"),
            ram_gb: 4.0,
            java_version: String::from("Java 17"),
            java_path: None,
            window_width: 1280,
            window_height: 720,
            fullscreen: false,
            vsync: true,
            keep_launcher_open: false,
            debug_info: false,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("GrandEden");
    path.push("launcher_config.json");
    path
}

pub fn get_game_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("GrandEden");
    path.push("minecraft");
    path
}
