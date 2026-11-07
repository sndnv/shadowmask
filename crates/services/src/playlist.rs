use std::collections::HashSet;

use domain::catalog::{EpisodeId, TitleId, TitleListFilter};
use domain::discovery::Audience;
use domain::error::{PlaylistError, RepositoryError};
use domain::playback::{
    Playlist, PlaylistCard, PlaylistDetail, PlaylistId, PlaylistSummary, QueueCard, QueueEntryId,
    WatchTarget,
};
use domain::repository::{CatalogRepository, PreferencesRepository, UserRepository};
use domain::service::PlaylistService;
use domain::user::UserId;
use jiff::Timestamp;
use uuid::Uuid;

use crate::acl;
use crate::discovery::{visible_cards, visible_titles};

const SPECIALS: u16 = 0;

#[derive(Clone)]
pub struct PlaylistServiceImpl<Pf, C, U> {
    preferences: Pf,
    catalog: C,
    users: U,
}

impl<Pf, C, U> PlaylistServiceImpl<Pf, C, U> {
    pub fn new(preferences: Pf, catalog: C, users: U) -> Self {
        Self { preferences, catalog, users }
    }
}

fn named(name: &str) -> Result<String, PlaylistError> {
    let name = name.trim();
    if name.is_empty() { Err(PlaylistError::EmptyName) } else { Ok(name.to_owned()) }
}

impl<Pf, C, U> PlaylistServiceImpl<Pf, C, U>
where
    Pf: PreferencesRepository + Sync,
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn existing(&self, user: &UserId, id: &PlaylistId) -> Result<Playlist, PlaylistError> {
        self.preferences.get_playlist(user, id).await?.ok_or(PlaylistError::NotFound)
    }

    async fn episodes(
        &self,
        ids: Vec<EpisodeId>,
        filter: &TitleListFilter,
        without_specials: bool,
    ) -> Result<Vec<TitleId>, RepositoryError> {
        let shown: HashSet<EpisodeId> = self
            .catalog
            .visible_episodes(&ids, filter)
            .await?
            .into_iter()
            .filter(|context| !(without_specials && context.season_number == SPECIALS))
            .map(|context| context.episode.id)
            .collect();
        Ok(ids.into_iter().filter(|id| shown.contains(id)).map(TitleId::Episode).collect())
    }

    async fn expand(
        &self,
        target: &WatchTarget,
        filter: &TitleListFilter,
    ) -> Result<Vec<TitleId>, RepositoryError> {
        match target {
            WatchTarget::Movie(id) => Ok(self
                .catalog
                .visible_movies(std::slice::from_ref(id), filter)
                .await?
                .into_iter()
                .map(|movie| TitleId::Movie(movie.id))
                .collect()),
            WatchTarget::Episode(id) => self.episodes(vec![id.clone()], filter, false).await,
            WatchTarget::Season(id) => {
                let mut found =
                    self.catalog.episode_ids_for_seasons(std::slice::from_ref(id)).await?;
                self.episodes(found.remove(id).unwrap_or_default(), filter, false).await
            }
            WatchTarget::Series(id) => {
                let mut found =
                    self.catalog.episode_ids_for_series(std::slice::from_ref(id)).await?;
                self.episodes(found.remove(id).unwrap_or_default(), filter, true).await
            }
        }
    }
}

