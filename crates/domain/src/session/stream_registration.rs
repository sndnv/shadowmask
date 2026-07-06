use std::path::PathBuf;

use crate::session::DeliveryMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleRendition {
    pub name: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct StreamRegistration {
    pub mode: DeliveryMode,
    pub output_dir: PathBuf,
    pub direct_path: Option<PathBuf>,
    pub bandwidth: u64,
    pub subtitle: Option<SubtitleRendition>,
}
