use std::collections::{HashMap, HashSet};
use std::path::Path;

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryKind, LibraryOrigin,
    MatchCandidate, ResolutionStatus, ScanState, ScanStatus, UnmatchedFile, UnmatchedFileId,
    WatcherStrategy,
};
use domain::repository::LibraryRepository;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{AssertSqlSafe, SqliteConnection, SqlitePool};

use crate::codec::{title_from_parts, title_kind};
use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/libraries");

const LIST_UNMATCHED_SQL: &str = "SELECT * FROM unmatched_files \
     WHERE library_id = ? AND status = 'active' ORDER BY id LIMIT ? OFFSET ?";
const LIST_DUPLICATES_SQL: &str = "SELECT * FROM duplicate_candidates \
     WHERE library_id = ? AND status = 'active' ORDER BY id LIMIT ? OFFSET ?";

const BIND_CHUNK: usize = 500;

async fn delete_by_id(
    conn: &mut SqliteConnection,
    table: &str,
    ids: &[String],
) -> Result<(), RepositoryError> {
    for chunk in ids.chunks(BIND_CHUNK) {
        let placeholders = vec!["?"; chunk.len()].join(", ");
        let mut query = sqlx::query(AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE id IN ({placeholders})"
        )));
        for id in chunk {
            query = query.bind(id.as_str());
        }
        query.execute(&mut *conn).await.map_err(backend)?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct SqliteLibraryRepo {
    pool: SqlitePool,
}

impl SqliteLibraryRepo {
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

