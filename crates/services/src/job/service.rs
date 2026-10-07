use std::sync::Arc;

use domain::catalog::VersionId;
use domain::common::{Page, PageRequest};
use domain::error::JobServiceError;
use domain::job::{
    Job, JobCanceller, JobId, JobKind, JobNode, JobPage, JobQuery, JobStatus, VersionJob,
};
use domain::media::SubtitleFileId;
use domain::repository::JobRepository;
use domain::service::JobService;
use domain::user::Principal;
use jiff::{SignedDuration, Timestamp};

use crate::acl;
use crate::library::{CombineJobPayload, TranscriptionJobPayload, TranslationJobPayload};

struct NoopCanceller;

impl JobCanceller for NoopCanceller {
    fn request_cancel(&self, _id: &JobId) -> bool {
        false
    }
}

const VERSION_JOB_WINDOW: SignedDuration = SignedDuration::from_hours(24);

fn elapsed_ms(job: &Job, now: Timestamp) -> Option<u64> {
    let end = match job.status {
        JobStatus::Queued => return None,
        JobStatus::Running => now,
        _ => job.finished_at?,
    };
    u64::try_from(end.duration_since(job.started_at?).as_millis()).ok()
}

fn produces(job: &Job) -> (Option<String>, Option<SubtitleFileId>) {
    match job.kind {
        JobKind::Transcription => TranscriptionJobPayload::decode(&job.payload)
            .map(|payload| (payload.source_language.clone(), Some(payload.produces())))
            .unwrap_or_default(),
        JobKind::Translation => TranslationJobPayload::decode(&job.payload)
            .ok()
            .and_then(|payload| match payload.target_languages.as_slice() {
                [language] => Some((Some(language.clone()), Some(payload.produces(language)))),
                _ => None,
            })
            .unwrap_or_default(),
        JobKind::Combine => CombineJobPayload::decode(&job.payload)
            .map(|payload| (None, Some(payload.produces())))
            .unwrap_or_default(),
        _ => (None, None),
    }
}

#[derive(Clone)]
pub struct JobServiceImpl<J> {
    jobs: J,
    canceller: Arc<dyn JobCanceller>,
    parked: Arc<[JobKind]>,
    pools: Arc<[Vec<JobKind>]>,
}

impl<J> JobServiceImpl<J> {
    pub fn new(jobs: J) -> Self {
        Self {
            jobs,
            canceller: Arc::new(NoopCanceller),
            parked: Arc::from([]),
            pools: Arc::from([]),
        }
    }

    pub fn with_pools(mut self, pools: Vec<Vec<JobKind>>) -> Self {
        self.pools = pools.into();
        self
    }

    pub fn with_canceller(mut self, canceller: Arc<dyn JobCanceller>) -> Self {
        self.canceller = canceller;
        self
    }

    pub fn with_parked(mut self, parked: Vec<JobKind>) -> Self {
        self.parked = parked.into();
        self
    }
}

