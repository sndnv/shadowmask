use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::enrichment::ProviderChoice;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpscalingConfig {
    pub enabled: bool,
    pub provider: ProviderChoice,
    pub model_path: Option<PathBuf>,
    pub target_height: u32,
}

impl Default for UpscalingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: ProviderChoice::None,
            model_path: None,
            target_height: 1080,
        }
    }
}
