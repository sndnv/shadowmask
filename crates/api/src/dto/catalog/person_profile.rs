use serde::Serialize;

use domain::catalog::{FilmographyEntry, PersonProfile, TitleKind};

use crate::dto::catalog::detail::CreditRoleDto;
use crate::dto::common::ArtworkDto;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleKindDto {
    Movie,
    Series,
}

impl From<TitleKind> for TitleKindDto {
    fn from(k: TitleKind) -> Self {
        match k {
            TitleKind::Movie => TitleKindDto::Movie,
            TitleKind::Series => TitleKindDto::Series,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PersonProfileResponse {
    pub id: String,
    pub name: String,
    pub filmography: Vec<FilmographyEntryDto>,
}

#[derive(Debug, Serialize)]
pub struct FilmographyEntryDto {
    pub title_id: String,
    pub kind: TitleKindDto,
    pub display_title: String,
    pub year: Option<u16>,
    pub artwork: ArtworkDto,
    pub role: CreditRoleDto,
    pub character: Option<String>,
}

impl From<FilmographyEntry> for FilmographyEntryDto {
    fn from(e: FilmographyEntry) -> Self {
        FilmographyEntryDto {
            title_id: e.title.id().to_owned(),
            kind: e.title.kind().into(),
            display_title: e.display_title,
            year: e.year,
            artwork: ArtworkDto::from_refs(e.artwork),
            role: e.role.into(),
            character: e.character,
        }
    }
}

impl From<PersonProfile> for PersonProfileResponse {
    fn from(p: PersonProfile) -> Self {
        PersonProfileResponse {
            id: p.person.id.0,
            name: p.person.name,
            filmography: p.filmography.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, SeriesId, TitleRef};
    use domain::metadata::{CreditRole, Person, PersonId};
    use serde_json::json;

    #[test]
    fn title_kind_maps_both_variants() {
        assert_eq!(
            serde_json::to_string(&TitleKindDto::from(TitleKind::Movie)).unwrap(),
            "\"movie\""
        );
        assert_eq!(
            serde_json::to_string(&TitleKindDto::from(TitleKind::Series)).unwrap(),
            "\"series\""
        );
    }

    #[test]
    fn profile_serializes_filmography() {
        let profile = PersonProfile {
            person: Person {
                id: PersonId("p1".into()),
                name: "Ada".into(),
            },
            filmography: vec![
                FilmographyEntry {
                    title: TitleRef::Movie(MovieId("m1".into())),
                    display_title: "Alpha".into(),
                    year: Some(2020),
                    artwork: Vec::new(),
                    role: CreditRole::Actor,
                    character: Some("Hero".into()),
                },
                FilmographyEntry {
                    title: TitleRef::Series(SeriesId("s1".into())),
                    display_title: "Gamma".into(),
                    year: None,
                    artwork: Vec::new(),
                    role: CreditRole::Director,
                    character: None,
                },
            ],
        };
        assert_eq!(
            serde_json::to_value(PersonProfileResponse::from(profile)).unwrap(),
            json!({
                "id": "p1",
                "name": "Ada",
                "filmography": [
                    {
                        "title_id": "m1",
                        "kind": "movie",
                        "display_title": "Alpha",
                        "year": 2020,
                        "artwork": {},
                        "role": "actor",
                        "character": "Hero"
                    },
                    {
                        "title_id": "s1",
                        "kind": "series",
                        "display_title": "Gamma",
                        "year": null,
                        "artwork": {},
                        "role": "director",
                        "character": null
                    }
                ]
            })
        );
    }
}
