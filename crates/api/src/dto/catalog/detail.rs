use serde::Serialize;

use domain::metadata::{
    CreditRole, CreditedPerson, ExternalId, Extra, ExtraKind, Genre, Person, Rating, Studio,
};

#[derive(Debug, Serialize)]
pub struct GenreDto {
    pub id: String,
    pub name: String,
}

impl From<Genre> for GenreDto {
    fn from(g: Genre) -> Self {
        GenreDto { id: g.id.0, name: g.name }
    }
}

#[derive(Debug, Serialize)]
pub struct StudioDto {
    pub id: String,
    pub name: String,
}

impl From<Studio> for StudioDto {
    fn from(s: Studio) -> Self {
        StudioDto { id: s.id.0, name: s.name }
    }
}

#[derive(Debug, Serialize)]
pub struct RatingDto {
    pub source: String,
    pub value: f32,
}

impl From<Rating> for RatingDto {
    fn from(r: Rating) -> Self {
        RatingDto { source: r.source, value: r.value }
    }
}

#[derive(Debug, Serialize)]
pub struct ExternalIdDto {
    pub source: String,
    pub value: String,
}

impl From<ExternalId> for ExternalIdDto {
    fn from(e: ExternalId) -> Self {
        ExternalIdDto { source: e.source, value: e.value }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CreditRoleDto {
    Actor,
    Director,
    Writer,
}

impl From<CreditRole> for CreditRoleDto {
    fn from(r: CreditRole) -> Self {
        match r {
            CreditRole::Actor => CreditRoleDto::Actor,
            CreditRole::Director => CreditRoleDto::Director,
            CreditRole::Writer => CreditRoleDto::Writer,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PersonRefDto {
    pub id: String,
    pub name: String,
}

impl From<Person> for PersonRefDto {
    fn from(p: Person) -> Self {
        PersonRefDto { id: p.id.0, name: p.name }
    }
}

#[derive(Debug, Serialize)]
pub struct CreditDto {
    pub person: PersonRefDto,
    pub role: CreditRoleDto,
    pub character: Option<String>,
    pub order: u32,
}

impl From<CreditedPerson> for CreditDto {
    fn from(c: CreditedPerson) -> Self {
        CreditDto {
            person: c.person.into(),
            role: c.role.into(),
            character: c.character,
            order: c.order,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraKindDto {
    Trailer,
    Featurette,
    BehindTheScenes,
}

impl From<ExtraKind> for ExtraKindDto {
    fn from(k: ExtraKind) -> Self {
        match k {
            ExtraKind::Trailer => ExtraKindDto::Trailer,
            ExtraKind::Featurette => ExtraKindDto::Featurette,
            ExtraKind::BehindTheScenes => ExtraKindDto::BehindTheScenes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExtraDto {
    pub kind: ExtraKindDto,
    pub title: String,
}

impl From<Extra> for ExtraDto {
    fn from(e: Extra) -> Self {
        ExtraDto { kind: e.kind.into(), title: e.title }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::metadata::{GenreId, PersonId, StudioId};
    use serde_json::json;

    #[test]
    fn credit_role_maps_every_variant() {
        for (domain, dto) in [
            (CreditRole::Actor, CreditRoleDto::Actor),
            (CreditRole::Director, CreditRoleDto::Director),
            (CreditRole::Writer, CreditRoleDto::Writer),
        ] {
            assert_eq!(
                serde_json::to_string(&CreditRoleDto::from(domain)).unwrap(),
                serde_json::to_string(&dto).unwrap()
            );
        }
    }

    #[test]
    fn extra_kind_maps_every_variant() {
        for (domain, dto) in [
            (ExtraKind::Trailer, ExtraKindDto::Trailer),
            (ExtraKind::Featurette, ExtraKindDto::Featurette),
            (ExtraKind::BehindTheScenes, ExtraKindDto::BehindTheScenes),
        ] {
            assert_eq!(
                serde_json::to_string(&ExtraKindDto::from(domain)).unwrap(),
                serde_json::to_string(&dto).unwrap()
            );
        }
    }

    #[test]
    fn extra_drops_the_server_side_path() {
        let dto = ExtraDto::from(Extra {
            kind: ExtraKind::Trailer,
            title: "Teaser".into(),
            path: "/extras/teaser.mkv".into(),
        });
        assert_eq!(
            serde_json::to_value(&dto).unwrap(),
            json!({"kind": "trailer", "title": "Teaser"})
        );
    }

    #[test]
    fn leaf_dtos_carry_ids_and_values() {
        assert_eq!(
            serde_json::to_value(GenreDto::from(Genre {
                id: GenreId("g1".into()),
                name: "Action".into(),
            }))
            .unwrap(),
            json!({"id": "g1", "name": "Action"})
        );
        assert_eq!(
            serde_json::to_value(StudioDto::from(Studio {
                id: StudioId("st1".into()),
                name: "Acme".into(),
            }))
            .unwrap(),
            json!({"id": "st1", "name": "Acme"})
        );
        assert_eq!(
            serde_json::to_value(RatingDto::from(Rating { source: "tmdb".into(), value: 8.5 }))
                .unwrap(),
            json!({"source": "tmdb", "value": 8.5})
        );
        assert_eq!(
            serde_json::to_value(ExternalIdDto::from(ExternalId {
                source: "imdb".into(),
                value: "tt1".into(),
            }))
            .unwrap(),
            json!({"source": "imdb", "value": "tt1"})
        );
        assert_eq!(
            serde_json::to_value(CreditDto::from(CreditedPerson {
                person: Person {
                    id: PersonId("p1".into()),
                    name: "Ada".into(),
                    ..Person::default()
                },
                role: CreditRole::Actor,
                character: Some("Hero".into()),
                order: 0,
            }))
            .unwrap(),
            json!({
                "person": {"id": "p1", "name": "Ada"},
                "role": "actor",
                "character": "Hero",
                "order": 0
            })
        );
    }
}