    pub async fn insert_library(&self, library: Library) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = library.id.0.as_str();
        sqlx::query(
            "INSERT OR REPLACE INTO libraries (id, name, kind, origin, watcher, scan_schedule, sort_articles, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, COALESCE((SELECT created_at FROM libraries WHERE id = ?), ?), ?)",
        )
        .bind(id)
        .bind(library.name.as_str())
        .bind(kind_to_str(library.kind))
        .bind(origin_to_str(library.origin))
        .bind(watcher_to_str(library.watcher))
        .bind(library.scan_schedule.as_deref())
        .bind(join_list(&library.sort_articles))
        .bind(id)
        .bind(to_millis(library.created_at))
        .bind(to_millis(library.updated_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sqlx::query("DELETE FROM library_roots WHERE library_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, root) in library.roots.iter().enumerate() {
            sqlx::query("INSERT INTO library_roots (library_id, ordinal, path) VALUES (?, ?, ?)")
                .bind(id)
                .bind(ordinal as i64)
                .bind(root.as_str())
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        sqlx::query("DELETE FROM library_metadata_sources WHERE library_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, source) in library.metadata_sources.iter().enumerate() {
            sqlx::query(
                "INSERT INTO library_metadata_sources (library_id, ordinal, source) \
                 VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(ordinal as i64)
            .bind(source.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn strings(
        &self,
        query: &'static str,
        key: &str,
    ) -> Result<Vec<String>, RepositoryError> {
        let rows = sqlx::query(query)
            .bind(key)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter()
            .map(|row| column::<String>(row, "value"))
            .collect()
    }

    async fn load_library(&self, row: &SqliteRow) -> Result<Library, RepositoryError> {
        let id: String = column(row, "id")?;
        let roots = self
            .strings(
                "SELECT path AS value FROM library_roots WHERE library_id = ? ORDER BY ordinal",
                &id,
            )
            .await?;
        let metadata_sources = self
            .strings(
                "SELECT source AS value FROM library_metadata_sources \
                 WHERE library_id = ? ORDER BY ordinal",
                &id,
            )
            .await?;
        Ok(Library {
            name: column(row, "name")?,
            kind: kind_from_str(&column::<String>(row, "kind")?)?,
            origin: origin_from_str(&column::<String>(row, "origin")?)?,
            roots,
            watcher: watcher_from_str(&column::<String>(row, "watcher")?)?,
            scan_schedule: column(row, "scan_schedule")?,
            metadata_sources,
            sort_articles: split_list(&column::<String>(row, "sort_articles")?),
            created_at: from_millis(column(row, "created_at")?)?,
            updated_at: from_millis(column(row, "updated_at")?)?,
            id: LibraryId(id),
        })
    }

    async fn load_candidates(
        &self,
        unmatched_id: &str,
    ) -> Result<Vec<MatchCandidate>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT title_kind, title_id, confidence, label FROM unmatched_candidates \
             WHERE unmatched_id = ? ORDER BY ordinal",
        )
        .bind(unmatched_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(row_to_candidate).collect()
    }

    async fn candidates_for(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, Vec<MatchCandidate>>, RepositoryError> {
        let mut out: HashMap<String, Vec<MatchCandidate>> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT unmatched_id, title_kind, title_id, confidence, label \
                 FROM unmatched_candidates WHERE unmatched_id IN ({placeholders}) \
                 ORDER BY unmatched_id, ordinal"
            );
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for row in &rows {
                out.entry(column(row, "unmatched_id")?)
                    .or_default()
                    .push(row_to_candidate(row)?);
            }
        }
        Ok(out)
    }

    async fn paths_for(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, Vec<String>>, RepositoryError> {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT duplicate_id, path FROM duplicate_paths \
                 WHERE duplicate_id IN ({placeholders}) ORDER BY duplicate_id, ordinal"
            );
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for row in &rows {
                out.entry(column(row, "duplicate_id")?)
                    .or_default()
                    .push(column(row, "path")?);
            }
        }
        Ok(out)
    }
}

fn join_list(values: &[String]) -> String {
    values.join(",")
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn kind_to_str(kind: LibraryKind) -> &'static str {
    match kind {
        LibraryKind::Movie => "movie",
        LibraryKind::Tv => "tv",
    }
}

fn kind_from_str(value: &str) -> Result<LibraryKind, RepositoryError> {
    match value {
        "movie" => Ok(LibraryKind::Movie),
        "tv" => Ok(LibraryKind::Tv),
        other => Err(backend(format!("unknown library kind: {other}"))),
    }
}

fn origin_to_str(origin: LibraryOrigin) -> &'static str {
    match origin {
        LibraryOrigin::Local => "local",
        LibraryOrigin::External => "external",
    }
}

fn origin_from_str(value: &str) -> Result<LibraryOrigin, RepositoryError> {
    match value {
        "local" => Ok(LibraryOrigin::Local),
        "external" => Ok(LibraryOrigin::External),
        other => Err(backend(format!("unknown library origin: {other}"))),
    }
}

fn watcher_to_str(watcher: WatcherStrategy) -> &'static str {
    match watcher {
        WatcherStrategy::Local => "local",
        WatcherStrategy::Polling => "polling",
        WatcherStrategy::Scheduled => "scheduled",
        WatcherStrategy::Manual => "manual",
    }
}

fn watcher_from_str(value: &str) -> Result<WatcherStrategy, RepositoryError> {
    match value {
        "local" => Ok(WatcherStrategy::Local),
        "polling" => Ok(WatcherStrategy::Polling),
        "scheduled" => Ok(WatcherStrategy::Scheduled),
        "manual" => Ok(WatcherStrategy::Manual),
        other => Err(backend(format!("unknown watcher strategy: {other}"))),
    }
}

fn scan_status_to_str(status: ScanStatus) -> &'static str {
    match status {
        ScanStatus::Idle => "idle",
        ScanStatus::Queued => "queued",
        ScanStatus::Running => "running",
        ScanStatus::Failed => "failed",
    }
}

fn scan_status_from_str(value: &str) -> Result<ScanStatus, RepositoryError> {
    match value {
        "idle" => Ok(ScanStatus::Idle),
        "queued" => Ok(ScanStatus::Queued),
        "running" => Ok(ScanStatus::Running),
        "failed" => Ok(ScanStatus::Failed),
        other => Err(backend(format!("unknown scan status: {other}"))),
    }
}

fn resolution_status_to_str(status: ResolutionStatus) -> &'static str {
    match status {
        ResolutionStatus::Active => "active",
        ResolutionStatus::Resolved => "resolved",
        ResolutionStatus::Dismissed => "dismissed",
    }
}

fn row_to_candidate(row: &SqliteRow) -> Result<MatchCandidate, RepositoryError> {
    Ok(MatchCandidate {
        title: title_from_parts(
            &column::<String>(row, "title_kind")?,
            column(row, "title_id")?,
        )?,
        confidence: column::<f64>(row, "confidence")? as f32,
        label: column(row, "label")?,
    })
}

