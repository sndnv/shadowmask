use std::path::Path;

use domain::error::RepositoryError;
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::repository::JobRepository;
use jiff::Timestamp;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{AssertSqlSafe, Executor, Sqlite, SqlitePool};

use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

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

    pub async fn ping(&self) -> Result<(), RepositoryError> {
        ping(&self.pool).await
    }

    pub async fn close(&self) {
        checkpoint(&self.pool).await;
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
        JobKind::Relink => "relink",
        JobKind::Transcription => "transcription",
        JobKind::Translation => "translation",
        JobKind::Upscale => "upscale",
        JobKind::Combine => "combine",
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
        "relink" => Ok(JobKind::Relink),
        "transcription" => Ok(JobKind::Transcription),
        "translation" => Ok(JobKind::Translation),
        "upscale" => Ok(JobKind::Upscale),
        "combine" => Ok(JobKind::Combine),
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
        started_at: column::<Option<i64>>(row, "started_at")?
            .map(from_millis)
            .transpose()?,
        finished_at: column::<Option<i64>>(row, "finished_at")?
            .map(from_millis)
            .transpose()?,
        parent_id: column::<Option<String>>(row, "parent_id")?.map(JobId),
    })
}

async fn upsert<'e, E>(executor: E, job: &Job) -> Result<(), RepositoryError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT OR REPLACE INTO jobs \
         (id, kind, status, priority, payload, attempts, progress, available_at, last_error, created_at, updated_at, started_at, finished_at, parent_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(job.started_at.map(to_millis))
    .bind(job.finished_at.map(to_millis))
    .bind(job.parent_id.as_ref().map(|id| id.0.as_str()))
    .execute(executor)
    .await
    .map_err(backend)?;
    Ok(())
}

impl JobRepository for SqliteJobRepo {
    async fn enqueue(&self, job: Job) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("jobs", "enqueue");
        upsert(&self.pool, &job).await
    }

    async fn claim_ready(
        &self,
        now: Timestamp,
        limit: usize,
        kinds: Vec<JobKind>,
    ) -> Result<Vec<Job>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "claim_ready");
        if kinds.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; kinds.len()].join(", ");
        let sql = format!(
            "UPDATE jobs SET status = ?, started_at = ? \
             WHERE id IN ( \
                 SELECT id FROM jobs \
                 WHERE status = ? AND available_at <= ? AND kind IN ({placeholders}) \
                 ORDER BY priority DESC, created_at ASC, id ASC \
                 LIMIT ? \
             ) \
             RETURNING *"
        );
        let mut query = sqlx::query(AssertSqlSafe(sql))
            .bind(status_to_str(JobStatus::Running))
            .bind(to_millis(now))
            .bind(status_to_str(JobStatus::Queued))
            .bind(to_millis(now));
        for kind in &kinds {
            query = query.bind(kind_to_str(*kind));
        }
        let rows = query
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

    async fn reclaim_running(&self, now: Timestamp) -> Result<usize, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "reclaim_running");
        let result = sqlx::query(
            "UPDATE jobs SET status = ?, available_at = ?, started_at = NULL WHERE status = ?",
        )
        .bind(status_to_str(JobStatus::Queued))
        .bind(to_millis(now))
        .bind(status_to_str(JobStatus::Running))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(result.rows_affected() as usize)
    }

    async fn update(&self, job: Job) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("jobs", "update");
        upsert(&self.pool, &job).await
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "get");
        let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_job).transpose()
    }

    async fn list(&self) -> Result<Vec<Job>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "list");
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
            JobKind::Relink,
            JobKind::Transcription,
            JobKind::Translation,
            JobKind::Upscale,
            JobKind::Combine,
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
