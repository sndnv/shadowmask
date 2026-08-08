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
        let query = MetadataQuery {
            title: title.to_owned(),
            year,
            kind,
        };
        let matches = match provider.search(&query).await {
            Ok(matches) => matches,
            Err(err) => {
                debug!("metadata search failed for {title}: {err}");
                return None;
            }
        };
        let first = matches.into_iter().next()?;
        match provider.fetch(&first.external_id).await {
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
