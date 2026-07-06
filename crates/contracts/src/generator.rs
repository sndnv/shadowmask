use domain::catalog::{EpisodeId, MovieId, TitleId, VersionId};
use domain::discovery::{ContinueWatchingItem, Hub, HubItem, SearchResult};
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, LibraryId, MatchCandidate, UnmatchedFile,
    UnmatchedFileId,
};
use domain::metadata::{Person, PersonId};
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::user::UserId;
use services::mock::{
    MockAuthService, MockCatalogService, MockDiscoveryService, MockLibraryService,
    MockSessionService, MockUserLibraryService, MockUserService,
};

use crate::fixture::{self, episode, library, movie, saga_collection, season, series, ts};

pub struct Generator {
    pub auth: MockAuthService,
    pub catalog: MockCatalogService,
    pub session: MockSessionService,
    pub library: MockLibraryService,
    pub user: MockUserService,
    pub user_library: MockUserLibraryService,
    pub discovery: MockDiscoveryService,
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
        let generator = Self {
            auth,
            catalog: MockCatalogService::new(),
            session: MockSessionService::new(),
            library: MockLibraryService::new(),
            user: MockUserService::new(),
            user_library: MockUserLibraryService::new(),
            discovery: MockDiscoveryService::new(),
        };
        generator.generate_catalog();
        generator.generate_library();
        generator.generate_user_library();
        generator.generate_discovery();
        generator
    }

    fn generate_catalog(&self) {
        self.catalog.add_movie(movie("m1"));
        self.catalog.add_series(series("s1"));
        self.catalog.add_season(season("se1", "s1"));
        self.catalog.add_episode(episode("e1", "se1"));
        self.catalog.add_collection(saga_collection());
        for version in fixture::catalog_versions() {
            self.catalog.add_version(version);
        }
    }

    fn generate_library(&self) {
        let id = LibraryId("lib1".into());
        self.library.add_library(library("lib1"));
        self.library.add_unmatched(
            &id,
            UnmatchedFile {
                id: UnmatchedFileId("uf1".into()),
                library: id.clone(),
                path: "/media/x.mkv".into(),
                candidates: vec![MatchCandidate {
                    title: TitleId::Movie(MovieId("m1".into())),
                    confidence: 0.9,
                    label: "Alpha".into(),
                }],
            },
        );
        self.library.add_duplicate(
            &id,
            DuplicateCandidate {
                id: DuplicateCandidateId("d1".into()),
                title: TitleId::Episode(EpisodeId("e1".into())),
                paths: vec!["/a.mkv".into(), "/b.mkv".into()],
            },
        );
    }

    fn generate_user_library(&self) {
        let u1 = UserId("u1".into());
        self.user_library.add_history(WatchHistory {
            user: u1.clone(),
            title: TitleId::Movie(MovieId("m1".into())),
            watched: true,
            play_count: 2,
            last_watched_at: Some(ts(10)),
            completed: false,
        });
        self.user_library.set_progress(PlaybackProgress {
            user: u1,
            version: VersionId("v1".into()),
            position_ms: 1234,
            updated_at: ts(20),
        });
    }

    fn generate_discovery(&self) {
        let u1 = UserId("u1".into());
        self.discovery
            .add_search_result(SearchResult::Movie(movie("m1")));
        self.discovery
            .add_search_result(SearchResult::Series(series("s1")));
        self.discovery
            .add_search_result(SearchResult::Episode(episode("e1", "se1")));
        self.discovery
            .add_search_result(SearchResult::Person(Person {
                id: PersonId("p1".into()),
                name: "Alpha Person".into(),
            }));
        self.discovery.add_continue_watching(
            &u1,
            ContinueWatchingItem {
                progress: PlaybackProgress {
                    user: u1.clone(),
                    version: VersionId("v1".into()),
                    position_ms: 10,
                    updated_at: ts(30),
                },
            },
        );
        self.discovery.add_next_episode(&u1, episode("e1", "se1"));
        self.discovery.add_next_movie(&u1, movie("m2"));
        self.discovery.add_hub(Hub {
            id: "recent".into(),
            title: "Recently Added".into(),
            items: vec![HubItem::Movie(movie("m1")), HubItem::Series(series("s1"))],
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
