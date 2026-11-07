use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AddedResponse {
    pub added: usize,
}
