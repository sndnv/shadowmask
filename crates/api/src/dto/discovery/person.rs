use serde::Serialize;

use domain::metadata::Person;

#[derive(Debug, Serialize)]
pub struct PersonResponse {
    pub id: String,
    pub name: String,
}

impl From<Person> for PersonResponse {
    fn from(p: Person) -> Self {
        PersonResponse {
            id: p.id.0,
            name: p.name,
        }
    }
}
