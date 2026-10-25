use std::path::Path;

use domain::common::{Page, PageRequest};
use domain::diagnostics::{PlayerId, Report, ReportId, ReportSummary};
use domain::error::RepositoryError;
use domain::repository::ReportRepository;
use domain::user::UserId;
use jiff::Timestamp;
use sqlx::migrate::Migrator;
use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteRow};
use sqlx::{AssertSqlSafe, Sqlite, SqlitePool};

use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, like_pattern, open, ping, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/reports");

const SEARCH_PREDICATE: &str = " AND (LOWER(player_id) LIKE ? ESCAPE '\\' \
     OR LOWER(user_id) LIKE ? ESCAPE '\\' \
     OR LOWER(app) LIKE ? ESCAPE '\\' \
     OR LOWER(platform) LIKE ? ESCAPE '\\' \
     OR LOWER(app_version) LIKE ? ESCAPE '\\')";

const SEARCHED_COLUMNS: usize = 5;

fn where_clause(needle: Option<&str>) -> String {
    let search = if needle.is_some() { SEARCH_PREDICATE } else { "" };
    format!(" WHERE created_at >= ?{search}")
}

fn list_sql(needle: Option<&str>) -> String {
    format!(
        "SELECT id, created_at, user_id, player_id, app, platform, app_version, note \
         FROM reports{} ORDER BY created_at DESC, id ASC LIMIT ? OFFSET ?",
        where_clause(needle)
    )
}

fn bind_filter<'q>(
    mut query: Query<'q, Sqlite, SqliteArguments>,
    since: Timestamp,
    needle: Option<&str>,
) -> Query<'q, Sqlite, SqliteArguments> {
    query = query.bind(to_millis(since));
    if let Some(needle) = needle {
        let pattern = like_pattern(&needle.to_lowercase());
        for _ in 0..SEARCHED_COLUMNS {
            query = query.bind(pattern.clone());
        }
    }
    query
}

#[derive(Clone)]
pub struct SqliteReportRepo {
    pool: SqlitePool,
}

impl SqliteReportRepo {
    pub async fn connect(path: &Path) -> Result<Self, RepositoryError> {
        Ok(Self { pool: open(path, &MIGRATOR).await? })
    }

    pub async fn ping(&self) -> Result<(), RepositoryError> {
        ping(&self.pool).await
    }

    pub async fn close(&self) {
        checkpoint(&self.pool).await;
        self.pool.close().await;
    }
}

fn row_to_summary(row: &SqliteRow) -> Result<ReportSummary, RepositoryError> {
    Ok(ReportSummary {
        id: ReportId(column(row, "id")?),
        created_at: from_millis(column(row, "created_at")?)?,
        user: UserId(column(row, "user_id")?),
        player: PlayerId(column(row, "player_id")?),
        app: column(row, "app")?,
        platform: column(row, "platform")?,
        app_version: column(row, "app_version")?,
        note: column(row, "note")?,
    })
}

impl ReportRepository for SqliteReportRepo {
    async fn insert(&self, report: Report) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("reports", "insert");
        let summary = &report.summary;
        sqlx::query(
            "INSERT INTO reports \
             (id, created_at, user_id, player_id, app, platform, app_version, note, payload) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(summary.id.0.as_str())
        .bind(to_millis(summary.created_at))
        .bind(summary.user.0.as_str())
        .bind(summary.player.0.as_str())
        .bind(summary.app.as_str())
        .bind(summary.platform.as_str())
        .bind(summary.app_version.as_str())
        .bind(summary.note.as_deref())
        .bind(report.payload.as_str())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn list(
        &self,
        since: Timestamp,
        needle: Option<&str>,
        page: PageRequest,
    ) -> Result<Page<ReportSummary>, RepositoryError> {
        let _op = DbOpGuard::new("reports", "list");
        let count_sql = format!("SELECT COUNT(*) AS n FROM reports{}", where_clause(needle));
        let count = bind_filter(sqlx::query(AssertSqlSafe(count_sql)), since, needle)
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count, "n")? as u64;
        let rows = bind_filter(sqlx::query(AssertSqlSafe(list_sql(needle))), since, needle)
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let items = rows.iter().map(row_to_summary).collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
    }

    async fn get(
        &self,
        id: &ReportId,
        since: Timestamp,
    ) -> Result<Option<Report>, RepositoryError> {
        let _op = DbOpGuard::new("reports", "get");
        let row = sqlx::query("SELECT * FROM reports WHERE id = ? AND created_at >= ?")
            .bind(id.0.as_str())
            .bind(to_millis(since))
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref()
            .map(|row| {
                Ok(Report { summary: row_to_summary(row)?, payload: column(row, "payload")? })
            })
            .transpose()
    }

    async fn delete(&self, id: &ReportId) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("reports", "delete");
        let result = sqlx::query("DELETE FROM reports WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete_before(&self, cutoff: Timestamp) -> Result<u64, RepositoryError> {
        let _op = DbOpGuard::new("reports", "delete_before");
        let result = sqlx::query("DELETE FROM reports WHERE created_at < ?")
            .bind(to_millis(cutoff))
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn listing_walks_the_created_index_with_or_without_a_needle() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteReportRepo::connect(&dir.path().join("reports.db")).await.unwrap();
        for needle in [None, Some("roku")] {
            let sql = AssertSqlSafe(format!("EXPLAIN QUERY PLAN {}", list_sql(needle)));
            let rows = bind_filter(sqlx::query(sql), Timestamp::UNIX_EPOCH, needle)
                .bind(10_i64)
                .bind(0_i64)
                .fetch_all(&repo.pool)
                .await
                .unwrap();
            let plan = rows
                .iter()
                .map(|row| column::<String>(row, "detail").unwrap())
                .collect::<Vec<_>>()
                .join(" | ");
            assert!(
                plan.contains("USING INDEX reports_created_idx") && !plan.contains("TEMP B-TREE"),
                "the list reads the index in order instead of sorting ({needle:?}): {plan}"
            );
        }
    }

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteReportRepo::connect(&dir.path().join("reports.db")).await.unwrap();
        repo.pool.close().await;
        let id = ReportId("r1".into());
        let report = Report {
            summary: ReportSummary {
                id: id.clone(),
                created_at: Timestamp::UNIX_EPOCH,
                user: UserId("u1".into()),
                player: PlayerId("p1".into()),
                app: "flutter".into(),
                platform: "Android 14".into(),
                app_version: "0.0.7".into(),
                note: None,
            },
            payload: "{}".into(),
        };
        assert!(repo.insert(report).await.is_err());
        assert!(repo.list(Timestamp::UNIX_EPOCH, None, PageRequest::ALL).await.is_err());
        assert!(repo.list(Timestamp::UNIX_EPOCH, Some("roku"), PageRequest::ALL).await.is_err());
        assert!(repo.get(&id, Timestamp::UNIX_EPOCH).await.is_err());
        assert!(repo.delete(&id).await.is_err());
        assert!(repo.delete_before(Timestamp::UNIX_EPOCH).await.is_err());
    }
}