fn row_to_scan_state(row: &SqliteRow) -> Result<ScanState, RepositoryError> {
    Ok(ScanState {
        library: LibraryId(column(row, "library_id")?),
        status: scan_status_from_str(&column::<String>(row, "status")?)?,
        progress: column::<f64>(row, "progress")? as f32,
        started_at: column::<Option<i64>>(row, "started_at")?
            .map(from_millis)
            .transpose()?,
        last_scanned_at: column::<Option<i64>>(row, "last_scanned_at")?
            .map(from_millis)
            .transpose()?,
        error: column(row, "error")?,
    })
}

async fn count(pool: &SqlitePool, query: &'static str, key: &str) -> Result<u64, RepositoryError> {
    let row = sqlx::query(query)
        .bind(key)
        .fetch_one(pool)
        .await
        .map_err(backend)?;
    Ok(column::<i64>(&row, "n")? as u64)
}

impl LibraryRepository for SqliteLibraryRepo {
    async fn list(&self) -> Result<Vec<Library>, RepositoryError> {
        let _op = DbOpGuard::new("library", "list");
        let rows = sqlx::query("SELECT * FROM libraries ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut libraries = Vec::with_capacity(rows.len());
        for row in &rows {
            libraries.push(self.load_library(row).await?);
        }
        Ok(libraries)
    }

    async fn get(&self, id: &LibraryId) -> Result<Option<Library>, RepositoryError> {
        let _op = DbOpGuard::new("library", "get");
        let row = sqlx::query("SELECT * FROM libraries WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(row) => Ok(Some(self.load_library(&row).await?)),
            None => Ok(None),
        }
    }

    async fn scan_state(&self, id: &LibraryId) -> Result<Option<ScanState>, RepositoryError> {
        let _op = DbOpGuard::new("library", "scan_state");
        let row = sqlx::query("SELECT * FROM scan_state WHERE library_id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_scan_state).transpose()
    }

    async fn save_scan_state(&self, state: ScanState) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "save_scan_state");
        sqlx::query(
            "INSERT OR REPLACE INTO scan_state (library_id, status, progress, started_at, last_scanned_at, error) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(state.library.0.as_str())
        .bind(scan_status_to_str(state.status))
        .bind(f64::from(state.progress))
        .bind(state.started_at.map(to_millis))
        .bind(state.last_scanned_at.map(to_millis))
        .bind(state.error.as_deref())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn upsert(&self, library: Library) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "upsert");
        self.insert_library(library).await
    }

