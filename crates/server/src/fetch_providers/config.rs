use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FetchProvidersConfig {
    pub enabled: bool,
    pub yt_dlp_binary: String,
    pub yt_dlp_plugin_dir: Option<PathBuf>,
    pub max_height: Option<u32>,
    pub cookies_file: Option<PathBuf>,
}

impl Default for FetchProvidersConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            yt_dlp_binary: "yt-dlp".to_owned(),
            yt_dlp_plugin_dir: None,
            max_height: None,
            cookies_file: None,
        }
    }
}
