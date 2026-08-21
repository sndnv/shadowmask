use crate::catalog::{CollectionId, SeasonId, SeriesId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RandomScope {
    Movies,
    Episodes,
    Collection(CollectionId),
    Series(SeriesId),
    Season(SeasonId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_compare_by_target() {
        let first = RandomScope::Series(SeriesId("s1".to_owned()));
        assert_eq!(first, RandomScope::Series(SeriesId("s1".to_owned())));
        assert_ne!(first, RandomScope::Series(SeriesId("s2".to_owned())));
        assert_ne!(RandomScope::Movies, RandomScope::Episodes);
    }
}
