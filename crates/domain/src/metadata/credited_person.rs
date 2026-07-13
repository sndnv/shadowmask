use crate::metadata::{CreditRole, Person};

#[derive(Debug, Clone)]
pub struct CreditedPerson {
    pub person: Person,
    pub role: CreditRole,
    pub character: Option<String>,
    pub order: u32,
}
