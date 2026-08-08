use domain::job::Job;
use services::library::{MetadataJobPayload, MetadataRefresher, PersonRefresher};

use crate::error::JobError;
use crate::job_handler::JobHandler;

pub struct MetadataJobHandler<E> {
    refresher: E,
}

impl<E> MetadataJobHandler<E> {
    pub fn new(refresher: E) -> Self {
        Self { refresher }
    }
}

impl<E> JobHandler for MetadataJobHandler<E>
where
    E: MetadataRefresher + PersonRefresher + Send + Sync,
{
    async fn handle(&self, job: &Job) -> Result<(), JobError> {
        let payload = MetadataJobPayload::decode(&job.payload)
            .map_err(|e| JobError::Permanent(format!("invalid metadata payload: {e}")))?;
        match payload {
            MetadataJobPayload::Title { title, external_id } => {
                tracing::info!("refreshing metadata for [{title:?}]");
                self.refresher
                    .refresh(&title, external_id.as_ref(), Some(&job.id))
                    .await
                    .map_err(|e| JobError::Retryable(e.to_string()))?;
            }
            MetadataJobPayload::People { ids, force } => {
                tracing::info!("refreshing metadata for {} person(s)", ids.len());
                for id in &ids {
                    self.refresher
                        .refresh_person(id, force, Some(&job.id))
                        .await
                        .map_err(|e| JobError::Retryable(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{Movie, MovieId, TitleRef};
    use domain::job::{JobId, JobKind, JobPriority, JobStatus};
    use domain::metadata::{ExternalId, MediaKind, MetadataMatch, PersonId};
    use domain::repository::CatalogRepository;
    use jiff::Timestamp;
    use services::library::Enricher;
    use services::mock::{MockCatalogRepo, MockJobStore, MockLibraryRepo, MockMetadataProvider};

    fn enricher(
        catalog: MockCatalogRepo,
        provider: Option<MockMetadataProvider>,
    ) -> Enricher<MockCatalogRepo, MockMetadataProvider, MockJobStore, MockLibraryRepo> {
        Enricher::new(
            catalog,
            provider,
            MockJobStore::new(),
            MockLibraryRepo::new(),
        )
    }

    fn job(payload: String) -> Job {
        let now = Timestamp::UNIX_EPOCH;
        Job {
            id: JobId("job-1".into()),
            kind: JobKind::Metadata,
            status: JobStatus::Running,
            priority: JobPriority::Normal,
            payload,
            attempts: 0,
            progress: 0.0,
            available_at: now,
            last_error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            finished_at: None,
            parent_id: None,
        }
    }

    #[tokio::test]
    async fn refreshes_movie_metadata() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(Movie {
            id: MovieId("m1".into()),
            title: "Old".into(),
            year: Some(1999),
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        let provider = MockMetadataProvider::with_matches(vec![MetadataMatch {
            external_id: ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            },
            title: "New".into(),
            year: Some(1999),
            kind: MediaKind::Movie,
        }]);
        let handler = MetadataJobHandler::new(enricher(catalog.clone(), Some(provider)));
        let payload = MetadataJobPayload::Title {
            title: TitleRef::Movie(MovieId("m1".into())),
            external_id: None,
        }
        .encode()
        .unwrap();

        handler.handle(&job(payload)).await.unwrap();

        assert!(
            catalog
                .get_movie(&MovieId("m1".into()))
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn refreshes_people_payload() {
        let handler = MetadataJobHandler::new(enricher(MockCatalogRepo::new(), None));
        let payload = MetadataJobPayload::People {
            ids: vec![PersonId("p1".into())],
            force: true,
        }
        .encode()
        .unwrap();
        handler.handle(&job(payload)).await.unwrap();
    }

    #[tokio::test]
    async fn invalid_payload_is_permanent() {
        let handler = MetadataJobHandler::new(enricher(MockCatalogRepo::new(), None));
        assert!(matches!(
            handler.handle(&job("garbage".into())).await.unwrap_err(),
            JobError::Permanent(_)
        ));
    }

    #[tokio::test]
    async fn repository_error_is_retryable() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let handler = MetadataJobHandler::new(enricher(catalog, None::<MockMetadataProvider>));
        let payload = MetadataJobPayload::Title {
            title: TitleRef::Movie(MovieId("m1".into())),
            external_id: None,
        }
        .encode()
        .unwrap();
        assert!(matches!(
            handler.handle(&job(payload)).await.unwrap_err(),
            JobError::Retryable(_)
        ));
    }
}
