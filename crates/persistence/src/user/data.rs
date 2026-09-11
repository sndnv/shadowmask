use domain::error::RepositoryError;
use domain::repository::UserDataStore;
use domain::user::UserId;

use crate::user::{SqlitePreferencesRepo, SqliteProgressRepo};

#[derive(Clone)]
pub struct SqliteUserData {
    progress: SqliteProgressRepo,
    preferences: SqlitePreferencesRepo,
}

impl SqliteUserData {
    pub fn new(progress: SqliteProgressRepo, preferences: SqlitePreferencesRepo) -> Self {
        Self { progress, preferences }
    }
}

impl UserDataStore for SqliteUserData {
    async fn purge(&self, user: &UserId) -> Result<(), RepositoryError> {
        self.progress.purge(user).await?;
        self.preferences.purge(user).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn purge_removes_both_databases_and_the_directory_holding_them() {
        let dir = tempfile::tempdir().unwrap();
        let progress = SqliteProgressRepo::new(dir.path());
        let preferences = SqlitePreferencesRepo::new(dir.path());
        let user = UserId("u1".into());
        progress.ensure_migrated(&user).await.unwrap();
        preferences.ensure_migrated(&user).await.unwrap();
        assert!(dir.path().join("u1").join("progress.db").exists());
        assert!(dir.path().join("u1").join("prefs.db").exists());

        SqliteUserData::new(progress, preferences).purge(&user).await.unwrap();

        assert!(!dir.path().join("u1").exists());
    }

    #[tokio::test]
    async fn purging_an_account_that_never_stored_anything_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let data = SqliteUserData::new(
            SqliteProgressRepo::new(dir.path()),
            SqlitePreferencesRepo::new(dir.path()),
        );

        data.purge(&UserId("never-seen".into())).await.unwrap();
    }
}
