use std::collections::HashSet;

use crate::catalog::{ArtworkOwner, EpisodeId, MovieId, SeasonId, SeriesId, TitleRef};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TitleSet {
    pub movies: HashSet<MovieId>,
    pub series: HashSet<SeriesId>,
    pub seasons: HashSet<SeasonId>,
    pub episodes: HashSet<EpisodeId>,
}

impl TitleSet {
    pub fn holds(&self, owner: &ArtworkOwner) -> bool {
        match owner {
            ArtworkOwner::Movie(id) => self.movies.contains(id),
            ArtworkOwner::Series(id) => self.series.contains(id),
            ArtworkOwner::Season(id) => self.seasons.contains(id),
            ArtworkOwner::Episode(id) => self.episodes.contains(id),
            ArtworkOwner::Collection(_) | ArtworkOwner::Person(_) => false,
        }
    }

    pub fn holds_title(&self, title: &TitleRef) -> bool {
        match title {
            TitleRef::Movie(id) => self.movies.contains(id),
            TitleRef::Series(id) => self.series.contains(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CollectionId;
    use crate::metadata::PersonId;

    fn set() -> TitleSet {
        TitleSet {
            movies: HashSet::from([MovieId("m".into())]),
            series: HashSet::from([SeriesId("s".into())]),
            seasons: HashSet::from([SeasonId("se".into())]),
            episodes: HashSet::from([EpisodeId("e".into())]),
        }
    }

    #[test]
    fn an_owner_is_held_by_its_own_kind_only() {
        let held = set();
        for owner in [
            ArtworkOwner::Movie(MovieId("m".into())),
            ArtworkOwner::Series(SeriesId("s".into())),
            ArtworkOwner::Season(SeasonId("se".into())),
            ArtworkOwner::Episode(EpisodeId("e".into())),
        ] {
            assert!(held.holds(&owner), "{owner:?}");
        }
        for owner in [
            ArtworkOwner::Movie(MovieId("s".into())),
            ArtworkOwner::Series(SeriesId("m".into())),
            ArtworkOwner::Season(SeasonId("e".into())),
            ArtworkOwner::Episode(EpisodeId("se".into())),
            ArtworkOwner::Collection(CollectionId("m".into())),
            ArtworkOwner::Person(PersonId("m".into())),
        ] {
            assert!(!held.holds(&owner), "{owner:?}");
        }
    }

    #[test]
    fn a_title_is_held_by_its_own_kind_only() {
        let held = set();
        assert!(held.holds_title(&TitleRef::Movie(MovieId("m".into()))));
        assert!(held.holds_title(&TitleRef::Series(SeriesId("s".into()))));
        assert!(!held.holds_title(&TitleRef::Movie(MovieId("s".into()))));
        assert!(!held.holds_title(&TitleRef::Series(SeriesId("m".into()))));
    }
}
