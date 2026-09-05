use std::sync::Arc;

use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::discovery::SearchResult;
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, LibraryId, MatchCandidate, UnmatchedFile,
    UnmatchedFileId,
};
use domain::metadata::{Person, PersonId};
use domain::playback::{PlaybackProgress, WatchHistory, WatchlistItem};
use domain::user::{User, UserId};
use jiff::Timestamp;
use mocks::{
    MockAuthService, MockAuthTokenRepo, MockCatalogRepo, MockJobStore, MockLibraryRepo,
    MockMetadataProvider, MockPreferencesRepo, MockProgressRepo, MockSearchIndex,
    MockSessionService, MockUserDataStore, MockUserRepo,
};
use services::catalog::CatalogServiceImpl;
use services::discovery::DiscoveryServiceImpl;
use services::job::JobServiceImpl;
use services::library::LibraryServiceImpl;
use services::user::UserServiceImpl;
use services::user_library::UserLibraryServiceImpl;

use crate::fixture::{
    self, collection_art, episode, episode_art, library, movie, season_art, series, ts,
};

pub struct Generator {
    pub auth: MockAuthService,
    pub catalog_repo: MockCatalogRepo,
    pub users_repo: MockUserRepo,
    pub catalog: CatalogServiceImpl<MockCatalogRepo, MockUserRepo>,
    pub session: MockSessionService,
    pub library_repo: MockLibraryRepo,
    pub jobs_repo: MockJobStore,
    pub library: LibraryServiceImpl<
        MockLibraryRepo,
        MockUserRepo,
        MockJobStore,
        MockCatalogRepo,
        MockMetadataProvider,
    >,
    pub progress_repo: MockProgressRepo,
    pub preferences_repo: MockPreferencesRepo,
    pub user: UserServiceImpl<MockUserRepo, MockAuthTokenRepo, MockUserDataStore>,
    pub user_library:
        UserLibraryServiceImpl<MockProgressRepo, MockPreferencesRepo, MockCatalogRepo>,
    pub search_index: MockSearchIndex,
    pub discovery: DiscoveryServiceImpl<
        MockCatalogRepo,
        MockSearchIndex,
        MockProgressRepo,
        MockPreferencesRepo,
        MockUserRepo,
    >,
    pub job: JobServiceImpl<MockJobStore>,
}

impl Default for Generator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator {
    pub fn new() -> Self {
        Self::from_auth(seeded_auth())
    }

    pub fn from_auth(auth: MockAuthService) -> Self {
        let catalog_repo = MockCatalogRepo::new();
        let users_repo = MockUserRepo::new();
        let library_repo = MockLibraryRepo::new();
        let jobs_repo = MockJobStore::new();
        let progress_repo = MockProgressRepo::new();
        let preferences_repo = MockPreferencesRepo::new();
        let search_index = MockSearchIndex::new();
        let generator = Self {
            auth,
            catalog: CatalogServiceImpl::new(catalog_repo.clone(), users_repo.clone()),
            library: LibraryServiceImpl::new(
                library_repo.clone(),
                users_repo.clone(),
                jobs_repo.clone(),
                catalog_repo.clone(),
                None::<MockMetadataProvider>,
            ),
            user: UserServiceImpl::new(
                users_repo.clone(),
                MockAuthTokenRepo::new(),
                MockUserDataStore::new(),
            ),
            user_library: UserLibraryServiceImpl::new(
                Arc::new(progress_repo.clone()),
                Arc::new(preferences_repo.clone()),
                Arc::new(catalog_repo.clone()),
            ),
            job: JobServiceImpl::new(jobs_repo.clone()),
            discovery: DiscoveryServiceImpl::new(
                catalog_repo.clone(),
                search_index.clone(),
                progress_repo.clone(),
                preferences_repo.clone(),
                users_repo.clone(),
            ),
            catalog_repo,
            users_repo,
            library_repo,
            jobs_repo,
            progress_repo,
            preferences_repo,
            search_index,
            session: MockSessionService::new(),
        };
        generator.generate_catalog();
        generator.generate_library();
        generator.generate_users();
        generator.generate_user_library();
        generator.generate_discovery();
        generator
    }

