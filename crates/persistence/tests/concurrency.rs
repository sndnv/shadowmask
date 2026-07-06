use std::collections::BTreeSet;

use domain::catalog::VersionId;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::playback::PlaybackProgress;
use domain::repository::{JobRepository, ProgressRepository};
use domain::user::UserId;
use jiff::Timestamp;
use persistence::server::SqliteJobRepo;
use persistence::user::SqliteProgressRepo;

fn at(second: i64) -> Timestamp {
    Timestamp::from_second(second).unwrap()
}

fn job(id: &str, now: Timestamp) -> Job {
    Job {
        id: JobId(id.into()),
        kind: JobKind::LibraryScan,
        status: JobStatus::Queued,
        priority: JobPriority::Normal,
        payload: String::new(),
        attempts: 0,
        progress: 0.0,
        available_at: now,
        last_error: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_claim_ready_never_double_claims() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
        .await
        .unwrap();
    let now = at(1_700_000_000);
    for i in 0..12 {
        repo.enqueue(job(&format!("j{i}"), now)).await.unwrap();
    }

    let mut handles = Vec::new();
    for _ in 0..12 {
        let repo = repo.clone();
        handles.push(tokio::spawn(async move {
            repo.claim_ready(now, 1).await.unwrap()
        }));
    }

    let mut claimed = Vec::new();
    for handle in handles {
        for job in handle.await.unwrap() {
            assert_eq!(job.status, JobStatus::Running);
            claimed.push(job.id.0);
        }
    }

    let unique: BTreeSet<_> = claimed.iter().cloned().collect();
    assert_eq!(claimed.len(), unique.len(), "a job was claimed twice");
    assert_eq!(claimed.len(), 12, "every queued job claimed exactly once");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn racing_heartbeats_serialize_to_last_writer() {
    let dir = tempfile::tempdir().unwrap();
    let repo = SqliteProgressRepo::new(dir.path());
    let user = UserId("u1".into());
    let version = VersionId("v1".into());
    repo.ensure_migrated(&user).await.unwrap();

    let mut handles = Vec::new();
    for i in 1..=16u64 {
        let repo = repo.clone();
        let user = user.clone();
        let version = version.clone();
        handles.push(tokio::spawn(async move {
            repo.upsert(PlaybackProgress {
                user,
                version,
                position_ms: i * 1000,
                updated_at: at(1_700_000_000 + i as i64),
            })
            .await
        }));
    }
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let stored = repo.get(&user, &version).await.unwrap().unwrap();
    let written: Vec<u64> = (1..=16).map(|i| i * 1000).collect();
    assert!(
        written.contains(&stored.position_ms),
        "final position {} was not one of the racing writes",
        stored.position_ms
    );
}
