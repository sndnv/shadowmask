use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedAssetDir {
    pub owner: String,
    pub modified_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedAssetFile {
    pub path: String,
    pub modified_at: Timestamp,
}
