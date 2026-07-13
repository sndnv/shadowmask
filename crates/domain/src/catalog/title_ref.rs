use crate::catalog::{MovieId, SeriesId, TitleKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TitleRef {
    Movie(MovieId),
    Series(SeriesId),
}

impl TitleRef {
    pub fn kind(&self) -> TitleKind {
        match self {
            TitleRef::Movie(_) => TitleKind::Movie,
            TitleRef::Series(_) => TitleKind::Series,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            TitleRef::Movie(id) => &id.0,
            TitleRef::Series(id) => &id.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MovieId, SeriesId};

    #[test]
    fn kind_and_id_reflect_variant() {
        let movie = TitleRef::Movie(MovieId("m1".to_owned()));
        assert_eq!(movie.kind(), TitleKind::Movie);
        assert_eq!(movie.id(), "m1");

        let series = TitleRef::Series(SeriesId("s1".to_owned()));
        assert_eq!(series.kind(), TitleKind::Series);
        assert_eq!(series.id(), "s1");
    }
}
