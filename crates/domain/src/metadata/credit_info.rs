use crate::metadata::{CreditRole, ExternalId};

#[derive(Debug, Clone)]
pub struct CreditInfo {
    pub external_person_id: ExternalId,
    pub name: String,
    pub role: CreditRole,
    pub character: Option<String>,
    pub order: u32,
}
