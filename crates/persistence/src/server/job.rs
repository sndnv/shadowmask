use std::path::Path;

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::job::{
    Job, JobId, JobKind, JobNode, JobPriority, JobQuery, JobStatus, RECLAIM_DEAD_LETTER_ERROR,
    ReclaimOutcome,
};
use domain::repository::JobRepository;
use jiff::Timestamp;
use sqlx::migrate::Migrator;
use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteRow};
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
        JobKind::Fetch => "fetch",
        JobKind::ScheduledScan => "scheduled_scan",
        JobKind::Retention => "retention",
        JobKind::OrphanSweep => "orphan_sweep",
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
        "fetch" => Ok(JobKind::Fetch),
        "scheduled_scan" => Ok(JobKind::ScheduledScan),
        "retention" => Ok(JobKind::Retention),
        "orphan_sweep" => Ok(JobKind::OrphanSweep),
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

const MAX_TREE_DEPTH: u32 = 64;

const SEARCH_PREDICATE: &str = "(LOWER(id) LIKE ? ESCAPE '\\' \
     OR REPLACE(kind, '_', ' ') LIKE ? ESCAPE '\\' \
     OR status LIKE ? ESCAPE '\\')";

fn where_clause(query: &JobQuery) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if query.active_only {
        parts.push("status IN ('queued', 'running')");
    }
    if query.needle().is_some() {
        parts.push(SEARCH_PREDICATE);
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", parts.join(" AND "))
    }
}

fn like_pattern(needle: &str) -> String {
    let escaped = needle.replace('\\', "\\\\").replace('%', "\\%");
    format!("%{escaped}%")
}

fn bind_search<'q>(
    mut query: Query<'q, Sqlite, SqliteArguments>,
    filter: &JobQuery,
) -> Query<'q, Sqlite, SqliteArguments> {
    if let Some(needle) = filter.needle() {
        let pattern = like_pattern(&needle);
        for _ in 0..3 {
            query = query.bind(pattern.clone());
        }
    }
    query
}

fn descendants_cte() -> String {
    format!(
        "WITH RECURSIVE tree(id, depth, path) AS ( \
             SELECT id, 0, printf('%020d-%s', created_at, id) FROM jobs WHERE parent_id = ? \
             UNION ALL \
             SELECT j.id, t.depth + 1, t.path || '/' || printf('%020d-%s', j.created_at, j.id) \
             FROM jobs j JOIN tree t ON j.parent_id = t.id WHERE t.depth < {MAX_TREE_DEPTH} \
         )"
    )
}

