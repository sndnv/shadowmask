use domain::error::MetadataError;
use domain::metadata::{ExternalId, MetadataMatch, MetadataProvider, MetadataQuery, TitleMetadata};

#[derive(Clone, Default)]
pub struct MockMetadataProvider {
    matches: Vec<MetadataMatch>,
    fail: bool,
}

impl MockMetadataProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_matches(matches: Vec<MetadataMatch>) -> Self {
        Self {
            matches,
            fail: false,
        }
    }

    pub fn failing() -> Self {
        Self {
            matches: Vec::new(),
            fail: true,
        }
    }
}

impl MetadataProvider for MockMetadataProvider {
    async fn search(&self, _query: &MetadataQuery) -> Result<Vec<MetadataMatch>, MetadataError> {
        if self.fail {
            return Err(MetadataError::Backend("mock provider failure".to_owned()));
        }
        Ok(self.matches.clone())
    }

    async fn fetch(&self, _id: &ExternalId) -> Result<TitleMetadata, MetadataError> {
        if self.fail {
            return Err(MetadataError::NotFound);
        }
        Ok(TitleMetadata::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::metadata::MediaKind;

    fn query() -> MetadataQuery {
        MetadataQuery {
            title: "x".into(),
            year: None,
            kind: MediaKind::Movie,
        }
    }

    fn external() -> ExternalId {
        ExternalId {
            source: "tmdb".into(),
            value: "movie/1".into(),
        }
    }

    #[tokio::test]
    async fn empty_by_default_and_returns_seeded_matches() {
        assert!(
            MockMetadataProvider::new()
                .search(&query())
                .await
                .unwrap()
                .is_empty()
        );
        let provider = MockMetadataProvider::with_matches(vec![MetadataMatch {
            external_id: external(),
            title: "X".into(),
            year: Some(2001),
            kind: MediaKind::Movie,
        }]);
        assert_eq!(provider.search(&query()).await.unwrap().len(), 1);
        assert!(provider.fetch(&external()).await.unwrap().title.is_empty());
    }

    #[tokio::test]
    async fn failing_errors_on_search_and_fetch() {
        let provider = MockMetadataProvider::failing();
        assert!(matches!(
            provider.search(&query()).await.unwrap_err(),
            MetadataError::Backend(_)
        ));
        assert!(matches!(
            provider.fetch(&external()).await.unwrap_err(),
            MetadataError::NotFound
        ));
    }
}
