use serde::Deserialize;

use crate::dto::library::ResolveTargetInput;

#[derive(Debug, Deserialize)]
pub struct RelinkRequest {
    pub target: ResolveTargetInput,
}
