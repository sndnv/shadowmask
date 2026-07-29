use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::enrichment::ProviderChoice;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranslationConfig {
    pub enabled: bool,
    pub provider: ProviderChoice,
    pub model_path: Option<PathBuf>,
    pub source_prefix: Option<String>,
    pub target_prefix: Option<String>,
}
