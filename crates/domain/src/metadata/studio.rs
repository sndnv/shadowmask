#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StudioId(pub String);

#[derive(Debug, Clone)]
pub struct Studio {
    pub id: StudioId,
    pub name: String,
}
