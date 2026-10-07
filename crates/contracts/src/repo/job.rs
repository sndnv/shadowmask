use domain::catalog::VersionId;
use domain::common::PageRequest;
use domain::job::{
    Job, JobId, JobKind, JobPriority, JobQuery, JobStatus, RECLAIM_DEAD_LETTER_ERROR,
};
use domain::repository::JobRepository;
use jiff::Timestamp;

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn job(id: &str, priority: JobPriority, available_at: Timestamp, created_at: Timestamp) -> Job {
    Job {
        id: JobId(id.into()),
        kind: JobKind::LibraryScan,
        status: JobStatus::Queued,
        priority,
        payload: String::new(),
        attempts: 0,
        progress: 0.0,
        available_at,
        last_error: None,
        created_at,
        updated_at: created_at,
        started_at: None,
        finished_at: None,
        parent_id: None,
    }
}

fn ids(jobs: &[Job]) -> Vec<&str> {
    jobs.iter().map(|j| j.id.0.as_str()).collect()
}

pub async fn job_repository_contract<R: JobRepository>(repo: R) {
    let now = at(1_700_000_000);
    let older = at(1_700_000_000 - 10);
    let future = at(1_700_000_000 + 60);

    assert!(repo.get(&JobId("missing".into())).await.unwrap().is_none());
    assert!(repo.list().await.unwrap().is_empty());

    repo.enqueue(job("high", JobPriority::High, now, now)).await.unwrap();
    repo.enqueue(job("normal-old", JobPriority::Normal, now, older)).await.unwrap();
    repo.enqueue(job("normal-new", JobPriority::Normal, now, now)).await.unwrap();
    repo.enqueue(job("low", JobPriority::Low, now, now)).await.unwrap();
    repo.enqueue(job("future", JobPriority::High, future, now)).await.unwrap();
    let mut running = job("running", JobPriority::High, now, now);
    running.status = JobStatus::Running;
    repo.enqueue(running).await.unwrap();

    let stored = repo.get(&JobId("normal-old".into())).await.unwrap().unwrap();
    assert_eq!(stored.created_at, older);
    assert_eq!(stored.available_at, now);
    assert_eq!(stored.priority, JobPriority::Normal);

    let listed = repo.list().await.unwrap();
    assert_eq!(ids(&listed), ["normal-old", "future", "high", "low", "normal-new", "running"]);

    let first = repo.claim_ready(now, 1, vec![JobKind::LibraryScan]).await.unwrap();
    assert_eq!(ids(&first), ["high"]);
    assert!(first.iter().all(|j| j.status == JobStatus::Running));
    assert!(first.iter().all(|j| j.started_at == Some(now)));

    let rest = repo.claim_ready(now, 10, vec![JobKind::LibraryScan]).await.unwrap();
    assert_eq!(ids(&rest), ["normal-old", "normal-new", "low"]);
    assert!(rest.iter().all(|j| j.status == JobStatus::Running));
    assert!(rest.iter().all(|j| j.started_at == Some(now)));

    assert!(repo.claim_ready(now, 10, vec![JobKind::LibraryScan]).await.unwrap().is_empty());

    let mut done = repo.get(&JobId("high".into())).await.unwrap().unwrap();
    done.status = JobStatus::Succeeded;
    done.progress = 1.0;
    done.finished_at = Some(future);
    repo.update(done).await.unwrap();
    let reloaded = repo.get(&JobId("high".into())).await.unwrap().unwrap();
    assert_eq!(reloaded.status, JobStatus::Succeeded);
    assert_eq!(reloaded.progress, 1.0);
    assert_eq!(reloaded.started_at, Some(now));
    assert_eq!(reloaded.finished_at, Some(future));

    let mut exhausted = job("exhausted", JobPriority::Normal, now, now);
    exhausted.status = JobStatus::Running;
    exhausted.attempts = 2;
    repo.enqueue(exhausted).await.unwrap();

    let reclaimed = repo.reclaim_running(now, 3).await.unwrap();
    assert_eq!(reclaimed.requeued, 4);
    assert_eq!(reclaimed.dead_lettered, 1);
    assert_eq!(reclaimed.total(), 5);
    let dead_lettered = repo.get(&JobId("exhausted".into())).await.unwrap().unwrap();
    assert_eq!(dead_lettered.status, JobStatus::Failed);
    assert_eq!(dead_lettered.attempts, 3);
    assert_eq!(dead_lettered.finished_at, Some(now));
    assert_eq!(dead_lettered.last_error.as_deref(), Some(RECLAIM_DEAD_LETTER_ERROR));
    let reclaimed_old = repo.get(&JobId("normal-old".into())).await.unwrap().unwrap();
    assert_eq!(reclaimed_old.status, JobStatus::Queued);
    assert_eq!(reclaimed_old.attempts, 1);
    assert!(reclaimed_old.started_at.is_none());
    assert_eq!(
        repo.get(&JobId("high".into())).await.unwrap().unwrap().status,
        JobStatus::Succeeded
    );
    let reclaimed_jobs = repo.claim_ready(now, 10, vec![JobKind::LibraryScan]).await.unwrap();
    assert_eq!(ids(&reclaimed_jobs), ["running", "normal-old", "normal-new", "low"]);

    repo.enqueue(job("scan-2", JobPriority::Normal, now, now)).await.unwrap();
    let mut transcribe = job("transcribe-1", JobPriority::High, now, now);
    transcribe.kind = JobKind::Transcription;
    repo.enqueue(transcribe).await.unwrap();
    let scans = repo.claim_ready(now, 10, vec![JobKind::LibraryScan]).await.unwrap();
    assert_eq!(ids(&scans), ["scan-2"]);
    let transcribes = repo.claim_ready(now, 10, vec![JobKind::Transcription]).await.unwrap();
    assert_eq!(ids(&transcribes), ["transcribe-1"]);

    let mut child = job("child-1", JobPriority::Low, now, now);
    child.parent_id = Some(JobId("scan-2".into()));
    repo.enqueue(child).await.unwrap();
    let reloaded_child = repo.get(&JobId("child-1".into())).await.unwrap().unwrap();
    assert_eq!(reloaded_child.parent_id, Some(JobId("scan-2".into())));
    assert!(repo.get(&JobId("scan-2".into())).await.unwrap().unwrap().parent_id.is_none());

    repo.enqueue(job("cancel-me", JobPriority::Normal, now, now)).await.unwrap();
    assert!(repo.cancel(&JobId("cancel-me".into()), future).await.unwrap());
    let cancelled = repo.get(&JobId("cancel-me".into())).await.unwrap().unwrap();
    assert_eq!(cancelled.status, JobStatus::Cancelled);
    assert_eq!(cancelled.finished_at, Some(future));
    assert_eq!(cancelled.updated_at, future);
    assert!(!repo.cancel(&JobId("cancel-me".into()), future).await.unwrap());

    let mut running_2 = job("running-2", JobPriority::Normal, now, now);
    running_2.status = JobStatus::Running;
    repo.enqueue(running_2).await.unwrap();
    assert!(!repo.cancel(&JobId("running-2".into()), future).await.unwrap());
    assert_eq!(
        repo.get(&JobId("running-2".into())).await.unwrap().unwrap().status,
        JobStatus::Running
    );

    assert!(!repo.cancel(&JobId("missing".into()), future).await.unwrap());

    paged_and_filtered_reads(&repo).await;
    retry_requeues_only_failed_or_cancelled_jobs(&repo).await;
    a_versions_jobs_are_read_from_their_payload(&repo).await;
    queued_work_ahead_follows_the_claim_order(&repo).await;
}

