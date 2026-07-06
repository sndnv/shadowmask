use std::io::ErrorKind;
use std::path::Path;

use domain::error::RepositoryError;
use domain::user::UserId;

use crate::pool::backend;
use crate::server::{
    SqliteAuthTokenRepo, SqliteCatalogRepo, SqliteJobRepo, SqliteLibraryRepo, SqliteUserRepo,
};
use crate::user::{SqlitePreferencesRepo, SqliteProgressRepo};

pub async fn migrate_all(db_root: &Path) -> Result<(), RepositoryError> {
    migrate_server(&db_root.join("server")).await?;
    migrate_users(&db_root.join("users")).await?;
    Ok(())
}

async fn migrate_server(dir: &Path) -> Result<(), RepositoryError> {
    tokio::fs::create_dir_all(dir).await.map_err(backend)?;
    SqliteJobRepo::connect(&dir.join("jobs.db")).await?;
    SqliteLibraryRepo::connect(&dir.join("libraries.db")).await?;
    SqliteUserRepo::connect(&dir.join("users.db")).await?;
    SqliteCatalogRepo::connect(&dir.join("catalog.db")).await?;
    SqliteAuthTokenRepo::connect(&dir.join("auth.db")).await?;
    Ok(())
}

async fn migrate_users(dir: &Path) -> Result<(), RepositoryError> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(backend(error)),
    };
    let progress = SqliteProgressRepo::new(dir);
    let prefs = SqlitePreferencesRepo::new(dir);
    while let Some(entry) = entries.next_entry().await.map_err(backend)? {
        if !entry.file_type().await.map_err(backend)?.is_dir() {
            continue;
        }
        let user = UserId(entry.file_name().to_string_lossy().into_owned());
        progress.ensure_migrated(&user).await?;
        prefs.ensure_migrated(&user).await?;
    }
    Ok(())
}
