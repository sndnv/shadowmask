use domain::catalog::VersionId;
use domain::error::RepositoryError;
use domain::job::{JobId, TranslationTrigger};
use domain::repository::JobRepository;

use super::translation_job;

pub struct TranslationEnqueuer<J> {
    jobs: J,
    target_languages: Vec<String>,
}

impl<J> TranslationEnqueuer<J> {
    pub fn new(jobs: J, target_languages: Vec<String>) -> Self {
        Self { jobs, target_languages }
    }
}

impl<J> TranslationTrigger for TranslationEnqueuer<J>
where
    J: JobRepository + Send + Sync,
{
    async fn trigger(
        &self,
        version_id: &VersionId,
        parent: Option<&JobId>,
    ) -> Result<(), RepositoryError> {
        match translation_job(version_id, &self.target_languages) {
            Some(mut job) => {
                job.parent_id = parent.cloned();
                self.jobs.enqueue(job).await
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::job::{JobId, JobKind};

    use super::*;
    use mocks::MockJobStore;

    #[tokio::test]
    async fn enqueues_a_translation_job_when_languages_configured() {
        let jobs = MockJobStore::new();
        let enqueuer = TranslationEnqueuer::new(jobs.clone(), vec!["fr".into()]);

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0].kind, JobKind::Translation);
    }

    #[tokio::test]
    async fn links_the_child_to_its_parent_job() {
        let jobs = MockJobStore::new();
        let enqueuer = TranslationEnqueuer::new(jobs.clone(), vec!["fr".into()]);

        enqueuer.trigger(&VersionId("v1".into()), Some(&JobId("parent".into()))).await.unwrap();

        let enqueued = jobs.list().await.unwrap();
        assert_eq!(enqueued[0].parent_id, Some(JobId("parent".into())));
    }

    #[tokio::test]
    async fn enqueues_nothing_when_no_languages() {
        let jobs = MockJobStore::new();
        let enqueuer = TranslationEnqueuer::new(jobs.clone(), Vec::new());

        enqueuer.trigger(&VersionId("v1".into()), None).await.unwrap();

        assert!(jobs.list().await.unwrap().is_empty());
    }
}
