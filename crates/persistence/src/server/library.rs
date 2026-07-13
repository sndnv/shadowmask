use std::path::Path;

use domain::common::{Page, PageRequest};
use domain::error::RepositoryError;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryKind, MatchCandidate,
    ResolutionStatus, ScanState, ScanStatus, UnmatchedFile, UnmatchedFileId, WatcherStrategy,
};
use domain::repository::LibraryRepository;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;

use crate::codec::{title_from_parts, title_kind};
use crate::pool::{backend, column, from_millis, open, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/libraries");

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

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn insert_library(&self, library: Library) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = library.id.0.as_str();
        sqlx::query(
            "INSERT OR REPLACE INTO libraries (id, name, kind, watcher, scan_schedule) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(library.name.as_str())
        .bind(kind_to_str(library.kind))
        .bind(watcher_to_str(library.watcher))
        .bind(library.scan_schedule.as_deref())
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

    async fn strings(&self, query: &str, key: &str) -> Result<Vec<String>, RepositoryError> {
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
            roots,
            watcher: watcher_from_str(&column::<String>(row, "watcher")?)?,
            scan_schedule: column(row, "scan_schedule")?,
            metadata_sources,
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

    async fn load_paths(&self, duplicate_id: &str) -> Result<Vec<String>, RepositoryError> {
        self.strings(
            "SELECT path AS value FROM duplicate_paths WHERE duplicate_id = ? ORDER BY ordinal",
            duplicate_id,
        )
        .await
    }
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
        ScanStatus::Running => "running",
        ScanStatus::Failed => "failed",
    }
}

fn scan_status_from_str(value: &str) -> Result<ScanStatus, RepositoryError> {
    match value {
        "idle" => Ok(ScanStatus::Idle),
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
        last_scanned_at: column::<Option<i64>>(row, "last_scanned_at")?
            .map(from_millis)
            .transpose()?,
        error: column(row, "error")?,
    })
}

async fn count(pool: &SqlitePool, query: &str, key: &str) -> Result<u64, RepositoryError> {
    let row = sqlx::query(query)
        .bind(key)
        .fetch_one(pool)
        .await
        .map_err(backend)?;
    Ok(column::<i64>(&row, "n")? as u64)
}

impl LibraryRepository for SqliteLibraryRepo {
    async fn list(&self) -> Result<Vec<Library>, RepositoryError> {
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
        let row = sqlx::query("SELECT * FROM scan_state WHERE library_id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_scan_state).transpose()
    }

    async fn save_scan_state(&self, state: ScanState) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT OR REPLACE INTO scan_state (library_id, status, progress, last_scanned_at, error) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(state.library.0.as_str())
        .bind(scan_status_to_str(state.status))
        .bind(f64::from(state.progress))
        .bind(state.last_scanned_at.map(to_millis))
        .bind(state.error.as_deref())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn upsert(&self, library: Library) -> Result<(), RepositoryError> {
        self.insert_library(library).await
    }

    async fn delete(&self, id: &LibraryId) -> Result<(), RepositoryError> {
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
        let total = count(
            &self.pool,
            "SELECT COUNT(*) AS n FROM unmatched_files WHERE library_id = ? AND status = 'active'",
            id.0.as_str(),
        )
        .await?;
        let rows = sqlx::query(
            "SELECT * FROM unmatched_files WHERE library_id = ? AND status = 'active' \
             ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(id.0.as_str())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let uid: String = column(row, "id")?;
            let candidates = self.load_candidates(&uid).await?;
            items.push(UnmatchedFile {
                library: LibraryId(column(row, "library_id")?),
                path: column(row, "path")?,
                candidates,
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
        let total = count(
            &self.pool,
            "SELECT COUNT(*) AS n FROM duplicate_candidates \
             WHERE library_id = ? AND status = 'active'",
            id.0.as_str(),
        )
        .await?;
        let rows = sqlx::query(
            "SELECT * FROM duplicate_candidates WHERE library_id = ? AND status = 'active' \
             ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(id.0.as_str())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            let did: String = column(row, "id")?;
            let paths = self.load_paths(&did).await?;
            items.push(DuplicateCandidate {
                title: title_from_parts(
                    &column::<String>(row, "title_kind")?,
                    column(row, "title_id")?,
                )?,
                paths,
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
                    id: UnmatchedFileId(uid),
                }))
            }
            None => Ok(None),
        }
    }

    async fn insert_unmatched(&self, file: UnmatchedFile) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = file.id.0.as_str();
        sqlx::query(
            "INSERT INTO unmatched_files (id, library_id, path) VALUES (?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET library_id = excluded.library_id, path = excluded.path",
        )
        .bind(id)
        .bind(file.library.0.as_str())
        .bind(file.path.as_str())
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
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = duplicate.id.0.as_str();
        sqlx::query(
            "INSERT INTO duplicate_candidates (id, library_id, title_kind, title_id) \
             VALUES (?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET library_id = excluded.library_id, \
             title_kind = excluded.title_kind, title_id = excluded.title_id",
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
        sqlx::query("UPDATE duplicate_candidates SET status = ? WHERE id = ?")
            .bind(resolution_status_to_str(status))
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_round_trips_and_rejects_unknown() {
        for kind in [LibraryKind::Movie, LibraryKind::Tv] {
            assert_eq!(kind_from_str(kind_to_str(kind)).unwrap(), kind);
        }
        for watcher in [
            WatcherStrategy::Local,
            WatcherStrategy::Polling,
            WatcherStrategy::Scheduled,
            WatcherStrategy::Manual,
        ] {
            assert_eq!(watcher_from_str(watcher_to_str(watcher)).unwrap(), watcher);
        }
        for status in [ScanStatus::Idle, ScanStatus::Running, ScanStatus::Failed] {
            assert_eq!(
                scan_status_from_str(scan_status_to_str(status)).unwrap(),
                status
            );
        }
        assert!(kind_from_str("nope").is_err());
        assert!(watcher_from_str("nope").is_err());
        assert!(scan_status_from_str("nope").is_err());
    }

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteLibraryRepo::connect(&dir.path().join("libraries.db"))
            .await
            .unwrap();
        repo.pool.close().await;
        assert!(repo.list().await.is_err());
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
            roots: vec!["/a".into(), "/b".into()],
            watcher: WatcherStrategy::Scheduled,
            scan_schedule: Some("0 0 * * *".into()),
            metadata_sources: vec!["tmdb".into(), "omdb".into()],
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
            "INSERT INTO unmatched_files (id, library_id, path) VALUES ('uf1', 'lib1', '/x.mkv')",
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