fn ready_ids_sql(kinds: usize) -> String {
    let placeholders = vec!["?"; kinds].join(", ");
    format!(
        "SELECT id FROM jobs \
         WHERE status = ? AND available_at <= ? AND kind IN ({placeholders}) \
         ORDER BY priority DESC, created_at ASC, id ASC \
         LIMIT ?"
    )
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
        let sql = format!(
            "UPDATE jobs SET status = ?, started_at = ? \
             WHERE id IN ({}) RETURNING *",
            ready_ids_sql(kinds.len())
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

    async fn reclaim_running(
        &self,
        now: Timestamp,
        max_attempts: u32,
    ) -> Result<ReclaimOutcome, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "reclaim_running");
        let dead_lettered = sqlx::query(
            "UPDATE jobs SET status = ?, attempts = attempts + 1, last_error = ?, \
             finished_at = ?, updated_at = ?, started_at = NULL \
             WHERE status = ? AND attempts + 1 >= ?",
        )
        .bind(status_to_str(JobStatus::Failed))
        .bind(RECLAIM_DEAD_LETTER_ERROR)
        .bind(to_millis(now))
        .bind(to_millis(now))
        .bind(status_to_str(JobStatus::Running))
        .bind(i64::from(max_attempts))
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected() as usize;
        let requeued = sqlx::query(
            "UPDATE jobs SET status = ?, attempts = attempts + 1, available_at = ?, \
             updated_at = ?, started_at = NULL WHERE status = ?",
        )
        .bind(status_to_str(JobStatus::Queued))
        .bind(to_millis(now))
        .bind(to_millis(now))
        .bind(status_to_str(JobStatus::Running))
        .execute(&self.pool)
        .await
        .map_err(backend)?
        .rows_affected() as usize;
        Ok(ReclaimOutcome {
            requeued,
            dead_lettered,
        })
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

    async fn list_page(
        &self,
        query: &JobQuery,
        page: PageRequest,
    ) -> Result<Vec<Job>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "list_page");
        let sql = format!(
            "SELECT * FROM jobs{} ORDER BY created_at DESC, id ASC LIMIT ? OFFSET ?",
            where_clause(query)
        );
        let rows = bind_search(sqlx::query(AssertSqlSafe(sql)), query)
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_job).collect()
    }

    async fn count(&self, query: &JobQuery) -> Result<u64, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "count");
        let sql = format!("SELECT COUNT(*) AS n FROM jobs{}", where_clause(query));
        let row = bind_search(sqlx::query(AssertSqlSafe(sql)), query)
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        Ok(column::<i64>(&row, "n")? as u64)
    }

    async fn list_descendants(
        &self,
        root: &JobId,
        page: PageRequest,
    ) -> Result<Page<JobNode>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "list_descendants");
        let cte = descendants_cte();
        let count_row = sqlx::query(AssertSqlSafe(format!(
            "{cte} SELECT COUNT(*) AS n FROM tree"
        )))
        .bind(root.0.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(AssertSqlSafe(format!(
            "{cte} SELECT j.*, t.depth AS depth FROM tree t JOIN jobs j ON j.id = t.id \
             ORDER BY t.path LIMIT ? OFFSET ?"
        )))
        .bind(root.0.as_str())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let items = rows
            .iter()
            .map(|row| {
                Ok(JobNode {
                    job: row_to_job(row)?,
                    depth: column::<i64>(row, "depth")? as u32,
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn cancel(&self, id: &JobId, now: Timestamp) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "cancel");
        let result = sqlx::query(
            "UPDATE jobs SET status = ?, finished_at = ?, updated_at = ? WHERE id = ? AND status = ?",
        )
        .bind(status_to_str(JobStatus::Cancelled))
        .bind(to_millis(now))
        .bind(to_millis(now))
        .bind(id.0.as_str())
        .bind(status_to_str(JobStatus::Queued))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_finished_before(
        &self,
        cutoff: Timestamp,
    ) -> Result<Vec<JobId>, RepositoryError> {
        let _op = DbOpGuard::new("jobs", "delete_finished_before");
        const SELECT: &str = "SELECT id FROM jobs \
             WHERE status IN ('succeeded', 'failed', 'cancelled') \
             AND finished_at IS NOT NULL AND finished_at < ?";
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let rows = sqlx::query(SELECT)
            .bind(to_millis(cutoff))
            .fetch_all(&mut *tx)
            .await
            .map_err(backend)?;
        let mut removed = Vec::with_capacity(rows.len());
        for row in &rows {
            removed.push(JobId(column::<String>(row, "id")?));
        }
        sqlx::query(
            "DELETE FROM jobs \
             WHERE status IN ('succeeded', 'failed', 'cancelled') \
             AND finished_at IS NOT NULL AND finished_at < ?",
        )
        .bind(to_millis(cutoff))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::SignedDuration;

    #[tokio::test]
    async fn claiming_ready_jobs_walks_the_ready_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
            .await
            .unwrap();
        let rows = sqlx::query(AssertSqlSafe(format!(
            "EXPLAIN QUERY PLAN {}",
            ready_ids_sql(2)
        )))
        .bind("queued")
        .bind(0_i64)
        .bind("ingest")
        .bind("metadata")
        .bind(4_i64)
        .fetch_all(&repo.pool)
        .await
        .unwrap();
        let plan = rows
            .iter()
            .map(|row| column::<String>(row, "detail").unwrap())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            plan.contains("USING INDEX jobs_ready_idx") && !plan.contains("TEMP B-TREE"),
            "the worker claims jobs every five seconds, so this must never sort the queue: {plan}"
        );
    }

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
            JobKind::Fetch,
            JobKind::ScheduledScan,
            JobKind::Retention,
            JobKind::OrphanSweep,
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
        assert!(
            repo.count(&JobQuery::default()).await.is_err(),
            "a filtered count fails the same way"
        );
        assert!(
            repo.list_page(
                &JobQuery::default(),
                PageRequest {
                    offset: 0,
                    limit: 1
                }
            )
            .await
            .is_err()
        );
        assert!(
            repo.list_descendants(
                &JobId("root".into()),
                PageRequest {
                    offset: 0,
                    limit: 1
                }
            )
            .await
            .is_err()
        );
        assert!(
            repo.delete_finished_before(Timestamp::UNIX_EPOCH)
                .await
                .is_err()
        );
        let id = JobId("j1".into());
        assert!(repo.enqueue(contracts::fixture::admin_job()).await.is_err());
        assert!(
            repo.claim_ready(Timestamp::UNIX_EPOCH, 1, vec![JobKind::LibraryScan])
                .await
                .is_err(),
            "an empty kind list short circuits before the pool, so it proves nothing"
        );
        assert!(
            repo.reclaim_running(Timestamp::UNIX_EPOCH, 3)
                .await
                .is_err()
        );
        assert!(repo.update(contracts::fixture::admin_job()).await.is_err());
        assert!(repo.get(&id).await.is_err());
        assert!(repo.cancel(&id, Timestamp::UNIX_EPOCH).await.is_err());
    }

    #[test]
    fn a_needle_cannot_smuggle_in_a_wildcard() {
        assert_eq!(like_pattern("100%"), "%100\\%%");
        assert_eq!(like_pattern("a\\b"), "%a\\\\b%");
        assert_eq!(like_pattern("plain"), "%plain%");
    }

    #[test]
    fn the_where_clause_only_carries_the_parts_it_needs() {
        assert!(where_clause(&JobQuery::default()).is_empty());
        assert_eq!(
            where_clause(&JobQuery::active(true)),
            " WHERE status IN ('queued', 'running')"
        );
        let searched = JobQuery {
            search: Some("scan".into()),
            active_only: false,
        };
        assert!(where_clause(&searched).starts_with(" WHERE (LOWER(id) LIKE"));
        let both = JobQuery {
            search: Some("scan".into()),
            active_only: true,
        };
        assert!(where_clause(&both).contains(" AND "));
    }

    #[tokio::test]
    async fn a_wildcard_in_the_needle_matches_literally() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
            .await
            .unwrap();
        let now = Timestamp::UNIX_EPOCH;
        for id in ["100%-done", "anything"] {
            repo.enqueue(Job {
                id: JobId(id.into()),
                kind: JobKind::Artwork,
                status: JobStatus::Queued,
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
            })
            .await
            .unwrap();
        }

        let wildcard = JobQuery {
            search: Some("%".into()),
            active_only: false,
        };
        assert_eq!(
            repo.count(&wildcard).await.unwrap(),
            1,
            "a bare % must not match every row"
        );
    }

    #[tokio::test]
    async fn retention_removes_only_terminal_jobs_older_than_the_cutoff() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
            .await
            .unwrap();
        let old = Timestamp::UNIX_EPOCH;
        let cutoff = old.checked_add(SignedDuration::from_hours(24)).unwrap();
        let recent = cutoff.checked_add(SignedDuration::from_hours(1)).unwrap();
        let seed = [
            ("old-succeeded", JobStatus::Succeeded, Some(old)),
            ("old-failed", JobStatus::Failed, Some(old)),
            ("old-cancelled", JobStatus::Cancelled, Some(old)),
            ("recent-succeeded", JobStatus::Succeeded, Some(recent)),
            ("succeeded-unfinished", JobStatus::Succeeded, None),
            ("queued", JobStatus::Queued, None),
            ("running", JobStatus::Running, None),
        ];
        for (id, status, finished_at) in seed {
            repo.enqueue(Job {
                id: JobId(id.into()),
                kind: JobKind::Retention,
                status,
                priority: JobPriority::Normal,
                payload: String::new(),
                attempts: 0,
                progress: 0.0,
                available_at: old,
                last_error: None,
                created_at: old,
                updated_at: old,
                started_at: None,
                finished_at,
                parent_id: None,
            })
            .await
            .unwrap();
        }

        let mut removed: Vec<String> = repo
            .delete_finished_before(cutoff)
            .await
            .unwrap()
            .into_iter()
            .map(|id| id.0)
            .collect();
        removed.sort();
        assert_eq!(removed, ["old-cancelled", "old-failed", "old-succeeded"]);

        let mut left: Vec<String> = repo
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|job| job.id.0)
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "queued",
                "recent-succeeded",
                "running",
                "succeeded-unfinished"
            ],
            "a job still running, still queued, or never finished outlives the cutoff"
        );
    }

    #[tokio::test]
    async fn retention_reports_nothing_when_every_job_is_young() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteJobRepo::connect(&dir.path().join("jobs.db"))
            .await
            .unwrap();
        assert!(
            repo.delete_finished_before(Timestamp::UNIX_EPOCH)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
