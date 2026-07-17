use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
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

    repo.enqueue(job("high", JobPriority::High, now, now))
        .await
        .unwrap();
    repo.enqueue(job("normal-old", JobPriority::Normal, now, older))
        .await
        .unwrap();
    repo.enqueue(job("normal-new", JobPriority::Normal, now, now))
        .await
        .unwrap();
    repo.enqueue(job("low", JobPriority::Low, now, now))
        .await
        .unwrap();
    repo.enqueue(job("future", JobPriority::High, future, now))
        .await
        .unwrap();
    let mut running = job("running", JobPriority::High, now, now);
    running.status = JobStatus::Running;
    repo.enqueue(running).await.unwrap();

    let stored = repo
        .get(&JobId("normal-old".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.created_at, older);
    assert_eq!(stored.available_at, now);
    assert_eq!(stored.priority, JobPriority::Normal);

    let listed = repo.list().await.unwrap();
    assert_eq!(
        ids(&listed),
        [
            "normal-old",
            "future",
            "high",
            "low",
            "normal-new",
            "running"
        ]
    );

    let first = repo.claim_ready(now, 1).await.unwrap();
    assert_eq!(ids(&first), ["high"]);
    assert!(first.iter().all(|j| j.status == JobStatus::Running));
    assert!(first.iter().all(|j| j.started_at == Some(now)));

    let rest = repo.claim_ready(now, 10).await.unwrap();
    assert_eq!(ids(&rest), ["normal-old", "normal-new", "low"]);
    assert!(rest.iter().all(|j| j.status == JobStatus::Running));
    assert!(rest.iter().all(|j| j.started_at == Some(now)));

    assert!(repo.claim_ready(now, 10).await.unwrap().is_empty());

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

    let reclaimed = repo.reclaim_running(now).await.unwrap();
    assert_eq!(reclaimed, 4);
    let reclaimed_old = repo
        .get(&JobId("normal-old".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reclaimed_old.status, JobStatus::Queued);
    assert!(reclaimed_old.started_at.is_none());
    assert_eq!(
        repo.get(&JobId("high".into()))
            .await
            .unwrap()
            .unwrap()
            .status,
        JobStatus::Succeeded
    );
    let reclaimed_jobs = repo.claim_ready(now, 10).await.unwrap();
    assert_eq!(
        ids(&reclaimed_jobs),
        ["running", "normal-old", "normal-new", "low"]
    );
}
