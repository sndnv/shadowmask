use crate::catalog::ArtworkRef;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct PersonId(pub String);

#[derive(Debug, Clone, Default)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub biography: Option<String>,
    pub birthday: Option<String>,
    pub deathday: Option<String>,
    pub place_of_birth: Option<String>,
    pub also_known_as: Vec<String>,
    pub external_id: Option<String>,
    pub artwork: Vec<ArtworkRef>,
}