impl<Pf, C, U> PlaylistService for PlaylistServiceImpl<Pf, C, U>
where
    Pf: PreferencesRepository + Sync,
    C: CatalogRepository + Sync,
    U: UserRepository + Sync,
{
    async fn playlists(&self, audience: &Audience) -> Result<Vec<PlaylistSummary>, PlaylistError> {
        let filter = acl::audience_filter(&self.users, audience).await?;
        let playlists = self.preferences.list_playlists(&audience.user).await?;
        let mut contents = Vec::with_capacity(playlists.len());
        for playlist in &playlists {
            contents
                .push(self.preferences.list_playlist_items(&audience.user, &playlist.id).await?);
        }
        let titles: Vec<TitleId> =
            contents.iter().flatten().map(|item| item.title.clone()).collect();
        let visible = visible_titles(&self.catalog, &titles, &filter).await?;
        Ok(playlists
            .into_iter()
            .zip(contents)
            .map(|(playlist, items)| PlaylistSummary {
                playlist,
                item_count: items.iter().filter(|item| visible.contains(&item.title)).count(),
            })
            .collect())
    }

    async fn create_playlist(&self, user: &UserId, name: &str) -> Result<Playlist, PlaylistError> {
        let now = Timestamp::now();
        let playlist = Playlist {
            id: PlaylistId(Uuid::new_v4().to_string()),
            name: named(name)?,
            created_at: now,
            updated_at: now,
        };
        self.preferences.save_playlist(user, playlist.clone()).await?;
        Ok(playlist)
    }

    async fn playlist(
        &self,
        audience: &Audience,
        id: &PlaylistId,
    ) -> Result<PlaylistDetail, PlaylistError> {
        let playlist = self.existing(&audience.user, id).await?;
        let filter = acl::audience_filter(&self.users, audience).await?;
        let items = self.preferences.list_playlist_items(&audience.user, id).await?;
        let titles: Vec<TitleId> = items.iter().map(|item| item.title.clone()).collect();
        let mut cards = visible_cards(&self.catalog, &titles, &filter).await?;
        let items = items
            .into_iter()
            .filter_map(|item| {
                let card = cards.remove(&item.title)?;
                Some(PlaylistCard { item, card })
            })
            .collect();
        Ok(PlaylistDetail { playlist, items })
    }

    async fn rename_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
        name: &str,
    ) -> Result<Playlist, PlaylistError> {
        let name = named(name)?;
        let playlist =
            Playlist { name, updated_at: Timestamp::now(), ..self.existing(user, id).await? };
        self.preferences.save_playlist(user, playlist.clone()).await?;
        Ok(playlist)
    }

    async fn delete_playlist(&self, user: &UserId, id: &PlaylistId) -> Result<(), PlaylistError> {
        if self.preferences.delete_playlist(user, id).await? {
            Ok(())
        } else {
            Err(PlaylistError::NotFound)
        }
    }

    async fn add_to_playlist(
        &self,
        audience: &Audience,
        id: &PlaylistId,
        target: &WatchTarget,
    ) -> Result<usize, PlaylistError> {
        self.existing(&audience.user, id).await?;
        let filter = acl::audience_filter(&self.users, audience).await?;
        let titles = self.expand(target, &filter).await?;
        if titles.is_empty() {
            return Err(PlaylistError::TitleNotFound);
        }
        let added = self
            .preferences
            .add_playlist_items(&audience.user, id, &titles, Timestamp::now())
            .await?;
        added.ok_or(PlaylistError::NotFound)
    }

    async fn remove_from_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
        title_id: &str,
    ) -> Result<(), PlaylistError> {
        self.existing(user, id).await?;
        self.preferences.remove_playlist_item(user, id, title_id, Timestamp::now()).await?;
        Ok(())
    }

    async fn move_in_playlist(
        &self,
        audience: &Audience,
        id: &PlaylistId,
        title_id: &str,
        position: u32,
    ) -> Result<(), PlaylistError> {
        let user = &audience.user;
        self.existing(user, id).await?;
        let filter = acl::audience_filter(&self.users, audience).await?;
        let stored: Vec<TitleId> = self
            .preferences
            .list_playlist_items(user, id)
            .await?
            .into_iter()
            .map(|item| item.title)
            .filter(|title| title.id() == title_id)
            .collect();
        if visible_titles(&self.catalog, &stored, &filter).await?.is_empty() {
            return Err(PlaylistError::TitleNotFound);
        }
        let moved = self
            .preferences
            .move_playlist_item(user, id, title_id, position, Timestamp::now())
            .await?;
        moved.then_some(()).ok_or(PlaylistError::TitleNotFound)
    }

    async fn queue(&self, audience: &Audience) -> Result<Vec<QueueCard>, PlaylistError> {
        let filter = acl::audience_filter(&self.users, audience).await?;
        let queue = self.preferences.list_queue(&audience.user).await?;
        let titles: Vec<TitleId> = queue.iter().map(|entry| entry.title.clone()).collect();
        let cards = visible_cards(&self.catalog, &titles, &filter).await?;
        Ok(queue
            .into_iter()
            .filter_map(|entry| {
                let card = cards.get(&entry.title)?.clone();
                Some(QueueCard { entry, card })
            })
            .collect())
    }

    async fn queue_playlist(
        &self,
        user: &UserId,
        id: &PlaylistId,
        shuffle: bool,
    ) -> Result<usize, PlaylistError> {
        self.existing(user, id).await?;
        Ok(self.preferences.queue_playlist(user, id, shuffle).await?)
    }

    async fn clear_queue(&self, user: &UserId) -> Result<(), PlaylistError> {
        Ok(self.preferences.clear_queue(user).await?)
    }

    async fn remove_from_queue(
        &self,
        user: &UserId,
        entry: &QueueEntryId,
    ) -> Result<(), PlaylistError> {
        Ok(self.preferences.remove_queue_entry(user, entry).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{
        Episode, Movie, MovieId, Season, SeasonId, Series, SeriesId, TitleCard, Version, VersionId,
    };
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::metadata::ContentRating;
    use domain::user::{Role, User};
    use mocks::{MockCatalogRepo, MockPreferencesRepo, MockUserRepo};

    type Svc = PlaylistServiceImpl<MockPreferencesRepo, MockCatalogRepo, MockUserRepo>;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn audience() -> Audience {
        Audience { user: user(), age_limit: None }
    }

    fn rating(code: &str) -> ContentRating {
        ContentRating { system: "mpaa".into(), code: code.into() }
    }

    fn account(cap: Option<&str>) -> User {
        User {
            id: user(),
            username: "u1".into(),
            password_hash: "hash".into(),
            role: Role::User,
            max_content_rating: cap.map(rating),
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn movie(id: &str, rated: &str) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: Some(rating(rated)),
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, number: u16) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId("s1".into()),
            number,
            title: None,
            overview: None,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn episode(id: &str, season: &str, number: u16) -> Episode {
        Episode {
            id: EpisodeId(id.into()),
            season: SeasonId(season.into()),
            number,
            title: id.into(),
            overview: None,
            runtime_minutes: None,
            air_date: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn version(title: TitleId, library: &str) -> Version {
        Version {
            id: VersionId(format!("v-{}", title.id())),
            title,
            library: LibraryId(library.into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: "/media/x.mkv".into(),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn catalog() -> MockCatalogRepo {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", "pg"));
        catalog.add_movie(movie("m2", "r"));
        catalog.add_movie(movie("m3", "pg"));
        catalog.add_series(Series {
            id: SeriesId("s1".into()),
            title: "s1".into(),
            sort_title: "s1".into(),
            year: None,
            overview: None,
            content_rating: Some(rating("pg")),
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_season(season("se2", 2));
        catalog.add_season(season("se9", 9));
        catalog.add_season(season("se1", 1));
        catalog.add_season(season("sp", 0));
        catalog.add_episode(episode("e21", "se2", 1));
        catalog.add_episode(episode("e12", "se1", 2));
        catalog.add_episode(episode("sp1", "sp", 1));
        catalog.add_episode(episode("e11", "se1", 1));
        for id in ["m1", "m2"] {
            catalog.add_version(version(TitleId::Movie(MovieId(id.into())), "lib1"));
        }
        catalog.add_version(version(TitleId::Movie(MovieId("m3".into())), "lib2"));
        for id in ["sp1", "e11", "e12", "e21"] {
            catalog.add_version(version(TitleId::Episode(EpisodeId(id.into())), "lib1"));
        }
        catalog
    }

    fn svc(preferences: &MockPreferencesRepo, cap: Option<&str>) -> Svc {
        let users = MockUserRepo::new();
        users.insert(account(cap));
        users.grant(&user(), &[LibraryId("lib1".into())]);
        PlaylistServiceImpl::new(preferences.clone(), catalog(), users)
    }

    fn card_id(card: &TitleCard) -> &str {
        match card {
            TitleCard::Movie(movie) => &movie.id.0,
            TitleCard::Episode(card) => &card.episode.id.0,
        }
    }

    async fn shown(svc: &Svc, id: &PlaylistId) -> Vec<String> {
        let detail = svc.playlist(&audience(), id).await.unwrap();
        detail.items.iter().map(|item| card_id(&item.card).to_owned()).collect()
    }

    async fn queued(svc: &Svc) -> Vec<String> {
        let queue = svc.queue(&audience()).await.unwrap();
        queue.iter().map(|entry| card_id(&entry.card).to_owned()).collect()
    }

    fn movie_target(id: &str) -> WatchTarget {
        WatchTarget::Movie(MovieId(id.into()))
    }

    #[tokio::test]
    async fn adding_a_series_takes_its_episodes_in_order_without_the_specials() {
        let preferences = MockPreferencesRepo::new();
        let svc = svc(&preferences, None);
        let id = svc.create_playlist(&user(), "Binge").await.unwrap().id;
        let series = WatchTarget::Series(SeriesId("s1".into()));

        assert_eq!(svc.add_to_playlist(&audience(), &id, &series).await.unwrap(), 3);
        assert_eq!(shown(&svc, &id).await, ["e11", "e12", "e21"]);

        let specials = WatchTarget::Season(SeasonId("sp".into()));
        assert_eq!(svc.add_to_playlist(&audience(), &id, &specials).await.unwrap(), 1);
        assert_eq!(svc.add_to_playlist(&audience(), &id, &series).await.unwrap(), 0);
        let episode = WatchTarget::Episode(EpisodeId("e12".into()));
        assert_eq!(svc.add_to_playlist(&audience(), &id, &episode).await.unwrap(), 0);
        assert_eq!(shown(&svc, &id).await, ["e11", "e12", "e21", "sp1"]);
        let positions: Vec<u32> = svc
            .playlist(&audience(), &id)
            .await
            .unwrap()
            .items
            .iter()
            .map(|item| item.item.position)
            .collect();
        assert_eq!(positions, [0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn a_playlist_leaves_out_what_the_viewer_cannot_see_and_keeps_it_stored() {
        let preferences = MockPreferencesRepo::new();
        let open = svc(&preferences, None);
        let id = open.create_playlist(&user(), "Mixed").await.unwrap().id;
        for target in [movie_target("m1"), movie_target("m2")] {
            open.add_to_playlist(&audience(), &id, &target).await.unwrap();
        }
        let capped = svc(&preferences, Some("pg-13"));

        assert_eq!(shown(&open, &id).await, ["m1", "m2"]);
        assert_eq!(shown(&capped, &id).await, ["m1"]);
        let child = Audience { user: user(), age_limit: Some(12) };
        assert_eq!(open.playlist(&child, &id).await.unwrap().items.len(), 1);
        assert_eq!(open.playlists(&audience()).await.unwrap()[0].item_count, 2);
        assert_eq!(capped.playlists(&audience()).await.unwrap()[0].item_count, 1);
        assert_eq!(preferences.list_playlist_items(&user(), &id).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn what_the_viewer_cannot_see_cannot_be_added() {
        let preferences = MockPreferencesRepo::new();
        let svc = svc(&preferences, Some("pg-13"));
        let id = svc.create_playlist(&user(), "Capped").await.unwrap().id;

        for target in [
            movie_target("m2"),
            movie_target("m3"),
            movie_target("ghost"),
            WatchTarget::Season(SeasonId("se9".into())),
            WatchTarget::Series(SeriesId("ghost".into())),
        ] {
            let refused = svc.add_to_playlist(&audience(), &id, &target).await;
            assert!(matches!(refused, Err(PlaylistError::TitleNotFound)), "{target:?}");
        }
        assert!(preferences.list_playlist_items(&user(), &id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_name_is_trimmed_and_cannot_be_blank() {
        let preferences = MockPreferencesRepo::new();
        let svc = svc(&preferences, None);

        let created = svc.create_playlist(&user(), "  Road trip ").await.unwrap();
        assert_eq!(created.name, "Road trip");
        assert!(matches!(svc.create_playlist(&user(), " ").await, Err(PlaylistError::EmptyName)));
        let blank = svc.rename_playlist(&user(), &created.id, "").await;
        assert!(matches!(blank, Err(PlaylistError::EmptyName)));

        let renamed = svc.rename_playlist(&user(), &created.id, "Sunday").await.unwrap();
        assert_eq!((renamed.name.as_str(), renamed.created_at), ("Sunday", created.created_at));
        let listed = svc.playlists(&audience()).await.unwrap();
        assert_eq!(listed[0].playlist.name, "Sunday");
    }

    #[tokio::test]
    async fn a_missing_playlist_is_not_found() {
        let svc = svc(&MockPreferencesRepo::new(), None);
        let ghost = PlaylistId("ghost".into());
        let missing = |result: Result<(), PlaylistError>| {
            assert!(matches!(result, Err(PlaylistError::NotFound)), "{result:?}");
        };

        missing(svc.playlist(&audience(), &ghost).await.map(drop));
        missing(svc.rename_playlist(&user(), &ghost, "x").await.map(drop));
        missing(svc.delete_playlist(&user(), &ghost).await);
        missing(svc.add_to_playlist(&audience(), &ghost, &movie_target("m1")).await.map(drop));
        missing(svc.remove_from_playlist(&user(), &ghost, "m1").await);
        missing(svc.move_in_playlist(&audience(), &ghost, "m1", 0).await);
        missing(svc.queue_playlist(&user(), &ghost, false).await.map(drop));
    }

    #[tokio::test]
    async fn items_move_and_leave_and_a_deleted_playlist_is_gone() {
        let preferences = MockPreferencesRepo::new();
        let svc = svc(&preferences, None);
        let id = svc.create_playlist(&user(), "Order").await.unwrap().id;
        let series = WatchTarget::Series(SeriesId("s1".into()));
        svc.add_to_playlist(&audience(), &id, &series).await.unwrap();

        svc.move_in_playlist(&audience(), &id, "e21", 0).await.unwrap();
        assert_eq!(shown(&svc, &id).await, ["e21", "e11", "e12"]);
        let absent = svc.move_in_playlist(&audience(), &id, "m1", 0).await;
        assert!(matches!(absent, Err(PlaylistError::TitleNotFound)));
        svc.remove_from_playlist(&user(), &id, "e11").await.unwrap();
        svc.remove_from_playlist(&user(), &id, "e11").await.unwrap();
        assert_eq!(shown(&svc, &id).await, ["e21", "e12"]);

        svc.delete_playlist(&user(), &id).await.unwrap();
        assert!(svc.playlists(&audience()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_viewer_moves_what_they_see_across_what_they_cannot() {
        let preferences = MockPreferencesRepo::new();
        let open = svc(&preferences, None);
        let id = open.create_playlist(&user(), "Mixed").await.unwrap().id;
        for target in
            [movie_target("m1"), movie_target("m2"), WatchTarget::Season(SeasonId("se1".into()))]
        {
            open.add_to_playlist(&audience(), &id, &target).await.unwrap();
        }
        let capped = svc(&preferences, Some("pg-13"));
        let positions = |detail: PlaylistDetail| -> Vec<(u32, String)> {
            detail
                .items
                .iter()
                .map(|item| (item.item.position, card_id(&item.card).to_owned()))
                .collect()
        };
        let seen = positions(capped.playlist(&audience(), &id).await.unwrap());
        assert_eq!(seen, [(0, "m1".to_owned()), (2, "e11".to_owned()), (3, "e12".to_owned())]);

        capped.move_in_playlist(&audience(), &id, "e12", 0).await.unwrap();
        assert_eq!(shown(&open, &id).await, ["e12", "m1", "m2", "e11"]);

        let hidden = capped.move_in_playlist(&audience(), &id, "m2", 0).await;
        assert!(matches!(hidden, Err(PlaylistError::TitleNotFound)), "{hidden:?}");
        let child = Audience { user: user(), age_limit: Some(12) };
        let limited = open.move_in_playlist(&child, &id, "m2", 0).await;
        assert!(matches!(limited, Err(PlaylistError::TitleNotFound)), "{limited:?}");
        let ungranted = TitleId::Movie(MovieId("m3".into()));
        preferences
            .add_playlist_items(&user(), &id, &[ungranted], Timestamp::UNIX_EPOCH)
            .await
            .unwrap();
        let outside = open.move_in_playlist(&audience(), &id, "m3", 0).await;
        assert!(matches!(outside, Err(PlaylistError::TitleNotFound)), "{outside:?}");
        assert_eq!(shown(&open, &id).await, ["e12", "m1", "m2", "e11"]);
        let stored = preferences.list_playlist_items(&user(), &id).await.unwrap();
        assert_eq!(stored.last().map(|item| item.title.id()), Some("m3"));
    }

    #[tokio::test]
    async fn the_queue_appends_behind_what_it_holds_and_hides_what_the_viewer_cannot_see() {
        let preferences = MockPreferencesRepo::new();
        let open = svc(&preferences, None);
        let id = open.create_playlist(&user(), "Night").await.unwrap().id;
        for target in [movie_target("m1"), movie_target("m2")] {
            open.add_to_playlist(&audience(), &id, &target).await.unwrap();
        }

        assert_eq!(open.queue_playlist(&user(), &id, false).await.unwrap(), 2);
        assert_eq!(open.queue_playlist(&user(), &id, false).await.unwrap(), 2);
        assert_eq!(open.queue_playlist(&user(), &id, true).await.unwrap(), 2);
        assert_eq!(queued(&open).await, ["m1", "m2", "m1", "m2", "m2", "m1"]);
        let capped = svc(&preferences, Some("pg-13"));
        assert_eq!(queued(&capped).await, ["m1", "m1", "m1"]);

        let first = open.queue(&audience()).await.unwrap()[0].entry.id.clone();
        open.remove_from_queue(&user(), &first).await.unwrap();
        assert_eq!(queued(&open).await, ["m2", "m1", "m2", "m2", "m1"]);
        open.clear_queue(&user()).await.unwrap();
        assert!(queued(&open).await.is_empty());
    }
}
