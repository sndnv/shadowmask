use jiff::Timestamp;

use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionId,
};
use domain::common::Quality;
use domain::discovery::{ContinueWatchingItem, Hub, HubItem, SearchResult};
use domain::library::{
    DuplicateCandidate, DuplicateCandidateId, Library, LibraryId, LibraryKind, MatchCandidate,
    UnmatchedFile, UnmatchedFileId, WatcherStrategy,
};
use domain::metadata::{ContentRating, Person, PersonId};
use domain::playback::{PlaybackProgress, WatchHistory};
use domain::user::{IssuedToken, Role, UserId};
use services::mock::{
    MockAuthService, MockCatalogService, MockDiscoveryService, MockLibraryService,
    MockSessionService, MockUserLibraryService, MockUserService,
};

const EPOCH: i64 = 1_700_000_000;

fn ts(offset: i64) -> Timestamp {
    Timestamp::from_second(EPOCH + offset).expect("valid fixture timestamp")
}

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
        self.catalog.add_collection(Collection {
            id: CollectionId("c1".into()),
            name: "Saga".into(),
            overview: Some("epic".into()),
            movies: vec![MovieId("m1".into())],
        });
        for (vid, quality) in [
            ("v1", Quality::Sd),
            ("v2", Quality::Hd),
            ("v3", Quality::Fhd),
            ("v4", Quality::Uhd),
        ] {
            self.catalog.add_version(version(
                vid,
                TitleId::Movie(MovieId("m1".into())),
                "lib1",
                quality,
            ));
        }
        self.catalog.add_version(version(
            "ev1",
            TitleId::Episode(EpisodeId("e1".into())),
            "lib1",
            Quality::Hd,
        ));
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
    auth.add_account("admin", "pw", UserId("admin".into()), Role::Admin);
    auth.add_account("user", "pw", UserId("u1".into()), Role::User);
    auth.add_link_code(
        "CODE",
        IssuedToken {
            token: "player-token".into(),
            expires_at: None,
        },
    );
    auth
}

fn movie(id: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: format!("Alpha {id}"),
        year: Some(2020),
        overview: Some("overview".into()),
        runtime_minutes: Some(100),
        content_rating: Some(ContentRating {
            system: "MPAA".into(),
            code: "PG-13".into(),
        }),
        added_at: ts(1),
    }
}

fn series(id: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: format!("Alpha {id}"),
        year: Some(2019),
        overview: None,
        content_rating: Some(ContentRating {
            system: "TV".into(),
            code: "TV-14".into(),
        }),
        added_at: ts(2),
    }
}

fn season(id: &str, series: &str) -> Season {
    Season {
        id: SeasonId(id.into()),
        series: SeriesId(series.into()),
        number: 1,
        title: Some("Season 1".into()),
        overview: None,
    }
}

fn episode(id: &str, season: &str) -> Episode {
    Episode {
        id: EpisodeId(id.into()),
        season: SeasonId(season.into()),
        number: 1,
        title: format!("Alpha {id}"),
        overview: None,
        runtime_minutes: Some(42),
        air_date: Some(ts(3)),
        added_at: ts(4),
    }
}

fn version(id: &str, title: TitleId, lib: &str, quality: Quality) -> Version {
    Version {
        id: VersionId(id.into()),
        title,
        library: LibraryId(lib.into()),
        quality,
        container: "mkv".into(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1000,
        edition: None,
    }
}

fn library(id: &str) -> Library {
    Library {
        id: LibraryId(id.into()),
        name: format!("Lib {id}"),
        kind: LibraryKind::Movie,
        roots: vec!["/media".into()],
        watcher: WatcherStrategy::Manual,
        scan_schedule: Some("0 0 * * *".into()),
        metadata_sources: vec!["tmdb".into()],
    }
}
