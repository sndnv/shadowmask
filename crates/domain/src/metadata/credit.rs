use crate::catalog::TitleRef;
use crate::metadata::PersonId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreditRole {
    Actor,
    Director,
    Writer,
}

#[derive(Debug, Clone)]
pub struct Credit {
    pub person: PersonId,
    pub title: TitleRef,
    pub role: CreditRole,
    pub character: Option<String>,
    pub order: u32,
}
