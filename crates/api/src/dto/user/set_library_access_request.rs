use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetLibraryAccessRequest {
    pub libraries: Vec<String>,
}
