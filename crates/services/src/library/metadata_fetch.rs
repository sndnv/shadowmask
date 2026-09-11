use domain::metadata::{
    ExternalId, MediaKind, MetadataProvider, MetadataQuery, PersonMetadata, SeasonArtwork,
    TitleMetadata,
};
use tracing::debug;

pub(super) struct MetadataFetcher<M> {
    provider: Option<M>,
}

impl<M> MetadataFetcher<M> {
    pub(super) fn new(provider: Option<M>) -> Self {
        Self { provider }
    }
}

impl<M> MetadataFetcher<M>
where
    M: MetadataProvider + Send + Sync,
{
    pub(super) async fn fetch_metadata(
        &self,
        kind: MediaKind,
        title: &str,
        year: Option<u16>,
    ) -> Option<TitleMetadata> {
        let provider = self.provider.as_ref()?;
        let query = MetadataQuery { title: title.to_owned(), year, kind };
        let matches = match provider.search(&query).await {
            Ok(matches) => matches,
            Err(err) => {
                debug!("metadata search failed for {title}: {err}");
                return None;
            }
        };
        let candidate = match year {
            Some(year) => matches.into_iter().find(|found| found.year == Some(year)),
            None => matches.into_iter().next(),
        };
        let Some(candidate) = candidate else {
            debug!("no metadata candidate for {title} matched year {year:?}");
            return None;
        };
        match provider.fetch(&candidate.external_id).await {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                debug!("metadata fetch failed for {title}: {err}");
                None
            }
        }
    }

    pub(super) async fn fetch_by_id(&self, id: &ExternalId) -> Option<TitleMetadata> {
        let provider = self.provider.as_ref()?;
        match provider.fetch(id).await {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                debug!("metadata fetch failed for {}: {err}", id.value);
                None
            }
        }
    }

    pub(super) async fn fetch_season(&self, id: &ExternalId, season: u16) -> Option<SeasonArtwork> {
        let provider = self.provider.as_ref()?;
        match provider.fetch_season(id, season).await {
            Ok(season) => Some(season),
            Err(err) => {
                debug!("season metadata fetch failed for {}: {err}", id.value);
                None
            }
        }
    }

    pub(super) async fn fetch_person(&self, id: &ExternalId) -> Option<PersonMetadata> {
        let provider = self.provider.as_ref()?;
        match provider.fetch_person(id).await {
            Ok(person) => Some(person),
            Err(err) => {
                debug!("person metadata fetch failed for {}: {err}", id.value);
                None
            }
        }
    }

    pub(super) async fn fetch_refresh(
        &self,
        kind: MediaKind,
        title: &str,
        year: Option<u16>,
        external_id: Option<&ExternalId>,
    ) -> Option<TitleMetadata> {
        match external_id {
            Some(id) => self.fetch_by_id(id).await,
            None => self.fetch_metadata(kind, title, year).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::metadata::MetadataMatch;
    use mocks::MockMetadataProvider;

    fn candidate(value: &str, title: &str, year: Option<u16>) -> MetadataMatch {
        MetadataMatch {
            external_id: ExternalId { source: "tmdb".into(), value: value.to_owned() },
            title: title.to_owned(),
            year,
            kind: MediaKind::Movie,
        }
    }

    fn fetcher(matches: Vec<MetadataMatch>) -> MetadataFetcher<MockMetadataProvider> {
        MetadataFetcher::new(Some(MockMetadataProvider::with_matches(matches)))
    }

    #[tokio::test]
    async fn a_popular_result_from_another_year_is_refused() {
        let fetcher = fetcher(vec![candidate("movie/1", "Spring Breakers", Some(2012))]);
        assert!(fetcher.fetch_metadata(MediaKind::Movie, "Spring", Some(2019)).await.is_none());
    }

    #[tokio::test]
    async fn the_first_candidate_agreeing_on_the_year_wins_over_a_higher_ranked_one() {
        let fetcher = fetcher(vec![
            candidate("movie/1", "Spring Breakers", Some(2012)),
            candidate("movie/2", "Spring", Some(2019)),
        ]);
        assert!(fetcher.fetch_metadata(MediaKind::Movie, "Spring", Some(2019)).await.is_some());
    }

    #[tokio::test]
    async fn a_candidate_with_no_year_is_refused_when_we_parsed_one() {
        let fetcher = fetcher(vec![candidate("movie/1", "Spring", None)]);
        assert!(fetcher.fetch_metadata(MediaKind::Movie, "Spring", Some(2019)).await.is_none());
    }

    #[tokio::test]
    async fn without_a_parsed_year_the_first_candidate_is_still_taken() {
        let fetcher = fetcher(vec![candidate("movie/1", "Spring Breakers", Some(2012))]);
        assert!(fetcher.fetch_metadata(MediaKind::Movie, "Spring", None).await.is_some());
    }

    #[tokio::test]
    async fn an_admin_reidentify_overrides_the_year_filter() {
        let fetcher = fetcher(vec![candidate("movie/1", "Spring Breakers", Some(2012))]);
        let chosen = ExternalId { source: "tmdb".into(), value: "movie/1".into() };
        assert!(
            fetcher
                .fetch_refresh(MediaKind::Movie, "Spring", Some(2019), Some(&chosen))
                .await
                .is_some()
        );
        assert!(
            fetcher.fetch_refresh(MediaKind::Movie, "Spring", Some(2019), None).await.is_none()
        );
    }
}