    async fn delete(&self, id: &LibraryId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "delete");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let key = id.0.as_str();
        sqlx::query("DELETE FROM scan_state WHERE library_id = ?")
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM unmatched_files WHERE library_id = ?")
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM duplicate_candidates WHERE library_id = ?")
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM libraries WHERE id = ?")
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn list_unmatched(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<UnmatchedFile>, RepositoryError> {
        let _op = DbOpGuard::new("library", "list_unmatched");
        let total = count(
            &self.pool,
            "SELECT COUNT(*) AS n FROM unmatched_files WHERE library_id = ? AND status = 'active'",
            id.0.as_str(),
        )
        .await?;
        let rows = sqlx::query(LIST_UNMATCHED_SQL)
            .bind(id.0.as_str())
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let ids = rows
            .iter()
            .map(|row| column::<String>(row, "id"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut candidates = self.candidates_for(&ids).await?;
        let mut items = Vec::with_capacity(rows.len());
        for (row, uid) in rows.iter().zip(ids) {
            items.push(UnmatchedFile {
                library: LibraryId(column(row, "library_id")?),
                path: column(row, "path")?,
                candidates: candidates.remove(&uid).unwrap_or_default(),
                created_at: from_millis(column(row, "created_at")?)?,
                updated_at: from_millis(column(row, "updated_at")?)?,
                id: UnmatchedFileId(uid),
            });
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn list_duplicates(
        &self,
        id: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<DuplicateCandidate>, RepositoryError> {
        let _op = DbOpGuard::new("library", "list_duplicates");
        let total = count(
            &self.pool,
            "SELECT COUNT(*) AS n FROM duplicate_candidates \
             WHERE library_id = ? AND status = 'active'",
            id.0.as_str(),
        )
        .await?;
        let rows = sqlx::query(LIST_DUPLICATES_SQL)
            .bind(id.0.as_str())
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let ids = rows
            .iter()
            .map(|row| column::<String>(row, "id"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut paths = self.paths_for(&ids).await?;
        let mut items = Vec::with_capacity(rows.len());
        for (row, did) in rows.iter().zip(ids) {
            items.push(DuplicateCandidate {
                title: title_from_parts(
                    &column::<String>(row, "title_kind")?,
                    column(row, "title_id")?,
                )?,
                paths: paths.remove(&did).unwrap_or_default(),
                id: DuplicateCandidateId(did),
            });
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_unmatched(
        &self,
        id: &UnmatchedFileId,
    ) -> Result<Option<UnmatchedFile>, RepositoryError> {
        let _op = DbOpGuard::new("library", "get_unmatched");
        let row = sqlx::query("SELECT * FROM unmatched_files WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(row) => {
                let uid: String = column(&row, "id")?;
                let candidates = self.load_candidates(&uid).await?;
                Ok(Some(UnmatchedFile {
                    library: LibraryId(column(&row, "library_id")?),
                    path: column(&row, "path")?,
                    candidates,
                    created_at: from_millis(column(&row, "created_at")?)?,
                    updated_at: from_millis(column(&row, "updated_at")?)?,
                    id: UnmatchedFileId(uid),
                }))
            }
            None => Ok(None),
        }
    }

    async fn insert_unmatched(&self, file: UnmatchedFile) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "insert_unmatched");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = file.id.0.as_str();
        sqlx::query(
            "INSERT INTO unmatched_files (id, library_id, path, created_at, updated_at) VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET library_id = excluded.library_id, path = excluded.path, \
             updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(file.library.0.as_str())
        .bind(file.path.as_str())
        .bind(to_millis(file.created_at))
        .bind(to_millis(file.updated_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sqlx::query("DELETE FROM unmatched_candidates WHERE unmatched_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, candidate) in file.candidates.iter().enumerate() {
            sqlx::query(
                "INSERT INTO unmatched_candidates \
                 (unmatched_id, ordinal, title_kind, title_id, confidence, label) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(ordinal as i64)
            .bind(title_kind(&candidate.title))
            .bind(candidate.title.id())
            .bind(f64::from(candidate.confidence))
            .bind(candidate.label.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn insert_duplicate(
        &self,
        library: &LibraryId,
        duplicate: DuplicateCandidate,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "insert_duplicate");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = duplicate.id.0.as_str();
        sqlx::query(
            "INSERT INTO duplicate_candidates (id, library_id, title_kind, title_id) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET library_id = excluded.library_id, \
             title_kind = excluded.title_kind, title_id = excluded.title_id, \
             status = 'active'",
        )
        .bind(id)
        .bind(library.0.as_str())
        .bind(title_kind(&duplicate.title))
        .bind(duplicate.title.id())
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sqlx::query("DELETE FROM duplicate_paths WHERE duplicate_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, path) in duplicate.paths.iter().enumerate() {
            sqlx::query(
                "INSERT INTO duplicate_paths (duplicate_id, ordinal, path) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(ordinal as i64)
            .bind(path.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn set_unmatched_status(
        &self,
        id: &UnmatchedFileId,
        status: ResolutionStatus,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "set_unmatched_status");
        sqlx::query("UPDATE unmatched_files SET status = ? WHERE id = ?")
            .bind(resolution_status_to_str(status))
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn set_duplicate_status(
        &self,
        id: &DuplicateCandidateId,
        status: ResolutionStatus,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "set_duplicate_status");
        sqlx::query("UPDATE duplicate_candidates SET status = ? WHERE id = ?")
            .bind(resolution_status_to_str(status))
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn reconcile_duplicates(
        &self,
        library: &LibraryId,
        detected: &[DuplicateCandidateId],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "reconcile_duplicates");
        let keep: HashSet<&str> = detected.iter().map(|id| id.0.as_str()).collect();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let rows = sqlx::query("SELECT id FROM duplicate_candidates WHERE library_id = ?")
            .bind(library.0.as_str())
            .fetch_all(&mut *tx)
            .await
            .map_err(backend)?;
        let mut stale = Vec::new();
        for row in &rows {
            let id = column::<String>(row, "id")?;
            if !keep.contains(id.as_str()) {
                stale.push(id);
            }
        }
        delete_by_id(&mut tx, "duplicate_candidates", &stale).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn reconcile_unmatched(
        &self,
        library: &LibraryId,
        present_paths: &[String],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("library", "reconcile_unmatched");
        let keep: HashSet<&str> = present_paths.iter().map(String::as_str).collect();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let rows = sqlx::query("SELECT id, path FROM unmatched_files WHERE library_id = ?")
            .bind(library.0.as_str())
            .fetch_all(&mut *tx)
            .await
            .map_err(backend)?;
        let mut stale = Vec::new();
        for row in &rows {
            if !keep.contains(column::<String>(row, "path")?.as_str()) {
                stale.push(column::<String>(row, "id")?);
            }
        }
        delete_by_id(&mut tx, "unmatched_files", &stale).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{MovieId, TitleId};

    #[tokio::test]
    async fn reconciling_a_backlog_deletes_more_rows_than_sqlite_allows_parameters() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        let rows = 40_000;
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO unmatched_files (id, library_id, path, status, created_at, updated_at) \
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
             SELECT 'u-' || i, 'lib-1', '/media/' || i || '.mkv', 'active', 0, 0 FROM c"
        )))
        .execute(&repo.pool)
        .await
        .unwrap();
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO duplicate_candidates (id, library_id, title_kind, title_id, status) \
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
             SELECT 'd-' || i, 'lib-1', 'movie', 'm-' || i, 'active' FROM c"
        )))
        .execute(&repo.pool)
        .await
        .unwrap();

        let library = LibraryId("lib-1".into());
        repo.reconcile_unmatched(&library, &["/media/1.mkv".to_string()])
            .await
            .unwrap();
        repo.reconcile_duplicates(&library, &[DuplicateCandidateId("d-1".into())])
            .await
            .unwrap();

        for table in ["unmatched_files", "duplicate_candidates"] {
            let left: i64 = column(
                &sqlx::query(AssertSqlSafe(format!("SELECT COUNT(*) AS n FROM {table}")))
                    .fetch_one(&repo.pool)
                    .await
                    .unwrap(),
                "n",
            )
            .unwrap();
            assert_eq!(
                left, 1,
                "a rescan clears the whole previous backlog, and {table} binds one parameter \
                 per removed row against a SQLite cap of 32766"
            );
        }
    }

    #[tokio::test]
    async fn scan_backlog_listings_are_served_by_an_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        for (sql, index) in [
            (LIST_UNMATCHED_SQL, "idx_unmatched_library"),
            (LIST_DUPLICATES_SQL, "idx_duplicates_library"),
        ] {
            let rows = sqlx::query(AssertSqlSafe(format!("EXPLAIN QUERY PLAN {sql}")))
                .bind("lib-1")
                .bind(50_i64)
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
                plan.contains(&format!("USING INDEX {index}")),
                "a 200,000 file backlog cannot be paged by scanning it: {plan}"
            );
        }
    }

    #[test]
    fn enum_round_trips_and_rejects_unknown() {
        for kind in [LibraryKind::Movie, LibraryKind::Tv] {
            assert_eq!(kind_from_str(kind_to_str(kind)).unwrap(), kind);
        }
        for origin in [LibraryOrigin::Local, LibraryOrigin::External] {
            assert_eq!(origin_from_str(origin_to_str(origin)).unwrap(), origin);
        }
        for watcher in [
            WatcherStrategy::Local,
            WatcherStrategy::Polling,
            WatcherStrategy::Scheduled,
            WatcherStrategy::Manual,
        ] {
            assert_eq!(watcher_from_str(watcher_to_str(watcher)).unwrap(), watcher);
        }
        for status in [
            ScanStatus::Idle,
            ScanStatus::Queued,
            ScanStatus::Running,
            ScanStatus::Failed,
        ] {
            assert_eq!(
                scan_status_from_str(scan_status_to_str(status)).unwrap(),
                status
            );
        }
        assert!(kind_from_str("nope").is_err());
        assert!(origin_from_str("nope").is_err());
        assert!(watcher_from_str("nope").is_err());
        assert!(scan_status_from_str("nope").is_err());
    }

    // A method absent from this list has an error arm no test has ever taken.
    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        repo.pool.close().await;

        let page = PageRequest {
            offset: 0,
            limit: 10,
        };
        let library = LibraryId("lib1".into());
        let unmatched = UnmatchedFileId("uf1".into());
        let duplicate = DuplicateCandidateId("d1".into());

        assert!(repo.list().await.is_err());
        assert!(repo.get(&library).await.is_err());
        assert!(repo.scan_state(&library).await.is_err());
        assert!(
            repo.save_scan_state(ScanState {
                library: library.clone(),
                status: ScanStatus::Idle,
                progress: 0.0,
                started_at: None,
                last_scanned_at: None,
                error: None,
            })
            .await
            .is_err()
        );
        assert!(
            repo.upsert(contracts::fixture::library("lib1"))
                .await
                .is_err()
        );
        assert!(repo.delete(&library).await.is_err());
        assert!(repo.list_unmatched(&library, page).await.is_err());
        assert!(repo.list_duplicates(&library, page).await.is_err());
        assert!(repo.get_unmatched(&unmatched).await.is_err());
        assert!(
            repo.insert_unmatched(UnmatchedFile {
                id: unmatched.clone(),
                library: library.clone(),
                path: "/media/x.mkv".into(),
                candidates: Vec::new(),
                created_at: from_millis(0).unwrap(),
                updated_at: from_millis(0).unwrap(),
            })
            .await
            .is_err()
        );
        assert!(
            repo.insert_duplicate(
                &library,
                DuplicateCandidate {
                    id: duplicate.clone(),
                    title: TitleId::Movie(MovieId("m1".into())),
                    paths: vec!["/a.mkv".into()],
                }
            )
            .await
            .is_err()
        );
        assert!(
            repo.set_unmatched_status(&unmatched, ResolutionStatus::Dismissed)
                .await
                .is_err()
        );
        assert!(
            repo.set_duplicate_status(&duplicate, ResolutionStatus::Dismissed)
                .await
                .is_err()
        );
        assert!(
            repo.reconcile_duplicates(&library, &[duplicate])
                .await
                .is_err()
        );
        assert!(
            repo.reconcile_unmatched(&library, &["/media/x.mkv".to_owned()])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn library_with_roots_sources_and_schedule_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        repo.insert_library(Library {
            id: LibraryId("lib1".into()),
            name: "Films".into(),
            kind: LibraryKind::Tv,
            origin: LibraryOrigin::External,
            roots: vec!["/a".into(), "/b".into()],
            watcher: WatcherStrategy::Scheduled,
            scan_schedule: Some("0 0 * * *".into()),
            metadata_sources: vec!["tmdb".into(), "omdb".into()],
            sort_articles: vec!["the".into(), "a".into()],
            created_at: from_millis(0).unwrap(),
            updated_at: from_millis(0).unwrap(),
        })
        .await
        .unwrap();
        let loaded = repo.get(&LibraryId("lib1".into())).await.unwrap().unwrap();
        assert_eq!(loaded.roots, vec!["/a".to_owned(), "/b".to_owned()]);
        assert_eq!(
            loaded.metadata_sources,
            vec!["tmdb".to_owned(), "omdb".to_owned()]
        );
        assert_eq!(loaded.scan_schedule, Some("0 0 * * *".to_owned()));
        assert_eq!(loaded.kind, LibraryKind::Tv);
        assert_eq!(loaded.origin, LibraryOrigin::External);
        assert_eq!(loaded.watcher, WatcherStrategy::Scheduled);
    }

    #[tokio::test]
    async fn rejects_corrupt_candidate_and_duplicate_title_kind() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        let library = LibraryId("lib1".into());
        let page = PageRequest {
            offset: 0,
            limit: 10,
        };
        sqlx::query(
            "INSERT INTO unmatched_files (id, library_id, path, created_at, updated_at) \
             VALUES ('uf1', 'lib1', '/x.mkv', 0, 0)",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO unmatched_candidates \
             (unmatched_id, ordinal, title_kind, title_id, confidence, label) \
             VALUES ('uf1', 0, 'bogus', 'm1', 0.5, 'x')",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.list_unmatched(&library, page).await.is_err());

        sqlx::query(
            "INSERT INTO duplicate_candidates (id, library_id, title_kind, title_id) \
             VALUES ('d1', 'lib1', 'bogus', 'm1')",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.list_duplicates(&library, page).await.is_err());
    }
}
