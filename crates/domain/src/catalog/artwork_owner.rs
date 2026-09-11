use crate::catalog::{CollectionId, EpisodeId, MovieId, SeasonId, SeriesId};
use crate::metadata::PersonId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtworkOwner {
    Movie(MovieId),
    Series(SeriesId),
    Season(SeasonId),
    Episode(EpisodeId),
    Collection(CollectionId),
    Person(PersonId),
}

impl ArtworkOwner {
    pub fn kind(&self) -> &'static str {
        match self {
            ArtworkOwner::Movie(_) => "movie",
            ArtworkOwner::Series(_) => "series",
            ArtworkOwner::Season(_) => "season",
            ArtworkOwner::Episode(_) => "episode",
            ArtworkOwner::Collection(_) => "collection",
            ArtworkOwner::Person(_) => "person",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            ArtworkOwner::Movie(id) => &id.0,
            ArtworkOwner::Series(id) => &id.0,
            ArtworkOwner::Season(id) => &id.0,
            ArtworkOwner::Episode(id) => &id.0,
            ArtworkOwner::Collection(id) => &id.0,
            ArtworkOwner::Person(id) => &id.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_and_id_reflect_variant() {
        let owners: Vec<(ArtworkOwner, &str)> = vec![
            (ArtworkOwner::Movie(MovieId("m1".to_owned())), "movie"),
            (ArtworkOwner::Series(SeriesId("s1".to_owned())), "series"),
            (ArtworkOwner::Season(SeasonId("se1".to_owned())), "season"),
            (ArtworkOwner::Episode(EpisodeId("e1".to_owned())), "episode"),
            (ArtworkOwner::Collection(CollectionId("c1".to_owned())), "collection"),
            (ArtworkOwner::Person(PersonId("p1".to_owned())), "person"),
        ];
        for (owner, kind) in owners {
            assert_eq!(owner.kind(), kind);
            assert!(!owner.id().is_empty());
        }
    }
}
