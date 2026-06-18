use crate::catalog::TitleId;
use crate::common::Quality;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionId(pub String);

#[derive(Debug, Clone)]
pub struct Version {
    pub id: VersionId,
    pub title: TitleId,
    pub quality: Quality,
    pub container: String,
    pub path: String,
    pub size_bytes: u64,
}
