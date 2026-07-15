use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    pub token: Option<String>,
}
