use std::collections::HashMap;

use domain::catalog::{
    EpisodeCard, EpisodeContext, EpisodeId, Movie, MovieId, Series, SeriesId, SortOrder, TitleId,
    TitleListFilter, TitleSort,
};
use domain::common::{Page, PageRequest};
use domain::discovery::{ContinueWatchingItem, Hub, HubItem, SearchKind, SearchResult};
use domain::error::{DiscoveryError, RepositoryError};
use domain::playback::{WatchlistItem, is_started};
use domain::repository::{
    CatalogRepository, PreferencesRepository, ProgressRepository, SearchIndex, UserRepository,
};
use domain::service::DiscoveryService;
use domain::session::{NowPlaying, PlaybackSession};
use domain::user::UserId;

use crate::acl;
use crate::discovery::{
    EPISODE_WINDOW, RECENT_ROW, continue_watching, furthest_watched, home_hubs,
    next_movie_candidates, next_movies, recently_added_movies, recently_added_shows, resume_card,
    resume_card_from, watched_episode_ids, watched_movie_ids, watchlist_row,
};

#[derive(Clone)]
pub struct DiscoveryServiceImpl<C, Se, Pr, Pf, U> {
    catalog: C,
    search_index: Se,
    progress: Pr,
    preferences: Pf,
    users: U,
}

impl<C, Se, Pr, Pf, U> DiscoveryServiceImpl<C, Se, Pr, Pf, U> {
    pub fn new(catalog: C, search_index: Se, progress: Pr, preferences: Pf, users: U) -> Self {
        Self {
            catalog,
            search_index,
            progress,
            preferences,
            users,
        }
    }
}

