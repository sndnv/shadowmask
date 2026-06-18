#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagId(pub String);

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
}