fn work(id: &str, kind: JobKind, status: JobStatus, payload: &str, created_at: Timestamp) -> Job {
    let mut entry = job(id, JobPriority::Normal, created_at, created_at);
    entry.kind = kind;
    entry.status = status;
    entry.payload = payload.into();
    entry
}

async fn a_versions_jobs_are_read_from_their_payload<R: JobRepository>(repo: &R) {
    let now = at(1_900_000_000);
    let since = at(1_900_000_000 - 86_400);
    let v1 = r#"{"version_id":"v1","source_path":"/m/a.mkv"}"#;
    let mut recent =
        work("v-recent", JobKind::Translation, JobStatus::Succeeded, v1, at(1_899_999_980));
    recent.finished_at = Some(at(1_900_000_000 - 86_399));
    let mut old = work("v-old", JobKind::Combine, JobStatus::Failed, v1, at(1_899_999_970));
    old.finished_at = Some(at(1_900_000_000 - 86_401));
    for entry in [
        work("v-queued", JobKind::Transcription, JobStatus::Queued, v1, now),
        work("v-running", JobKind::Upscale, JobStatus::Running, v1, at(1_899_999_990)),
        recent,
        old,
        work(
            "v-relink",
            JobKind::Relink,
            JobStatus::Queued,
            r#"{"version":"v1"}"#,
            at(1_899_999_960),
        ),
        work("v-other", JobKind::Trickplay, JobStatus::Queued, r#"{"version_id":"v2"}"#, now),
        work("v-scan", JobKind::LibraryScan, JobStatus::Queued, "v1", now),
        work("v-metadata", JobKind::Metadata, JobStatus::Queued, v1, now),
        work("v-plain", JobKind::Subtitles, JobStatus::Queued, "v1", now),
    ] {
        repo.enqueue(entry).await.unwrap();
    }

    let found = repo.list_for_version(&VersionId("v1".into()), since).await.unwrap();

    assert_eq!(ids(&found), ["v-queued", "v-running", "v-recent", "v-relink"], "newest first");
    assert!(repo.list_for_version(&VersionId("v9".into()), since).await.unwrap().is_empty());
}

async fn queued_work_ahead_follows_the_claim_order<R: JobRepository>(repo: &R) {
    let now = at(1_950_000_000);
    let queued = |id: &str, kind: JobKind, priority: JobPriority, created_at: Timestamp| {
        let mut entry = work(id, kind, JobStatus::Queued, "{}", created_at);
        entry.priority = priority;
        entry
    };
    let target = queued("a-target", JobKind::Upscale, JobPriority::Normal, now);
    let mut running = queued("a-running", JobKind::Upscale, JobPriority::High, now);
    running.status = JobStatus::Running;
    for entry in [
        target.clone(),
        queued("a-high-later", JobKind::Upscale, JobPriority::High, at(1_950_000_005)),
        queued("a-normal-earlier", JobKind::Combine, JobPriority::Normal, at(1_949_999_995)),
        queued("a-0", JobKind::Upscale, JobPriority::Normal, now),
        queued("a-normal-later", JobKind::Upscale, JobPriority::Normal, at(1_950_000_001)),
        queued("a-low-earlier", JobKind::Upscale, JobPriority::Low, at(1_949_999_900)),
        queued("a-other-pool", JobKind::Trickplay, JobPriority::High, at(1_949_999_900)),
        running,
    ] {
        repo.enqueue(entry).await.unwrap();
    }

    let pool = [JobKind::Upscale, JobKind::Combine];
    assert_eq!(repo.count_ahead(&target, &pool).await.unwrap(), 3);
    assert_eq!(repo.count_ahead(&target, &[]).await.unwrap(), 0);
}

async fn retry_requeues_only_failed_or_cancelled_jobs<R: JobRepository>(repo: &R) {
    let now = at(1_700_000_000);
    let later = at(1_700_000_000 + 600);
    for status in [JobStatus::Failed, JobStatus::Cancelled] {
        let id = JobId(format!("retry-{}", status.slug()));
        let mut finished = job(&id.0, JobPriority::High, now, now);
        finished.status = status;
        finished.attempts = 3;
        finished.progress = 0.5;
        finished.last_error = Some("http status 406 Not Acceptable".into());
        finished.started_at = Some(now);
        finished.finished_at = Some(now);
        repo.enqueue(finished).await.unwrap();

        assert!(repo.retry(&id, later).await.unwrap(), "{status:?}");
        let requeued = repo.get(&id).await.unwrap().unwrap();
        assert_eq!(requeued.status, JobStatus::Queued);
        assert_eq!(requeued.attempts, 0);
        assert_eq!(requeued.progress, 0.0);
        assert_eq!(requeued.available_at, later);
        assert_eq!(requeued.updated_at, later);
        assert_eq!(requeued.last_error, None);
        assert_eq!(requeued.started_at, None);
        assert_eq!(requeued.finished_at, None);
        assert_eq!(requeued.priority, JobPriority::High);
        assert_eq!(requeued.created_at, now);
    }

    for status in [JobStatus::Queued, JobStatus::Running, JobStatus::Succeeded] {
        let id = JobId(format!("retry-{}", status.slug()));
        let mut entry = job(&id.0, JobPriority::Normal, now, now);
        entry.status = status;
        entry.attempts = 1;
        repo.enqueue(entry).await.unwrap();

        assert!(!repo.retry(&id, later).await.unwrap(), "{status:?}");
        let untouched = repo.get(&id).await.unwrap().unwrap();
        assert_eq!(untouched.status, status);
        assert_eq!(untouched.attempts, 1);
        assert_eq!(untouched.updated_at, now);
    }

    assert!(!repo.retry(&JobId("missing".into()), later).await.unwrap());
}

async fn paged_and_filtered_reads<R: JobRepository>(repo: &R) {
    let now = at(1_700_000_000);
    let mut tree = vec![
        ("zz-root", JobKind::LibraryScan, JobStatus::Running, 0, None),
        ("zz-kid-a", JobKind::Artwork, JobStatus::Queued, 100, Some("zz-root")),
        ("zz-grandkid", JobKind::Metadata, JobStatus::Succeeded, 200, Some("zz-kid-a")),
        ("zz-kid-b", JobKind::Artwork, JobStatus::Failed, 300, Some("zz-root")),
        ("zz-evict", JobKind::CacheEviction, JobStatus::Succeeded, 400, None),
    ];
    for (id, kind, status, offset_secs, parent) in tree.drain(..) {
        let created = at(1_800_000_000 + offset_secs);
        let mut entry = job(id, JobPriority::Normal, now, created);
        entry.kind = kind;
        entry.status = status;
        entry.parent_id = parent.map(|p| JobId(p.into()));
        repo.enqueue(entry).await.unwrap();
    }

    let mine = JobQuery { search: Some("zz-".into()), active_only: false };
    let page = repo.list_page(&mine, PageRequest { offset: 0, limit: 50 }).await.unwrap();
    assert_eq!(
        ids(&page),
        ["zz-evict", "zz-kid-b", "zz-grandkid", "zz-kid-a", "zz-root"],
        "newest first"
    );
    assert_eq!(repo.count(&mine).await.unwrap(), 5);

    let second = repo.list_page(&mine, PageRequest { offset: 1, limit: 2 }).await.unwrap();
    assert_eq!(ids(&second), ["zz-kid-b", "zz-grandkid"]);

    let active = JobQuery { search: Some("zz-".into()), active_only: true };
    assert_eq!(repo.count(&active).await.unwrap(), 2);
    let active_page = repo.list_page(&active, PageRequest { offset: 0, limit: 50 }).await.unwrap();
    assert_eq!(ids(&active_page), ["zz-kid-a", "zz-root"]);

    let by_kind = JobQuery { search: Some("cache eviction".into()), active_only: false };
    assert_eq!(repo.count(&by_kind).await.unwrap(), 1);
    let underscored = JobQuery { search: Some("cache_eviction".into()), active_only: false };
    assert_eq!(repo.count(&underscored).await.unwrap(), 1);

    let no_match = JobQuery { search: Some("no-such-job".into()), active_only: false };
    assert_eq!(repo.count(&no_match).await.unwrap(), 0);
    assert!(
        repo.list_page(&no_match, PageRequest { offset: 0, limit: 50 }).await.unwrap().is_empty()
    );

    let subtree = repo
        .list_descendants(&JobId("zz-root".into()), PageRequest { offset: 0, limit: 50 })
        .await
        .unwrap();
    assert_eq!(subtree.total, 3);
    let shape: Vec<(&str, u32)> =
        subtree.items.iter().map(|node| (node.job.id.0.as_str(), node.depth)).collect();
    assert_eq!(
        shape,
        [("zz-kid-a", 0), ("zz-grandkid", 1), ("zz-kid-b", 0)],
        "depth first, a branch stays with its parent"
    );

    let mid = repo
        .list_descendants(&JobId("zz-kid-a".into()), PageRequest { offset: 0, limit: 50 })
        .await
        .unwrap();
    assert_eq!(ids(&mid.items.iter().map(|n| n.job.clone()).collect::<Vec<_>>()), ["zz-grandkid"]);
    assert_eq!(mid.items[0].depth, 0);

    let paged_tree = repo
        .list_descendants(&JobId("zz-root".into()), PageRequest { offset: 1, limit: 1 })
        .await
        .unwrap();
    assert_eq!(paged_tree.total, 3);
    assert_eq!(paged_tree.items.len(), 1);
    assert_eq!(paged_tree.items[0].job.id.0, "zz-grandkid");
    assert_eq!(paged_tree.items[0].depth, 1, "depth survives a page boundary");

    let leaf = repo
        .list_descendants(&JobId("zz-grandkid".into()), PageRequest { offset: 0, limit: 50 })
        .await
        .unwrap();
    assert!(leaf.items.is_empty());
    assert_eq!(leaf.total, 0);

    let missing = repo
        .list_descendants(&JobId("no-such-job".into()), PageRequest { offset: 0, limit: 50 })
        .await
        .unwrap();
    assert!(missing.items.is_empty());
}
