use serde::Serialize;

use super::FontFile;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontSet {
    pub name: String,
    pub regular: FontFile,
    pub medium: FontFile,
    pub license: FontFile,
    pub italic: Option<FontFile>,
    pub italic_license: Option<FontFile>,
}
