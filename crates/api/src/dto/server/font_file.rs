use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontFile {
    pub url: String,
    pub size: u64,
    pub sha256: String,
}
