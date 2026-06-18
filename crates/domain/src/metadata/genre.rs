#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenreId(pub String);

#[derive(Debug, Clone)]
pub struct Genre {
    pub id: GenreId,
    pub name: String,
}
