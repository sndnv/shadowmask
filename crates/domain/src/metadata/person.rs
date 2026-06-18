#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PersonId(pub String);

#[derive(Debug, Clone)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
}
