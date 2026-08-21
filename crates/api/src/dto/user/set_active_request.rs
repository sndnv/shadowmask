use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetActiveRequest {
    pub active: bool,
}
