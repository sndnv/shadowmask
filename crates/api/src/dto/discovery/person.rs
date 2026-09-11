use serde::Serialize;

use domain::metadata::Person;

use crate::dto::common::ArtworkDto;

#[derive(Debug, Serialize)]
pub struct PersonResponse {
    pub id: String,
    pub name: String,
    pub artwork: ArtworkDto,
}

impl From<Person> for PersonResponse {
    fn from(p: Person) -> Self {
        PersonResponse { id: p.id.0, name: p.name, artwork: ArtworkDto::from_refs(p.artwork) }
    }
}
