use domain::error::MetadataError;
use domain::metadata::{
    ExternalId, MetadataMatch, MetadataProvider, MetadataQuery, PersonMetadata, SeasonArtwork,
    TitleMetadata,
};
use tracing::warn;

#[derive(Clone)]
pub struct CombiningProvider<P, R> {
    primary: P,
    ratings: Option<R>,
}

impl<P, R> CombiningProvider<P, R> {
    pub fn new(primary: P, ratings: Option<R>) -> Self {
        Self { primary, ratings }
    }
}

impl<P, R> MetadataProvider for CombiningProvider<P, R>
where
    P: MetadataProvider + Send + Sync,
    R: MetadataProvider + Send + Sync,
{
    async fn search(&self, query: &MetadataQuery) -> Result<Vec<MetadataMatch>, MetadataError> {
        self.primary.search(query).await
    }

    async fn fetch(&self, id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
        let mut metadata = self.primary.fetch(id).await?;
        let Some(ratings) = &self.ratings else {
            return Ok(metadata);
        };
        if metadata.content_rating.is_some() {
            return Ok(metadata);
        }
        let Some(imdb) =
            metadata.external_ids.iter().find(|external| external.source == "imdb").cloned()
        else {
            return Ok(metadata);
        };
        match ratings.fetch(&imdb).await {
            Ok(rated) => {
                metadata.content_rating = rated.content_rating;
                if metadata.ratings.is_empty() {
                    metadata.ratings = rated.ratings;
                }
            }
            Err(err) => warn!(
                imdb_id = %imdb.value,
                "rating provider lookup failed; content rating unset, parental controls will not gate this title: {err}"
            ),
        }
        Ok(metadata)
    }

    async fn fetch_season(
        &self,
        id: &ExternalId,
        season: u16,
    ) -> Result<SeasonArtwork, MetadataError> {
        self.primary.fetch_season(id, season).await
    }

    async fn fetch_person(&self, id: &ExternalId) -> Result<PersonMetadata, MetadataError> {
        self.primary.fetch_person(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::metadata::{ContentRating, MediaKind, Rating};

    #[derive(Clone, Default)]
    struct StubProvider {
        matches: Vec<MetadataMatch>,
        metadata: TitleMetadata,
        season: Option<SeasonArtwork>,
        person: Option<PersonMetadata>,
        search_err: bool,
        fetch_err: bool,
    }

    impl MetadataProvider for StubProvider {
        async fn search(
            &self,
            _query: &MetadataQuery,
        ) -> Result<Vec<MetadataMatch>, MetadataError> {
            if self.search_err {
                return Err(MetadataError::Backend("boom".into()));
            }
            Ok(self.matches.clone())
        }

        async fn fetch(&self, _id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
            if self.fetch_err {
                return Err(MetadataError::NotFound);
            }
            Ok(self.metadata.clone())
        }

        async fn fetch_season(
            &self,
            _id: &ExternalId,
            _season: u16,
        ) -> Result<SeasonArtwork, MetadataError> {
            self.season.clone().ok_or(MetadataError::NotFound)
        }

        async fn fetch_person(&self, _id: &ExternalId) -> Result<PersonMetadata, MetadataError> {
            self.person.clone().ok_or(MetadataError::NotFound)
        }
    }

    fn imdb() -> ExternalId {
        ExternalId { source: "imdb".into(), value: "tt0133093".into() }
    }

    fn tmdb() -> ExternalId {
        ExternalId { source: "tmdb".into(), value: "movie/603".into() }
    }

    fn rating(code: &str) -> ContentRating {
        ContentRating { system: "MPAA".into(), code: code.into() }
    }

    fn primary_without_rating() -> StubProvider {
        StubProvider {
            metadata: TitleMetadata {
                external_ids: vec![tmdb(), imdb()],
                ..TitleMetadata::default()
            },
            ..StubProvider::default()
        }
    }

    fn rating_provider(code: &str) -> StubProvider {
        StubProvider {
            metadata: TitleMetadata {
                content_rating: Some(rating(code)),
                ratings: vec![Rating { source: "imdb".into(), value: 8.7 }],
                ..TitleMetadata::default()
            },
            ..StubProvider::default()
        }
    }

    #[tokio::test]
    async fn search_delegates_to_primary() {
        let primary = StubProvider {
            matches: vec![MetadataMatch {
                external_id: tmdb(),
                title: "The Matrix".into(),
                year: Some(1999),
                kind: MediaKind::Movie,
            }],
            ..StubProvider::default()
        };
        let provider = CombiningProvider::new(primary, Some(rating_provider("R")));
        let query = MetadataQuery { title: "matrix".into(), year: None, kind: MediaKind::Movie };
        let matches = provider.search(&query).await.unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].title, "The Matrix");
    }

    #[tokio::test]
    async fn search_propagates_primary_error() {
        let primary = StubProvider { search_err: true, ..StubProvider::default() };
        let provider = CombiningProvider::<StubProvider, StubProvider>::new(primary, None);
        let query = MetadataQuery { title: "matrix".into(), year: None, kind: MediaKind::Movie };
        assert!(matches!(provider.search(&query).await, Err(MetadataError::Backend(_))));
    }

    #[tokio::test]
    async fn fetch_merges_rating_when_primary_has_none() {
        let provider = CombiningProvider::new(primary_without_rating(), Some(rating_provider("R")));
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert_eq!(meta.content_rating.as_ref().unwrap().code, "R");
        assert_eq!(meta.ratings.len(), 1);
        assert_eq!(meta.ratings[0].value, 8.7);
    }

    #[tokio::test]
    async fn fetch_keeps_primary_rating_when_present() {
        let primary = StubProvider {
            metadata: TitleMetadata {
                content_rating: Some(rating("PG-13")),
                external_ids: vec![imdb()],
                ..TitleMetadata::default()
            },
            ..StubProvider::default()
        };
        let provider = CombiningProvider::new(primary, Some(rating_provider("R")));
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert_eq!(meta.content_rating.as_ref().unwrap().code, "PG-13");
    }

    #[tokio::test]
    async fn fetch_preserves_primary_ratings_vec() {
        let primary = StubProvider {
            metadata: TitleMetadata {
                ratings: vec![Rating { source: "tmdb".into(), value: 9.0 }],
                external_ids: vec![imdb()],
                ..TitleMetadata::default()
            },
            ..StubProvider::default()
        };
        let provider = CombiningProvider::new(primary, Some(rating_provider("R")));
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert_eq!(meta.content_rating.as_ref().unwrap().code, "R");
        assert_eq!(meta.ratings.len(), 1);
        assert_eq!(meta.ratings[0].value, 9.0);
    }

    #[tokio::test]
    async fn fetch_without_imdb_id_keeps_none() {
        let primary = StubProvider {
            metadata: TitleMetadata { external_ids: vec![tmdb()], ..TitleMetadata::default() },
            ..StubProvider::default()
        };
        let provider = CombiningProvider::new(primary, Some(rating_provider("R")));
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert!(meta.content_rating.is_none());
    }

    #[tokio::test]
    async fn fetch_without_rating_provider_passes_through() {
        let provider =
            CombiningProvider::<StubProvider, StubProvider>::new(primary_without_rating(), None);
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert!(meta.content_rating.is_none());
    }

    #[tokio::test]
    async fn fetch_swallows_rating_provider_error() {
        let ratings = StubProvider { fetch_err: true, ..StubProvider::default() };
        let provider = CombiningProvider::new(primary_without_rating(), Some(ratings));
        let meta = provider.fetch(&tmdb()).await.unwrap();
        assert!(meta.content_rating.is_none());
    }

    #[tokio::test]
    async fn fetch_propagates_primary_error() {
        let primary = StubProvider { fetch_err: true, ..StubProvider::default() };
        let provider = CombiningProvider::new(primary, Some(rating_provider("R")));
        assert!(matches!(provider.fetch(&tmdb()).await, Err(MetadataError::NotFound)));
    }

    #[tokio::test]
    async fn fetch_season_delegates_to_primary() {
        let primary = StubProvider {
            season: Some(SeasonArtwork { number: 2, ..SeasonArtwork::default() }),
            ..StubProvider::default()
        };
        let provider = CombiningProvider::<StubProvider, StubProvider>::new(primary, None);
        let season = provider.fetch_season(&tmdb(), 2).await.unwrap();
        assert_eq!(season.number, 2);
    }

    #[tokio::test]
    async fn fetch_person_delegates_to_primary() {
        let primary = StubProvider {
            person: Some(PersonMetadata { name: "Ada".into(), ..PersonMetadata::default() }),
            ..StubProvider::default()
        };
        let provider = CombiningProvider::<StubProvider, StubProvider>::new(primary, None);
        let person = provider.fetch_person(&tmdb()).await.unwrap();
        assert_eq!(person.name, "Ada");
    }
}