impl<J> JobService for JobServiceImpl<J>
where
    J: JobRepository + Sync,
{
    async fn jobs(
        &self,
        caller: &Principal,
        query: &JobQuery,
        page: PageRequest,
    ) -> Result<JobPage, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        let items = self.jobs.list_page(query, page).await?;
        let active_total =
            self.jobs.count(&JobQuery { search: query.search.clone(), active_only: true }).await?;
        let all_total =
            self.jobs.count(&JobQuery { search: query.search.clone(), active_only: false }).await?;
        Ok(JobPage {
            page: Page {
                items,
                total: if query.active_only { active_total } else { all_total },
                offset: page.offset,
                limit: page.limit,
            },
            active_total,
            all_total,
        })
    }

    async fn job_descendants(
        &self,
        caller: &Principal,
        id: &JobId,
        page: PageRequest,
    ) -> Result<Page<JobNode>, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        Ok(self.jobs.list_descendants(id, page).await?)
    }

    async fn job(&self, caller: &Principal, id: &JobId) -> Result<Option<Job>, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        Ok(self.jobs.get(id).await?)
    }

    async fn cancel_job(&self, caller: &Principal, id: &JobId) -> Result<(), JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        if self.jobs.cancel(id, Timestamp::now()).await? {
            return Ok(());
        }
        let job = self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)?;
        match job.status {
            JobStatus::Cancelled => Ok(()),
            JobStatus::Running if job.kind.is_process_killable() => {
                self.canceller.request_cancel(id);
                Ok(())
            }
            _ => Err(JobServiceError::NotCancellable),
        }
    }

    async fn retry_job(&self, caller: &Principal, id: &JobId) -> Result<Job, JobServiceError> {
        if !acl::is_admin(caller) {
            return Err(JobServiceError::Forbidden);
        }
        let job = self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)?;
        if !self.is_retryable(&job) || !self.jobs.retry(id, Timestamp::now()).await? {
            return Err(JobServiceError::NotRetryable);
        }
        self.jobs.get(id).await?.ok_or(JobServiceError::NotFound)
    }

    fn is_retryable(&self, job: &Job) -> bool {
        matches!(job.status, JobStatus::Failed | JobStatus::Cancelled)
            && job.kind != JobKind::LibraryScan
            && !self.parked.contains(&job.kind)
    }

    async fn version_jobs(
        &self,
        caller: &Principal,
        version: &VersionId,
    ) -> Result<Vec<VersionJob>, JobServiceError> {
        if !acl::works_on_versions(caller) {
            return Err(JobServiceError::Forbidden);
        }
        let now = Timestamp::now();
        let jobs = self.jobs.list_for_version(version, now - VERSION_JOB_WINDOW).await?;
        let mut described = Vec::with_capacity(jobs.len());
        for job in jobs {
            let pool = self.pools.iter().find(|kinds| kinds.contains(&job.kind));
            let ahead = match (job.status, pool) {
                (JobStatus::Queued, Some(kinds)) => Some(self.jobs.count_ahead(&job, kinds).await?),
                _ => None,
            };
            let (language, subtitle) = produces(&job);
            let elapsed_ms = elapsed_ms(&job, now);
            described.push(VersionJob { job, ahead, elapsed_ms, language, subtitle });
        }
        Ok(described)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobPriority;
    use domain::user::{Role, UserId};
    use mocks::MockJobStore;

    const PAGE: PageRequest = PageRequest { offset: 0, limit: 50 };

    fn admin() -> Principal {
        Principal { user: UserId("admin".into()), role: Role::Admin, account_admin: true }
    }

    fn member() -> Principal {
        Principal { user: UserId("u1".into()), role: Role::User, account_admin: false }
    }

    fn job_with(id: &str, kind: JobKind, status: JobStatus) -> Job {
        let now = Timestamp::now();
        Job {
            id: JobId(id.into()),
            kind,
            status,
            priority: JobPriority::Normal,
            payload: String::new(),
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

    #[derive(Clone, Default)]
    struct RecordingCanceller {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl JobCanceller for RecordingCanceller {
        fn request_cancel(&self, id: &JobId) -> bool {
            self.calls.lock().unwrap().push(id.0.clone());
            true
        }
    }

    fn svc() -> JobServiceImpl<MockJobStore> {
        JobServiceImpl::new(MockJobStore::new())
    }

    #[tokio::test]
    async fn jobs_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("j1", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        let page = svc.jobs(&admin(), &JobQuery::default(), PAGE).await.unwrap();
        assert_eq!(page.page.items.len(), 1);
        assert_eq!(page.all_total, 1);
        assert_eq!(page.active_total, 1);
        assert!(matches!(
            svc.jobs(&member(), &JobQuery::default(), PAGE).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn jobs_filters_and_pages_with_both_tab_counts() {
        let svc = svc();
        for (id, kind, status) in [
            ("j1", JobKind::LibraryScan, JobStatus::Queued),
            ("j2", JobKind::Artwork, JobStatus::Running),
            ("j3", JobKind::Artwork, JobStatus::Succeeded),
        ] {
            svc.jobs.enqueue(job_with(id, kind, status)).await.unwrap();
        }

        let active = svc.jobs(&admin(), &JobQuery::active(true), PAGE).await.unwrap();
        assert_eq!(active.page.total, 2);
        assert_eq!(active.active_total, 2);
        assert_eq!(active.all_total, 3);

        let artwork = svc
            .jobs(&admin(), &JobQuery { search: Some("artwork".into()), active_only: false }, PAGE)
            .await
            .unwrap();
        assert_eq!(artwork.page.total, 2);
        assert_eq!(artwork.active_total, 1);
        assert_eq!(artwork.all_total, 2);

        let second = svc
            .jobs(&admin(), &JobQuery::default(), PageRequest { offset: 2, limit: 2 })
            .await
            .unwrap();
        assert_eq!(second.page.items.len(), 1);
        assert_eq!(second.page.total, 3);
    }

    #[tokio::test]
    async fn job_descendants_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("root", JobKind::LibraryScan, JobStatus::Running)).await.unwrap();
        let mut child = job_with("child", JobKind::Artwork, JobStatus::Queued);
        child.parent_id = Some(JobId("root".into()));
        svc.jobs.enqueue(child).await.unwrap();

        let tree = svc.job_descendants(&admin(), &JobId("root".into()), PAGE).await.unwrap();
        assert_eq!(tree.total, 1);
        assert_eq!(tree.items[0].job.id.0, "child");
        assert_eq!(tree.items[0].depth, 0);
        assert!(matches!(
            svc.job_descendants(&member(), &JobId("root".into()), PAGE).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn job_admin_only() {
        let svc = svc();
        svc.jobs.enqueue(job_with("j1", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        assert_eq!(
            svc.job(&admin(), &JobId("j1".into())).await.unwrap().unwrap().id,
            JobId("j1".into())
        );
        assert!(svc.job(&admin(), &JobId("nope".into())).await.unwrap().is_none());
        assert!(matches!(
            svc.job(&member(), &JobId("j1".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn cancel_job_requires_admin() {
        let svc = svc();
        assert!(matches!(
            svc.cancel_job(&member(), &JobId("x".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
    }

    #[tokio::test]
    async fn cancel_job_queued_marks_cancelled() {
        let svc = svc();
        svc.jobs.enqueue(job_with("q", JobKind::LibraryScan, JobStatus::Queued)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("q".into())).await.unwrap();
        let stored = svc.jobs.get(&JobId("q".into())).await.unwrap().unwrap();
        assert_eq!(stored.status, JobStatus::Cancelled);
        assert!(stored.finished_at.is_some());
    }

    #[tokio::test]
    async fn cancel_job_missing_is_not_found() {
        let svc = svc();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("nope".into())).await.unwrap_err(),
            JobServiceError::NotFound
        ));
    }

    #[tokio::test]
    async fn cancel_job_running_killable_signals_canceller() {
        let jobs = MockJobStore::new();
        jobs.enqueue(job_with("f", JobKind::Fetch, JobStatus::Running)).await.unwrap();
        let canceller = RecordingCanceller::default();
        let svc = JobServiceImpl::new(jobs).with_canceller(Arc::new(canceller.clone()));
        svc.cancel_job(&admin(), &JobId("f".into())).await.unwrap();
        let calls = canceller.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "f");
    }

    #[tokio::test]
    async fn cancel_job_running_killable_default_canceller_is_ok() {
        let svc = svc();
        svc.jobs.enqueue(job_with("t", JobKind::Trickplay, JobStatus::Running)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("t".into())).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_job_running_non_killable_conflicts() {
        let svc = svc();
        svc.jobs.enqueue(job_with("s", JobKind::LibraryScan, JobStatus::Running)).await.unwrap();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("s".into())).await.unwrap_err(),
            JobServiceError::NotCancellable
        ));
    }

    #[tokio::test]
    async fn cancel_job_already_cancelled_is_idempotent() {
        let svc = svc();
        svc.jobs.enqueue(job_with("c", JobKind::Fetch, JobStatus::Cancelled)).await.unwrap();
        svc.cancel_job(&admin(), &JobId("c".into())).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_job_terminal_conflicts() {
        let svc = svc();
        svc.jobs.enqueue(job_with("d", JobKind::Fetch, JobStatus::Succeeded)).await.unwrap();
        assert!(matches!(
            svc.cancel_job(&admin(), &JobId("d".into())).await.unwrap_err(),
            JobServiceError::NotCancellable
        ));
    }

    #[test]
    fn only_a_failed_or_cancelled_job_of_a_live_kind_is_retryable() {
        let svc = svc().with_parked(vec![JobKind::Transcription]);
        let cases = [
            (JobKind::Subtitles, JobStatus::Failed, true),
            (JobKind::Subtitles, JobStatus::Cancelled, true),
            (JobKind::Subtitles, JobStatus::Queued, false),
            (JobKind::Subtitles, JobStatus::Running, false),
            (JobKind::Subtitles, JobStatus::Succeeded, false),
            (JobKind::Transcription, JobStatus::Failed, false),
            (JobKind::LibraryScan, JobStatus::Failed, false),
            (JobKind::LibraryScan, JobStatus::Cancelled, false),
        ];
        for (kind, status, expected) in cases {
            assert_eq!(
                svc.is_retryable(&job_with("j", kind, status)),
                expected,
                "{kind:?} {status:?}"
            );
        }
    }

    #[tokio::test]
    async fn retry_job_requeues_a_failed_job() {
        let svc = svc();
        let mut failed = job_with("f", JobKind::Subtitles, JobStatus::Failed);
        failed.attempts = 3;
        failed.last_error = Some("http status 406 Not Acceptable".into());
        failed.finished_at = Some(Timestamp::now());
        svc.jobs.enqueue(failed).await.unwrap();

        let job = svc.retry_job(&admin(), &JobId("f".into())).await.unwrap();

        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.attempts, 0);
        assert_eq!(job.last_error, None);
        assert_eq!(job.finished_at, None);
    }

    #[tokio::test]
    async fn retry_job_requires_admin() {
        let svc = svc();
        svc.jobs.enqueue(job_with("f", JobKind::Subtitles, JobStatus::Failed)).await.unwrap();
        assert!(matches!(
            svc.retry_job(&member(), &JobId("f".into())).await.unwrap_err(),
            JobServiceError::Forbidden
        ));
        assert_eq!(
            svc.jobs.get(&JobId("f".into())).await.unwrap().unwrap().status,
            JobStatus::Failed
        );
    }

    #[tokio::test]
    async fn retry_job_missing_is_not_found() {
        assert!(matches!(
            svc().retry_job(&admin(), &JobId("nope".into())).await.unwrap_err(),
            JobServiceError::NotFound
        ));
    }

    #[tokio::test]
    async fn retry_job_refuses_a_parked_kind_and_a_finished_job() {
        let svc = svc().with_parked(vec![JobKind::Transcription]);
        svc.jobs.enqueue(job_with("t", JobKind::Transcription, JobStatus::Failed)).await.unwrap();
        svc.jobs.enqueue(job_with("s", JobKind::Subtitles, JobStatus::Succeeded)).await.unwrap();
        for id in ["t", "s"] {
            assert!(matches!(
                svc.retry_job(&admin(), &JobId(id.into())).await.unwrap_err(),
                JobServiceError::NotRetryable
            ));
        }
        assert_eq!(
            svc.jobs.get(&JobId("t".into())).await.unwrap().unwrap().status,
            JobStatus::Failed,
            "a parked kind would sit queued forever, so it stays failed"
        );
    }

    #[tokio::test]
    async fn a_failed_library_scan_is_scanned_again_rather_than_retried() {
        let svc = svc();
        svc.jobs.enqueue(job_with("l", JobKind::LibraryScan, JobStatus::Failed)).await.unwrap();

        assert!(matches!(
            svc.retry_job(&admin(), &JobId("l".into())).await.unwrap_err(),
            JobServiceError::NotRetryable
        ));
        assert_eq!(
            svc.jobs.get(&JobId("l".into())).await.unwrap().unwrap().status,
            JobStatus::Failed,
            "a re-queued scan would skip the library's scan slot"
        );
    }

    fn version_work(id: &str, mut job: Job, status: JobStatus, created_secs_ago: i64) -> Job {
        let created = Timestamp::now() - SignedDuration::from_secs(created_secs_ago);
        job.id = JobId(id.into());
        job.status = status;
        job.created_at = created;
        job.updated_at = created;
        job
    }

    fn by_id<'a>(jobs: &'a [VersionJob], id: &str) -> &'a VersionJob {
        jobs.iter().find(|entry| entry.job.id.0 == id).expect("listed")
    }

    #[tokio::test]
    async fn a_versions_jobs_say_what_they_produce_and_how_long_they_took() {
        use crate::library::{transcription_job, translation_job, translation_job_with_source};
        let v1 = VersionId("v1".into());
        let svc = svc().with_pools(vec![vec![JobKind::Transcription, JobKind::Translation]]);
        let mut running = version_work(
            "translate-zh",
            translation_job_with_source(&v1, "sf-en", "zh"),
            JobStatus::Running,
            120,
        );
        running.started_at = Some(Timestamp::now() - SignedDuration::from_secs(90));
        let mut finished = version_work(
            "translate-many",
            translation_job(&v1, &["fr".to_owned(), "de".to_owned()]).unwrap(),
            JobStatus::Succeeded,
            300,
        );
        let started = Timestamp::now() - SignedDuration::from_secs(60);
        finished.started_at = Some(started);
        finished.finished_at = Some(started + SignedDuration::from_secs(10));
        for job in [
            version_work(
                "transcribe",
                transcription_job(&v1, "/m/a.mkv", Some("en".into()), None, true),
                JobStatus::Queued,
                10,
            ),
            running,
            finished,
        ] {
            svc.jobs.enqueue(job).await.unwrap();
        }

        let jobs = svc.version_jobs(&admin(), &v1).await.unwrap();

        let ids: Vec<&str> = jobs.iter().map(|entry| entry.job.id.0.as_str()).collect();
        assert_eq!(ids, ["transcribe", "translate-zh", "translate-many"]);
        let transcribe = by_id(&jobs, "transcribe");
        assert_eq!(transcribe.ahead, Some(0));
        assert_eq!(transcribe.elapsed_ms, None);
        assert_eq!(transcribe.language.as_deref(), Some("en"));
        assert_eq!(transcribe.subtitle, Some(SubtitleFileId("generated:v1".into())));
        let translating = by_id(&jobs, "translate-zh");
        assert_eq!(translating.ahead, None);
        assert!(translating.elapsed_ms.is_some_and(|ms| ms >= 90_000));
        assert_eq!(translating.language.as_deref(), Some("zh"));
        assert_eq!(translating.subtitle, Some(SubtitleFileId("machine:v1:zh".into())));
        let many = by_id(&jobs, "translate-many");
        assert_eq!(many.elapsed_ms, Some(10_000));
        assert_eq!((many.language.clone(), many.subtitle.clone()), (None, None));
    }

    #[tokio::test]
    async fn combine_names_its_subtitle_and_a_job_never_started_has_no_elapsed_time() {
        use crate::library::{combine_job, upscale_job};
        let v1 = VersionId("v1".into());
        let svc = svc().with_pools(vec![vec![JobKind::Upscale, JobKind::Combine]]);
        let mut failed =
            version_work("combine", combine_job(&v1, "sf-en", "sf-fr"), JobStatus::Failed, 30);
        failed.finished_at = Some(Timestamp::now());
        svc.jobs.enqueue(failed).await.unwrap();
        svc.jobs
            .enqueue(version_work("upscale", upscale_job(&v1, 2160), JobStatus::Queued, 20))
            .await
            .unwrap();
        svc.jobs
            .enqueue(version_work(
                "earlier-upscale",
                upscale_job(&VersionId("v2".into()), 2160),
                JobStatus::Queued,
                60,
            ))
            .await
            .unwrap();

        let jobs = svc.version_jobs(&admin(), &v1).await.unwrap();

        let combine = by_id(&jobs, "combine");
        assert_eq!(combine.subtitle, Some(SubtitleFileId("combined:v1:sf-en:sf-fr".into())));
        assert_eq!(combine.language, None);
        assert_eq!(combine.elapsed_ms, None);
        assert_eq!(combine.ahead, None);
        let upscale = by_id(&jobs, "upscale");
        assert_eq!(upscale.ahead, Some(1), "another version's upscale is in the same pool");
        assert_eq!((upscale.language.clone(), upscale.subtitle.clone()), (None, None));
    }

    #[tokio::test]
    async fn a_switched_off_kind_has_nothing_ahead_because_no_pool_runs_it() {
        use crate::library::transcription_job;
        let v1 = VersionId("v1".into());
        let svc = svc().with_pools(vec![vec![JobKind::Upscale]]);
        svc.jobs
            .enqueue(version_work(
                "transcribe",
                transcription_job(&v1, "/m/a.mkv", None, None, true),
                JobStatus::Queued,
                5,
            ))
            .await
            .unwrap();

        let jobs = svc.version_jobs(&admin(), &v1).await.unwrap();

        assert_eq!(jobs[0].ahead, None);
        assert_eq!(jobs[0].language, None);
    }

    #[tokio::test]
    async fn a_versions_jobs_are_for_admins_and_their_linked_devices() {
        let v1 = VersionId("v1".into());
        let device = |account_admin| Principal {
            user: UserId("admin".into()),
            role: Role::Player,
            account_admin,
        };

        assert!(svc().version_jobs(&device(true), &v1).await.unwrap().is_empty());
        for refused in [member(), device(false)] {
            assert!(matches!(
                svc().version_jobs(&refused, &v1).await.unwrap_err(),
                JobServiceError::Forbidden
            ));
        }
    }
}
