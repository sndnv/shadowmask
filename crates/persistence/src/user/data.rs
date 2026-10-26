use std::collections::HashSet;

use domain::catalog::{TitleSet, VersionId};
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

    async fn forget(
        &self,
        user: &UserId,
        titles: &TitleSet,
        versions: &HashSet<VersionId>,
    ) -> Result<(), RepositoryError> {
        self.progress.forget(user, titles, versions).await?;
        self.preferences.forget(user, titles, versions).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{EpisodeId, MovieId, TitleId};
    use domain::playback::{
        Favorite, PlaybackProgress, SubtitleTrackRef, UserSubtitleOffset, WatchlistItem,
    };
    use domain::repository::{PreferencesRepository, ProgressRepository};
    use jiff::Timestamp;

    fn movie(id: &str) -> TitleId {
        TitleId::Movie(MovieId(id.into()))
    }

    fn episode(id: &str) -> TitleId {
        TitleId::Episode(EpisodeId(id.into()))
    }

    async fn seed(
        progress: &SqliteProgressRepo,
        preferences: &SqlitePreferencesRepo,
        user: &UserId,
    ) {
        for version in ["v1", "v2"] {
            progress
                .upsert(PlaybackProgress {
                    user: user.clone(),
                    version: VersionId(version.into()),
                    position_ms: 10,
                    audio_track: None,
                    subtitle: None,
                    updated_at: Timestamp::UNIX_EPOCH,
                })
                .await
                .unwrap();
            preferences
                .set_subtitle_offset(UserSubtitleOffset {
                    user: user.clone(),
                    version: VersionId(version.into()),
                    subtitle: SubtitleTrackRef::Embedded(0),
                    offset_ms: 250,
                })
                .await
                .unwrap();
        }
        for title in [movie("m1"), episode("e1"), movie("m2")] {
            progress.record_view(user, &title, Timestamp::UNIX_EPOCH).await.unwrap();
            preferences
                .add_watchlist(WatchlistItem {
                    user: user.clone(),
                    title: title.clone(),
                    added_at: Timestamp::UNIX_EPOCH,
                })
                .await
                .unwrap();
            preferences
                .add_favorite(Favorite {
                    user: user.clone(),
                    title,
                    added_at: Timestamp::UNIX_EPOCH,
                })
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn forgetting_removes_only_the_named_titles_and_versions() {
        let dir = tempfile::tempdir().unwrap();
        let progress = SqliteProgressRepo::new(dir.path());
        let preferences = SqlitePreferencesRepo::new(dir.path());
        let (user, other) = (UserId("u1".into()), UserId("u2".into()));
        seed(&progress, &preferences, &user).await;
        seed(&progress, &preferences, &other).await;
        let titles = TitleSet {
            movies: HashSet::from([MovieId("m1".into())]),
            episodes: HashSet::from([EpisodeId("e1".into())]),
            ..TitleSet::default()
        };
        let versions = HashSet::from([VersionId("v1".into())]);

        let data = SqliteUserData::new(progress.clone(), preferences.clone());
        data.forget(&user, &titles, &versions).await.unwrap();

        let resumed: Vec<String> = progress
            .list_in_progress(&user)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.version.0)
            .collect();
        assert_eq!(resumed, ["v2"]);
        let watched: Vec<TitleId> =
            progress.watched_state(&user).await.unwrap().into_iter().map(|row| row.title).collect();
        assert_eq!(watched, [movie("m2")]);
        let listed: Vec<TitleId> =
            preferences.list_watchlist(&user).await.unwrap().into_iter().map(|i| i.title).collect();
        assert_eq!(listed, [movie("m2")]);
        let liked: Vec<TitleId> =
            preferences.list_favorites(&user).await.unwrap().into_iter().map(|i| i.title).collect();
        assert_eq!(liked, [movie("m2")]);
        let track = SubtitleTrackRef::Embedded(0);
        let v1 = VersionId("v1".into());
        let v2 = VersionId("v2".into());
        assert!(preferences.get_subtitle_offset(&user, &v1, &track).await.unwrap().is_none());
        assert!(preferences.get_subtitle_offset(&user, &v2, &track).await.unwrap().is_some());
        assert_eq!(preferences.list_watchlist(&other).await.unwrap().len(), 3);
        assert_eq!(progress.list_in_progress(&other).await.unwrap().len(), 2);
        assert!(preferences.get_subtitle_offset(&other, &v1, &track).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn forgetting_fails_when_the_users_databases_cannot_be_opened() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("u1"), b"").unwrap();
        let data = SqliteUserData::new(
            SqliteProgressRepo::new(dir.path()),
            SqlitePreferencesRepo::new(dir.path()),
        );

        let forgot = data.forget(&UserId("u1".into()), &TitleSet::default(), &HashSet::new()).await;

        assert!(forgot.is_err());
    }

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