impl<C, Se, Pr, Pf, U> DiscoveryServiceImpl<C, Se, Pr, Pf, U>
where
    C: CatalogRepository + Sync,
    Se: SearchIndex + Sync,
    Pr: ProgressRepository + Sync,
    Pf: PreferencesRepository + Sync,
    U: UserRepository + Sync,
{
    async fn viewer_filter(&self, user: &UserId) -> Result<TitleListFilter, RepositoryError> {
        Ok(acl::viewer(&self.users, user).await?.filter(None))
    }

    async fn recent_movies(&self, filter: &TitleListFilter) -> Result<Vec<Movie>, RepositoryError> {
        let newest = TitleListFilter {
            sort: TitleSort::AddedAt,
            order: SortOrder::Desc,
            ..filter.clone()
        };
        Ok(self
            .catalog
            .list_movies_filtered(
                &newest,
                PageRequest {
                    offset: 0,
                    limit: RECENT_ROW as u32,
                },
            )
            .await?
            .items)
    }

    async fn recent_shows(
        &self,
        filter: &TitleListFilter,
    ) -> Result<Vec<HubItem>, RepositoryError> {
        let window = self.catalog.recent_episodes(filter, EPISODE_WINDOW).await?;
        let mut row = Vec::new();
        for (series, episodes) in recently_added_shows(&window, RECENT_ROW) {
            if let Some(series) = self.catalog.get_series(&series).await? {
                row.push(HubItem::Series {
                    series,
                    episode_count: Some(episodes),
                });
            }
        }
        Ok(row)
    }

    async fn cards_for(
        &self,
        contexts: Vec<EpisodeContext>,
    ) -> Result<Vec<EpisodeCard>, RepositoryError> {
        let mut parents: HashMap<SeriesId, Option<Series>> = HashMap::new();
        let mut cards = Vec::with_capacity(contexts.len());
        for context in contexts {
            let parent = match parents.get(&context.series) {
                Some(found) => found.clone(),
                None => {
                    let found = self.catalog.get_series(&context.series).await?;
                    parents.insert(context.series.clone(), found.clone());
                    found
                }
            };
            cards.push(EpisodeCard {
                episode: context.episode,
                series: Some(context.series),
                series_title: parent.as_ref().map(|s| s.title.clone()),
                series_artwork: parent.map(|s| s.artwork).unwrap_or_default(),
                season_number: Some(context.season_number),
                season_title: context.season_title,
            });
        }
        Ok(cards)
    }

    async fn up_next(
        &self,
        user: &UserId,
        filter: &TitleListFilter,
    ) -> Result<Vec<EpisodeCard>, RepositoryError> {
        let history = self.progress.watched_state(user).await?;
        let watched: Vec<EpisodeId> = watched_episode_ids(&history).into_iter().collect();
        let seen = self.catalog.visible_episodes(&watched, filter).await?;
        let mut found = Vec::new();
        for (series, season, number) in furthest_watched(&seen) {
            if let Some(next) = self
                .catalog
                .next_episode_in_series(&series, season, number, filter)
                .await?
            {
                found.push(next);
            }
        }
        self.cards_for(found).await
    }

    async fn on_deck(
        &self,
        user: &UserId,
        filter: &TitleListFilter,
    ) -> Result<Vec<Movie>, RepositoryError> {
        let collections = self.catalog.list_collections(PageRequest::ALL).await?.items;
        let history = self.progress.watched_state(user).await?;
        let watched = watched_movie_ids(&history);
        let candidates = next_movie_candidates(&collections, &watched);
        let movies = self.catalog.visible_movies(&candidates, filter).await?;
        Ok(next_movies(&collections, &movies, &watched))
    }

    async fn watchlist(
        &self,
        saved: &[WatchlistItem],
        filter: &TitleListFilter,
    ) -> Result<Vec<HubItem>, RepositoryError> {
        let mut movie_ids = Vec::new();
        let mut episode_ids = Vec::new();
        for item in saved {
            match &item.title {
                TitleId::Movie(id) => movie_ids.push(id.clone()),
                TitleId::Episode(id) => episode_ids.push(id.clone()),
            }
        }
        let movies = self.catalog.visible_movies(&movie_ids, filter).await?;
        let contexts = self.catalog.visible_episodes(&episode_ids, filter).await?;
        let cards = self.cards_for(contexts).await?;
        Ok(watchlist_row(saved, &movies, &cards, RECENT_ROW))
    }

    async fn continue_items(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, RepositoryError> {
        let progress = self.progress.list_in_progress(user).await?;
        let mut items = Vec::new();
        for entry in continue_watching(&progress) {
            let Some(played) = self.catalog.get_version(&entry.version).await? else {
                continue;
            };
            if !is_started(entry.position_ms, played.duration_ms) {
                continue;
            }
            let card = resume_card_from(&self.catalog, played, entry.position_ms).await?;
            items.push(ContinueWatchingItem {
                progress: entry,
                card,
            });
        }
        Ok(items)
    }
}

impl<C, Se, Pr, Pf, U> DiscoveryService for DiscoveryServiceImpl<C, Se, Pr, Pf, U>
where
    C: CatalogRepository + Sync,
    Se: SearchIndex + Sync,
    Pr: ProgressRepository + Sync,
    Pf: PreferencesRepository + Sync,
    U: UserRepository + Sync,
{
    async fn search(
        &self,
        user: &UserId,
        query: &str,
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, DiscoveryError> {
        let filter = self.viewer_filter(user).await?;
        Ok(self
            .search_index
            .search(query, types, &filter, page)
            .await?)
    }

    async fn continue_watching(
        &self,
        user: &UserId,
    ) -> Result<Vec<ContinueWatchingItem>, DiscoveryError> {
        Ok(self.continue_items(user).await?)
    }

    async fn now_playing(
        &self,
        sessions: Vec<PlaybackSession>,
    ) -> Result<Vec<NowPlaying>, DiscoveryError> {
        let mut items = Vec::with_capacity(sessions.len());
        for session in sessions {
            if session.completed {
                continue;
            }
            if let Some(card) =
                resume_card(&self.catalog, &session.version, session.position_ms).await?
            {
                items.push(NowPlaying { session, card });
            }
        }
        Ok(items)
    }

    async fn next_episodes(&self, user: &UserId) -> Result<Vec<EpisodeCard>, DiscoveryError> {
        let filter = self.viewer_filter(user).await?;
        Ok(self.up_next(user, &filter).await?)
    }

    async fn next_movies(&self, user: &UserId) -> Result<Vec<Movie>, DiscoveryError> {
        let filter = self.viewer_filter(user).await?;
        Ok(self.on_deck(user, &filter).await?)
    }

    async fn home_hubs(&self, user: &UserId) -> Result<Vec<Hub>, DiscoveryError> {
        let filter = self.viewer_filter(user).await?;
        let saved = self.preferences.list_watchlist(user).await?;

        let watchlist = self.watchlist(&saved, &filter).await?;
        let recent_movies = recently_added_movies(&self.recent_movies(&filter).await?, RECENT_ROW);
        let recent_shows = self.recent_shows(&filter).await?;
        let on_deck: Vec<HubItem> = self
            .on_deck(user, &filter)
            .await?
            .into_iter()
            .map(HubItem::Movie)
            .collect();

        let started: Vec<MovieId> = self
            .continue_items(user)
            .await?
            .into_iter()
            .filter_map(|item| match item.card.title {
                TitleId::Movie(id) => Some(id),
                TitleId::Episode(_) => None,
            })
            .collect();
        let visible = self.catalog.visible_movies(&started, &filter).await?;
        let by_id: HashMap<&MovieId, &Movie> = visible.iter().map(|m| (&m.id, m)).collect();
        let continue_row: Vec<HubItem> = started
            .iter()
            .filter_map(|id| by_id.get(id).map(|m| HubItem::Movie((*m).clone())))
            .collect();

        Ok(home_hubs(
            watchlist,
            recent_movies,
            recent_shows,
            on_deck,
            continue_row,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::catalog::{
        Collection, CollectionId, Episode, MovieId, Season, SeasonId, SeriesId, Version,
    };
    use domain::catalog::{EpisodeId, VersionId};
    use domain::common::Quality;
    use domain::library::LibraryId;
    use domain::metadata::ContentRating;
    use domain::playback::{PlaybackProgress, WatchHistory, WatchlistItem};
    use domain::session::{
        DeliveryMode, PlaybackSession, PlaybackState, SelectedTracks, SessionId,
    };
    use domain::user::{Role, User};
    use jiff::{SignedDuration, Timestamp};
    use mocks::{
        MockCatalogRepo, MockPreferencesRepo, MockProgressRepo, MockSearchIndex, MockUserRepo,
    };

    type Svc = DiscoveryServiceImpl<
        MockCatalogRepo,
        MockSearchIndex,
        MockProgressRepo,
        MockPreferencesRepo,
        MockUserRepo,
    >;

    fn user() -> UserId {
        UserId("u1".into())
    }

    fn account(role: Role, cap: Option<ContentRating>) -> User {
        User {
            id: user(),
            username: "u1".into(),
            password_hash: "hash".into(),
            role,
            max_content_rating: cap,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: None,
            bitrate_cap: None,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn admins() -> MockUserRepo {
        let users = MockUserRepo::new();
        users.insert(account(Role::Admin, None));
        users
    }

    async fn viewer(cap: Option<&str>, libraries: &[&str]) -> MockUserRepo {
        let users = MockUserRepo::new();
        users.insert(account(
            Role::User,
            cap.map(|code| ContentRating {
                system: "mpaa".into(),
                code: code.into(),
            }),
        ));
        let granted: Vec<LibraryId> = libraries.iter().map(|l| LibraryId((*l).into())).collect();
        users.set_library_access(&user(), &granted).await.unwrap();
        users
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
    }

    fn movie(id: &str, seconds: i64) -> Movie {
        Movie {
            id: MovieId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
            updated_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
            artwork: Vec::new(),
        }
    }

    fn series(id: &str) -> Series {
        Series {
            id: SeriesId(id.into()),
            title: id.into(),
            sort_title: id.into(),
            year: None,
            overview: None,
            content_rating: None,
            manually_edited: false,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        }
    }

    fn season(id: &str, series: &str) -> Season {
        Season {
            id: SeasonId(id.into()),
            series: SeriesId(series.into()),
            number: 1,
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

    fn version(id: &str, title: TitleId) -> Version {
        Version {
            id: VersionId(id.into()),
            title,
            library: LibraryId("lib1".into()),
            quality: Quality::Hd,
            container: "mkv".into(),
            path: format!("/media/{id}.mkv"),
            size_bytes: 1,
            duration_ms: 1000,
            available: true,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn history(title: TitleId, watched: bool, completed: bool) -> WatchHistory {
        WatchHistory {
            user: user(),
            title,
            watched,
            play_count: 1,
            last_watched_at: None,
            completed,
        }
    }

    async fn seeded() -> Svc {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_movie(movie("m2", 20));
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));
        catalog.add_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: None,
            movies: vec![MovieId("m1".into()), MovieId("m2".into())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        catalog.add_version(version("ev1", TitleId::Episode(EpisodeId("e1".into()))));

        let search = MockSearchIndex::new();
        search.add(SearchResult::Movie(movie("m1", 10)));

        let progress = MockProgressRepo::new();
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        progress.seed_history(history(TitleId::Movie(MovieId("m1".into())), true, false));
        progress.seed_history(history(
            TitleId::Episode(EpisodeId("e1".into())),
            true,
            false,
        ));

        DiscoveryServiceImpl::new(
            catalog,
            search,
            progress,
            MockPreferencesRepo::new(),
            admins(),
        )
    }

    #[tokio::test]
    async fn search_delegates_to_index() {
        let svc = seeded().await;
        let hits = svc.search(&user(), "m1", &[], page()).await.unwrap();
        assert_eq!(hits.total, 1);
    }

    fn session(id: &str, version: &str, position_ms: u64) -> PlaybackSession {
        PlaybackSession {
            id: SessionId(id.into()),
            user: user(),
            device: None,
            version: VersionId(version.into()),
            mode: DeliveryMode::Direct,
            position_ms,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::UNIX_EPOCH,
            last_heartbeat_at: Timestamp::UNIX_EPOCH,
            completed: false,
        }
    }

    #[tokio::test]
    async fn the_continue_rail_never_loads_a_full_version_detail() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        let progress = MockProgressRepo::new();
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 500,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let svc = DiscoveryServiceImpl::new(
            catalog.clone(),
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );

        let items = svc.continue_watching(&user()).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            catalog.detail_lookup_count(),
            0,
            "a resume card needs duration and title, both columns on the version row"
        );
    }

    #[tokio::test]
    async fn continue_watching_enriches_with_card() {
        let svc = seeded().await;
        let items = svc.continue_watching(&user()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].progress.version, VersionId("v1".into()));
        assert_eq!(items[0].card.display_title, "m1");
        assert_eq!(items[0].card.duration_ms, 1000);
        assert_eq!(items[0].card.progress_percent, 100);
    }

    #[tokio::test]
    async fn now_playing_enriches_and_skips_dangling_versions() {
        let svc = seeded().await;
        let now = svc
            .now_playing(vec![session("s1", "v1", 500), session("s2", "ghost", 10)])
            .await
            .unwrap();
        assert_eq!(now.len(), 1);
        assert_eq!(now[0].session.id, SessionId("s1".into()));
        assert_eq!(now[0].card.display_title, "m1");
        assert_eq!(now[0].card.duration_ms, 1000);
        assert_eq!(now[0].card.progress_percent, 50);
    }

    #[tokio::test]
    async fn a_session_that_finished_its_title_is_no_longer_playing() {
        let svc = seeded().await;
        let finished = PlaybackSession {
            completed: true,
            ..session("s1", "v1", 1000)
        };

        let now = svc
            .now_playing(vec![finished, session("s2", "v1", 500)])
            .await
            .unwrap();

        assert_eq!(
            now.len(),
            1,
            "a watched title must not sit in the continue rail until the reaper runs"
        );
        assert_eq!(now[0].session.id, SessionId("s2".into()));
    }

    #[tokio::test]
    async fn continue_watching_lists_a_replay_of_a_completed_title() {
        let progress = MockProgressRepo::new();
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 1234,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        progress.seed_history(history(TitleId::Movie(MovieId("m1".into())), true, true));
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );

        let items = svc.continue_watching(&user()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].progress.version, VersionId("v1".into()));
    }

    #[tokio::test]
    async fn continue_watching_drops_a_stored_row_below_the_resume_floor() {
        let progress = MockProgressRepo::new();
        progress
            .upsert(PlaybackProgress {
                user: user(),
                version: VersionId("v1".into()),
                position_ms: 30_000,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        let mut feature = version("v1", TitleId::Movie(MovieId("m1".into())));
        feature.duration_ms = 7_200_000;
        catalog.add_version(feature);
        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );

        assert!(
            svc.continue_watching(&user()).await.unwrap().is_empty(),
            "rows stored before the floor existed must not surface either"
        );
    }

    #[tokio::test]
    async fn continue_watching_excludes_titles_without_a_resume_point() {
        let progress = MockProgressRepo::new();
        progress.seed_history(history(TitleId::Movie(MovieId("m1".into())), true, true));
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );

        assert!(svc.continue_watching(&user()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn next_episodes_and_movies() {
        let svc = seeded().await;
        let next_eps = svc.next_episodes(&user()).await.unwrap();
        assert_eq!(next_eps.len(), 1);
        assert_eq!(next_eps[0].episode.id, EpisodeId("e2".into()));
        assert_eq!(next_eps[0].series, Some(SeriesId("s1".into())));
        assert_eq!(next_eps[0].season_number, Some(1));

        let next_films = svc.next_movies(&user()).await.unwrap();
        assert_eq!(next_films.len(), 1);
        assert_eq!(next_films[0].id, MovieId("m2".into()));
    }

    #[tokio::test]
    async fn two_saved_episodes_of_one_series_share_a_single_parent_lookup() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));

        let preferences = MockPreferencesRepo::new();
        for (id, seconds) in [("e1", 20), ("e2", 10)] {
            preferences
                .add_watchlist(WatchlistItem {
                    user: user(),
                    title: TitleId::Episode(EpisodeId(id.into())),
                    added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
                })
                .await
                .unwrap();
        }

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            preferences,
            admins(),
        );
        let hubs = svc.home_hubs(&user()).await.unwrap();

        match &hubs[0].items[..] {
            [HubItem::Episode(first), HubItem::Episode(second)] => {
                assert_eq!(first.episode.id, EpisodeId("e1".into()));
                assert_eq!(second.episode.id, EpisodeId("e2".into()));
                assert_eq!(
                    first.series_title, second.series_title,
                    "the second card is built from the cached parent, so it must not come \
                     out different from the first"
                );
                assert_eq!(first.series_title.as_deref(), Some("s1"));
            }
            other => panic!("unexpected watchlist row: {other:?}"),
        }
    }

    #[tokio::test]
    async fn up_next_never_offers_an_episode_with_no_playable_version() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));
        catalog.add_version(version("ev1", TitleId::Episode(EpisodeId("e1".into()))));

        let progress = MockProgressRepo::new();
        progress.seed_history(history(
            TitleId::Episode(EpisodeId("e1".into())),
            true,
            false,
        ));

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            viewer(None, &["lib1"]).await,
        );

        assert!(
            svc.next_episodes(&user()).await.unwrap().is_empty(),
            "e2 has no version, so the card would be a dead end; up next is gated the \
             same way the on deck movie rail already was"
        );
    }

    #[tokio::test]
    async fn marking_watched_without_playing_still_drives_up_next() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_movie(movie("m2", 20));
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));
        catalog.add_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: None,
            movies: vec![MovieId("m1".into()), MovieId("m2".into())],
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
            artwork: Vec::new(),
        });

        let progress = MockProgressRepo::new();
        for title in [
            TitleId::Episode(EpisodeId("e1".into())),
            TitleId::Movie(MovieId("m1".into())),
        ] {
            progress
                .set_watched_flags(&user(), &title, true)
                .await
                .unwrap();
        }
        assert!(
            progress
                .history(&user(), PageRequest::ALL)
                .await
                .unwrap()
                .items
                .is_empty(),
            "the fixture must be the hand-marked case, which history() excludes"
        );

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );

        let next_eps = svc.next_episodes(&user()).await.unwrap();
        assert_eq!(next_eps.len(), 1);
        assert_eq!(next_eps[0].episode.id, EpisodeId("e2".into()));

        let next_films = svc.next_movies(&user()).await.unwrap();
        assert_eq!(next_films.len(), 1);
        assert_eq!(next_films[0].id, MovieId("m2".into()));

        let hubs = svc.home_hubs(&user()).await.unwrap();
        let on_deck = hubs.iter().find(|h| h.id == "on_deck").unwrap();
        match &on_deck.items[..] {
            [HubItem::Movie(m)] => assert_eq!(m.id, MovieId("m2".into())),
            other => panic!("unexpected on deck row: {other:?}"),
        }
    }

    #[tokio::test]
    async fn home_hubs_composes_rows() {
        let svc = seeded().await;
        let hubs = svc.home_hubs(&user()).await.unwrap();
        let ids: Vec<&str> = hubs.iter().map(|h| h.id.as_str()).collect();
        assert!(ids.contains(&"recently_added_movies"));
        assert!(ids.contains(&"recently_added_shows"));
        assert!(ids.contains(&"continue_watching"));

        let shows = hubs
            .iter()
            .find(|h| h.id == "recently_added_shows")
            .unwrap();
        match &shows.items[..] {
            [
                HubItem::Series {
                    series,
                    episode_count,
                },
            ] => {
                assert_eq!(series.id, SeriesId("s1".into()));
                assert_eq!(*episode_count, Some(2));
            }
            other => panic!("unexpected shows row: {other:?}"),
        }
    }

    #[tokio::test]
    async fn an_empty_watchlist_leaves_no_rail_behind() {
        let svc = seeded().await;
        let hubs = svc.home_hubs(&user()).await.unwrap();
        assert!(hubs.iter().all(|h| h.id != "watchlist"));
    }

    #[tokio::test]
    async fn the_watchlist_rail_leads_and_keeps_one_card_per_saved_row() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("m1", 10));
        catalog.add_series(series("s1"));
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));

        let preferences = MockPreferencesRepo::new();
        for (title, seconds) in [
            (TitleId::Movie(MovieId("m1".into())), 10),
            (TitleId::Episode(EpisodeId("e1".into())), 20),
        ] {
            preferences
                .add_watchlist(WatchlistItem {
                    user: user(),
                    title,
                    added_at: Timestamp::UNIX_EPOCH + SignedDuration::from_secs(seconds),
                })
                .await
                .unwrap();
        }

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            preferences,
            admins(),
        );
        let hubs = svc.home_hubs(&user()).await.unwrap();

        assert_eq!(
            hubs[0].id, "watchlist",
            "the rail sits above recently added"
        );
        match &hubs[0].items[..] {
            [HubItem::Episode(card), HubItem::Movie(m)] => {
                assert_eq!(card.episode.id, EpisodeId("e1".into()));
                assert_eq!(
                    card.series,
                    Some(SeriesId("s1".into())),
                    "the card needs its parent to show a series poster"
                );
                assert_eq!(m.id, MovieId("m1".into()));
            }
            other => panic!("unexpected watchlist row: {other:?}"),
        }
    }

    #[tokio::test]
    async fn home_hubs_continue_row_keeps_movies_and_skips_the_rest() {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(movie("mm", 0));
        catalog.add_version(version("vm", TitleId::Movie(MovieId("mm".into()))));
        catalog.add_version(version("vmiss", TitleId::Movie(MovieId("gone".into()))));
        catalog.add_version(version("vep", TitleId::Episode(EpisodeId("e1".into()))));

        let progress = MockProgressRepo::new();
        for id in ["vm", "vmiss", "vep", "vnone"] {
            progress
                .upsert(PlaybackProgress {
                    user: user(),
                    version: VersionId(id.into()),
                    position_ms: 100,
                    audio_track: None,
                    subtitle: None,
                    updated_at: Timestamp::UNIX_EPOCH,
                })
                .await
                .unwrap();
        }

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            progress,
            MockPreferencesRepo::new(),
            admins(),
        );
        let hubs = svc.home_hubs(&user()).await.unwrap();
        let continue_row = hubs
            .iter()
            .find(|hub| hub.id == "continue_watching")
            .expect("continue_watching row present");
        assert_eq!(continue_row.items.len(), 1);
        assert!(matches!(
            &continue_row.items[0],
            HubItem::Movie(movie) if movie.id == MovieId("mm".into())
        ));
    }

    fn rated_movie(id: &str, seconds: i64, code: &str) -> Movie {
        Movie {
            content_rating: Some(ContentRating {
                system: "mpaa".into(),
                code: code.into(),
            }),
            ..movie(id, seconds)
        }
    }

    fn gated() -> MockCatalogRepo {
        let catalog = MockCatalogRepo::new();
        catalog.add_movie(rated_movie("m1", 10, "pg"));
        catalog.add_movie(rated_movie("m2", 20, "r"));
        catalog.add_version(version("v1", TitleId::Movie(MovieId("m1".into()))));
        catalog.add_version(version("v2", TitleId::Movie(MovieId("m2".into()))));
        catalog
    }

    fn hub_movie_ids(hubs: &[Hub], id: &str) -> Vec<String> {
        hubs.iter()
            .find(|h| h.id == id)
            .map(|hub| {
                hub.items
                    .iter()
                    .filter_map(|item| match item {
                        HubItem::Movie(m) => Some(m.id.0.clone()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn a_capped_viewer_never_sees_a_blocked_movie_on_home() {
        let svc = DiscoveryServiceImpl::new(
            gated(),
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            viewer(Some("pg"), &["lib1"]).await,
        );

        let hubs = svc.home_hubs(&user()).await.unwrap();

        assert_eq!(
            hub_movie_ids(&hubs, "recently_added_movies"),
            vec!["m1".to_owned()],
            "an 18 rated title must not reach a capped account through the home rails"
        );
    }

    #[tokio::test]
    async fn an_admin_home_is_not_gated() {
        let svc = DiscoveryServiceImpl::new(
            gated(),
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            admins(),
        );

        let hubs = svc.home_hubs(&user()).await.unwrap();

        assert_eq!(hub_movie_ids(&hubs, "recently_added_movies").len(), 2);
    }

    #[tokio::test]
    async fn a_viewer_granted_no_library_gets_an_empty_home() {
        let svc = DiscoveryServiceImpl::new(
            gated(),
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            viewer(None, &[]).await,
        );

        assert!(
            svc.home_hubs(&user()).await.unwrap().is_empty(),
            "no library access means nothing to discover, not everything"
        );
    }

    #[tokio::test]
    async fn a_capped_viewer_loses_the_whole_tree_under_a_blocked_series() {
        let catalog = MockCatalogRepo::new();
        catalog.add_series(Series {
            content_rating: Some(ContentRating {
                system: "mpaa".into(),
                code: "r".into(),
            }),
            ..series("s1")
        });
        catalog.add_season(season("se1", "s1"));
        catalog.add_episode(episode("e1", "se1", 1));
        catalog.add_episode(episode("e2", "se1", 2));
        catalog.add_version(version("ev1", TitleId::Episode(EpisodeId("e1".into()))));
        catalog.add_version(version("ev2", TitleId::Episode(EpisodeId("e2".into()))));

        let preferences = MockPreferencesRepo::new();
        preferences
            .add_watchlist(WatchlistItem {
                user: user(),
                title: TitleId::Episode(EpisodeId("e1".into())),
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let svc = DiscoveryServiceImpl::new(
            catalog,
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            preferences,
            viewer(Some("pg"), &["lib1"]).await,
        );

        let hubs = svc.home_hubs(&user()).await.unwrap();

        assert!(
            hubs.is_empty(),
            "an episode inherits its rating from the series, so a blocked show takes its episodes with it"
        );
        assert!(
            svc.next_episodes(&user()).await.unwrap().is_empty(),
            "up next must not route around the cap either"
        );
    }

    #[tokio::test]
    async fn search_hands_the_viewers_gates_to_the_index() {
        let index = MockSearchIndex::new();
        let svc = DiscoveryServiceImpl::new(
            MockCatalogRepo::new(),
            index.clone(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            viewer(Some("pg"), &["lib1"]).await,
        );

        svc.search(&user(), "anything", &[], page()).await.unwrap();

        let filter = index.last_filter().expect("the index was asked to search");
        assert_eq!(
            filter.libraries,
            Some(vec![LibraryId("lib1".into())]),
            "search has to carry the same library scope the catalog lists use"
        );
        assert!(
            filter
                .blocked_ratings
                .iter()
                .any(|rating| rating.code.eq_ignore_ascii_case("r")),
            "the cap must reach the index, not be applied after paging"
        );
    }

    #[tokio::test]
    async fn an_admin_search_carries_no_gates() {
        let index = MockSearchIndex::new();
        let svc = DiscoveryServiceImpl::new(
            MockCatalogRepo::new(),
            index.clone(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            admins(),
        );

        svc.search(&user(), "anything", &[], page()).await.unwrap();

        let filter = index.last_filter().expect("the index was asked to search");
        assert!(filter.libraries.is_none());
        assert!(filter.blocked_ratings.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_account_is_shown_nothing() {
        let svc = DiscoveryServiceImpl::new(
            gated(),
            MockSearchIndex::new(),
            MockProgressRepo::new(),
            MockPreferencesRepo::new(),
            MockUserRepo::new(),
        );

        assert!(
            svc.home_hubs(&user()).await.unwrap().is_empty(),
            "a missing account must fail closed, never open"
        );
    }

    #[tokio::test]
    async fn backend_errors_propagate() {
        let catalog = MockCatalogRepo::new();
        catalog.set_fail();
        let progress = MockProgressRepo::new();
        progress.set_fail();
        let search = MockSearchIndex::new();
        search.set_fail();
        let preferences = MockPreferencesRepo::new();
        preferences.set_fail();
        let users = MockUserRepo::new();
        users.set_fail();
        let svc = DiscoveryServiceImpl::new(catalog, search, progress, preferences, users);
        assert!(svc.search(&user(), "x", &[], page()).await.is_err());
        assert!(svc.continue_watching(&user()).await.is_err());
        assert!(svc.now_playing(vec![session("s1", "v1", 1)]).await.is_err());
        assert!(svc.next_episodes(&user()).await.is_err());
        assert!(svc.next_movies(&user()).await.is_err());
        assert!(svc.home_hubs(&user()).await.is_err());
    }
}