    fn generate_users(&self) {
        for account in fixture::accounts() {
            let row = User {
                id: account.user_id.clone(),
                username: account.username.to_owned(),
                password_hash: account.password.to_owned(),
                role: account.role,
                max_content_rating: None,
                preferred_audio: Vec::new(),
                preferred_subtitle: Vec::new(),
                concurrent_stream_limit: None,
                bitrate_cap: None,
                active: true,
                created_at: ts(0),
                updated_at: ts(0),
            };
            self.users_repo.insert(row);
            self.users_repo
                .grant(&account.user_id, &[LibraryId("lib1".into())]);
        }
    }

    fn generate_catalog(&self) {
        self.catalog_repo.seed_movie_detail(fixture::movie_detail());
        self.catalog_repo
            .seed_series_detail(fixture::series_detail_aggregate());
        self.catalog_repo.add_season(season_art("se1", "s1"));
        self.catalog_repo.add_episode(episode_art("e1", "se1"));
        self.catalog_repo.add_collection(collection_art());
        for person in fixture::people() {
            self.catalog_repo.add_person(person);
        }
        for version in fixture::catalog_versions() {
            self.catalog_repo.add_version(version);
        }
        self.catalog_repo
            .seed_version_detail(fixture::version_detail("v1"));
    }

    fn generate_library(&self) {
        let id = LibraryId("lib1".into());
        self.library_repo.insert_library(library("lib1"));
        self.library_repo.add_unmatched(UnmatchedFile {
            id: UnmatchedFileId("uf1".into()),
            library: id.clone(),
            path: "/media/x.mkv".into(),
            candidates: vec![MatchCandidate {
                title: TitleId::Movie(MovieId("m1".into())),
                confidence: 0.9,
                label: "Alpha".into(),
            }],
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        });
        self.library_repo.add_duplicate(
            &id,
            DuplicateCandidate {
                id: DuplicateCandidateId("d1".into()),
                title: TitleId::Episode(EpisodeId("e1".into())),
                paths: vec!["/a.mkv".into(), "/b.mkv".into()],
            },
        );
        self.jobs_repo.seed(fixture::admin_job());
        self.jobs_repo.seed(fixture::admin_child_job());
    }

    fn generate_user_library(&self) {
        let u1 = UserId("u1".into());
        self.progress_repo.seed_history(WatchHistory {
            user: u1.clone(),
            title: TitleId::Movie(MovieId("m1".into())),
            watched: true,
            play_count: 2,
            last_watched_at: Some(ts(10)),
            completed: false,
        });
        // 30% of the fixture version's 1000ms runtime: past the 5% start floor
        // and short of the 90% completion mark.
        self.progress_repo.seed_progress(PlaybackProgress {
            user: u1,
            version: VersionId("v1".into()),
            position_ms: 300,
            audio_track: None,
            subtitle: None,
            updated_at: ts(20),
        });
    }

    fn generate_discovery(&self) {
        let u1 = UserId("u1".into());
        self.search_index.add(SearchResult::Movie(movie("m1")));
        self.search_index.add(SearchResult::Series(series("s1")));
        self.search_index
            .add(SearchResult::Episode(episode("e1", "se1")));
        self.search_index.add(SearchResult::Person(Person {
            id: PersonId("p1".into()),
            name: "Alpha Person".into(),
            ..Person::default()
        }));
        // The real service derives every hub row from the catalog, progress and
        // watchlist, so the watchlist row only appears if something is saved.
        self.preferences_repo.seed_watchlist(WatchlistItem {
            user: u1.clone(),
            title: TitleId::Movie(MovieId("m2".into())),
            added_at: ts(40),
        });
        self.preferences_repo.seed_watchlist(WatchlistItem {
            user: u1,
            title: TitleId::Episode(EpisodeId("e1".into())),
            added_at: ts(41),
        });
    }
}

fn seeded_auth() -> MockAuthService {
    let auth = MockAuthService::new();
    for account in fixture::accounts() {
        auth.add_account(
            account.username,
            account.password,
            account.user_id,
            account.role,
        );
    }
    auth.add_link_code(fixture::LINK_CODE, fixture::link_token());
    auth
}
