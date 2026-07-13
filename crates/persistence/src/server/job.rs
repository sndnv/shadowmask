use std::path::Path;

use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Sqlite, SqlitePool};

use crate::pool::{backend, column, from_millis, open, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/jobs");

#[derive(Clone)]
pub struct SqliteJobRepo {
    pool: SqlitePool,
}

impl SqliteJobRepo {
    pub async fn connect(path: &Path) -> Result<Self, RepositoryError> {
        Ok(Self {
            pool: open(path, &MIGRATOR).await?,
        })
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

fn kind_to_str(kind: JobKind) -> &'static str {
    match kind {
        JobKind::LibraryScan => "library_scan",
        JobKind::Metadata => "metadata",
        JobKind::Artwork => "artwork",
        JobKind::Subtitles => "subtitles",
        JobKind::Trickplay => "trickplay",
        JobKind::Fingerprint => "fingerprint",
        JobKind::Dedup => "dedup",
        JobKind::CacheEviction => "cache_eviction",
        JobKind::SearchReindex => "search_reindex",
        JobKind::Ingest => "ingest",
    }
}

fn kind_from_str(value: &str) -> Result<JobKind, RepositoryError> {
    match value {
        "library_scan" => Ok(JobKind::LibraryScan),
        "metadata" => Ok(JobKind::Metadata),
        "artwork" => Ok(JobKind::Artwork),
        "subtitles" => Ok(JobKind::Subtitles),
        "trickplay" => Ok(JobKind::Trickplay),
        "fingerprint" => Ok(JobKind::Fingerprint),
        "dedup" => Ok(JobKind::Dedup),
        "cache_eviction" => Ok(JobKind::CacheEviction),
        "search_reindex" => Ok(JobKind::SearchReindex),
        "ingest" => Ok(JobKind::Ingest),
        other => Err(backend(format!("unknown job kind: {other}"))),
    }
}

fn status_to_str(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

fn status_from_str(value: &str) -> Result<JobStatus, RepositoryError> {
    match value {
        "queued" => Ok(JobStatus::Queued),
        "running" => Ok(JobStatus::Running),
        "succeeded" => Ok(JobStatus::Succeeded),
        "failed" => Ok(JobStatus::Failed),
        "cancelled" => Ok(JobStatus::Cancelled),
        other => Err(backend(format!("unknown job status: {other}"))),
    }
}

fn priority_to_int(priority: JobPriority) -> i64 {
    match priority {
        JobPriority::Low => 0,
        JobPriority::Normal => 1,
        JobPriority::High => 2,
    }
}

fn priority_from_int(value: i64) -> Result<JobPriority, RepositoryError> {
    match value {
        0 => Ok(JobPriority::Low),
        1 => Ok(JobPriority::Normal),
        2 => Ok(JobPriority::High),
        other => Err(backend(format!("unknown job priority: {other}"))),
    }
}

fn row_to_job(row: &SqliteRow) -> Result<Job, RepositoryError> {
    Ok(Job {
        id: JobId(column(row, "id")?),
        kind: kind_from_str(&column::<String>(row, "kind")?)?,
        status: status_from_str(&column::<String>(row, "status")?)?,
        priority: priority_from_int(column(row, "priority")?)?,
        payload: column(row, "payload")?,
        attempts: column::<i64>(row, "attempts")? as u32,
        progress: column::<f64>(row, "progress")? as f32,
        available_at: from_millis(column(row, "available_at")?)?,
        last_error: column(row, "last_error")?,
        created_at: from_millis(column(row, "created_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
    })
}

async fn upsert<'e, E>(executor: E, job: &Job) -> Result<(), RepositoryError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT OR REPLACE INTO jobs \
         (id, kind, status, priority, payload, attempts, progress, available_at, last_error, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(job.id.0.as_str())
    .bind(kind_to_str(job.kind))
    .bind(status_to_str(job.status))
    .bind(priority_to_int(job.priority))
    .bind(job.payload.as_str())
    .bind(job.attempts as i64)
    .bind(job.progress as f64)
    .bind(to_millis(job.available_at))
    .bind(job.last_error.as_deref())
    .bind(to_millis(job.created_at))
    .bind(to_millis(job.updated_at))
    .execute(executor)
    .await
    .map_err(backend)?;
    Ok(())
}

impl JobRepository for SqliteJobRepo {
    async fn enqueue(&self, job: Job) -> Result<(), RepositoryError> {
        upsert(&self.pool, &job).await
    }

    async fn claim_ready(&self, now: Timestamp, limit: usize) -> Result<Vec<Job>, RepositoryError> {
        let rows = sqlx::query(
            "UPDATE jobs SET status = ? \
             WHERE id IN ( \
                 SELECT id FROM jobs \
                 WHERE status = ? AND available_at <= ? \
                 ORDER BY priority DESC, created_at ASC, id ASC \
                 LIMIT ? \
             ) \
             RETURNING *",
        )
        .bind(status_to_str(JobStatus::Running))
        .bind(status_to_str(JobStatus::Queued))
        .bind(to_millis(now))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut jobs = rows.iter().map(row_to_job).collect::<Result<Vec<_>, _>>()?;
        jobs.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.created_at.cmp(&b.created_at))
                .then(a.id.cmp(&b.id))
        });
        Ok(jobs)
    }

    async fn update(&self, job: Job) -> Result<(), RepositoryError> {
        upsert(&self.pool, &job).await
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_job).transpose()
    }

    async fn list(&self) -> Result<Vec<Job>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM jobs ORDER BY created_at ASC, id ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_job).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_round_trips_and_rejects_unknown() {
        for kind in [
            JobKind::LibraryScan,
            JobKind::Metadata,
            JobKind::Artwork,
            JobKind::Subtitles,
            JobKind::Trickplay,
            JobKind::Fingerprint,
            JobKind::Dedup,
            JobKind::CacheEviction,
            JobKind::SearchReindex,
            JobKind::Ingest,
        ] {
            assert_eq!(kind_from_str(kind_to_str(kind)).unwrap(), kind);
        }
        for status in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Succeeded,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ] {
            assert_eq!(status_from_str(status_to_str(status)).unwrap(), status);
        }
        for priority in [JobPriority::Low, JobPriority::Normal, JobPriority::High] {
            assert_eq!(
                priority_from_int(priority_to_int(priority)).unwrap(),
                priority
            );
        }
        assert!(kind_from_str("nope").is_err());
        assert!(status_from_str("nope").is_err());
        assert!(priority_from_int(9).is_err());
    }

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
            .await
            .unwrap();
        repo.pool.close().await;
        assert!(repo.list().await.is_err());
    }
}
