use crate::metadata::Artwork;

#[derive(Debug, Clone, Default)]
pub struct PersonMetadata {
    pub name: String,
    pub biography: Option<String>,
    pub birthday: Option<String>,
    pub deathday: Option<String>,
    pub place_of_birth: Option<String>,
    pub also_known_as: Vec<String>,
    pub artwork: Vec<Artwork>,
}
