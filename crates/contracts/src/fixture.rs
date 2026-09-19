use jiff::Timestamp;

use domain::catalog::{
    ArtworkId, ArtworkRef, ArtworkWidth, Collection, CollectionId, Episode, EpisodeCard, EpisodeId,
    Movie, MovieDetail, MovieId, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleId,
    TitleRef, Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, Quality};
use domain::job::{Job, JobId, JobKind, JobPriority, JobStatus};
use domain::library::{Library, LibraryId, LibraryKind, LibraryOrigin, WatcherStrategy};
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFormat, TrickplayAsset, VideoTrack,
};
use domain::metadata::{
    ArtworkKind, ContentRating, Credit, CreditRole, CreditedPerson, ExternalId, Extra, ExtraKind,
    Genre, GenreId, Person, PersonId, Rating, Studio, StudioId, TitleEnrichment,
};
use domain::user::{DeviceId, IssuedToken, Role, UserId};

pub const EPOCH: i64 = 1_700_000_000;

pub const LINK_CODE: &str = "7G2K9QMP";

pub fn ts(offset: i64) -> Timestamp {
    Timestamp::from_second(EPOCH + offset).expect("valid fixture timestamp")
}

pub fn admin_job() -> Job {
    Job {
        id: JobId("job-scan".into()),
        kind: JobKind::LibraryScan,
        status: JobStatus::Succeeded,
        priority: JobPriority::Normal,
        payload: "lib1".into(),
        attempts: 1,
        progress: 1.0,
        available_at: ts(40),
        last_error: None,
        created_at: ts(40),
        updated_at: ts(41),
        started_at: Some(ts(40)),
        finished_at: Some(ts(41)),
        parent_id: None,
    }
}

pub fn admin_child_job() -> Job {
    Job {
        id: JobId("job-artwork".into()),
        kind: JobKind::Artwork,
        status: JobStatus::Queued,
        priority: JobPriority::Normal,
        payload: "m1".into(),
        attempts: 0,
        progress: 0.0,
        available_at: ts(42),
        last_error: None,
        created_at: ts(42),
        updated_at: ts(42),
        started_at: None,
        finished_at: None,
        parent_id: Some(JobId("job-scan".into())),
    }
}

pub fn movie(id: &str) -> Movie {
    Movie {
        id: MovieId(id.into()),
        title: format!("Alpha {id}"),
        sort_title: format!("alpha {id}"),
        year: Some(2020),
        overview: Some("overview".into()),
        runtime_minutes: Some(100),
        content_rating: Some(ContentRating { system: "MPAA".into(), code: "PG-13".into() }),
        manually_edited: false,
        added_at: ts(1),
        updated_at: ts(1),
        artwork: Vec::new(),
    }
}

pub fn series(id: &str) -> Series {
    Series {
        id: SeriesId(id.into()),
        title: format!("Alpha {id}"),
        sort_title: format!("alpha {id}"),
        year: Some(2019),
        overview: None,
        content_rating: Some(ContentRating { system: "TV".into(), code: "TV-14".into() }),
        manually_edited: false,
        added_at: ts(2),
        updated_at: ts(2),
        artwork: Vec::new(),
    }
}

pub fn season(id: &str, series: &str) -> Season {
    Season {
        id: SeasonId(id.into()),
        series: SeriesId(series.into()),
        number: 1,
        title: Some("Season 1".into()),
        overview: None,
        added_at: ts(5),
        updated_at: ts(5),
        artwork: Vec::new(),
    }
}

pub fn episode(id: &str, season: &str) -> Episode {
    Episode {
        id: EpisodeId(id.into()),
        season: SeasonId(season.into()),
        number: 1,
        title: format!("Alpha {id}"),
        overview: None,
        runtime_minutes: Some(42),
        air_date: Some(ts(3)),
        manually_edited: false,
        added_at: ts(4),
        updated_at: ts(4),
        artwork: Vec::new(),
    }
}

pub fn version(id: &str, title: TitleId, lib: &str, quality: Quality) -> Version {
    Version {
        id: VersionId(id.into()),
        title,
        library: LibraryId(lib.into()),
        quality,
        container: "mkv".into(),
        path: format!("/media/{id}.mkv"),
        size_bytes: 1,
        duration_ms: 1000,
        available: true,
        added_at: ts(6),
        updated_at: ts(6),
    }
}

pub fn library(id: &str) -> Library {
    Library {
        id: LibraryId(id.into()),
        name: format!("Lib {id}"),
        origin: LibraryOrigin::Local,
        kind: LibraryKind::Movie,
        roots: vec!["/media".into()],
        sort_articles: vec!["the".into()],
        watcher: WatcherStrategy::Manual,
        scan_schedule: Some("0 0 * * *".into()),
        metadata_sources: vec!["tmdb".into()],
        created_at: ts(8),
        updated_at: ts(8),
    }
}

pub fn saga_collection() -> Collection {
    Collection {
        id: CollectionId("c1".into()),
        name: "Saga".into(),
        overview: Some("epic".into()),
        movies: vec![MovieId("m1".into())],
        added_at: ts(7),
        updated_at: ts(7),
        artwork: Vec::new(),
    }
}

pub fn movie_art(id: &str) -> Movie {
    Movie { artwork: artwork_set(id), ..movie(id) }
}

pub fn series_art(id: &str) -> Series {
    Series { artwork: artwork_set(id), ..series(id) }
}

pub fn episode_card(id: &str, season_id: &str, series_id: &str) -> EpisodeCard {
    let parent = series_art(series_id);
    let within = season(season_id, series_id);
    EpisodeCard {
        series: Some(parent.id),
        series_title: Some(parent.title),
        series_artwork: parent.artwork,
        season_number: Some(within.number),
        season_title: within.title,
        ..EpisodeCard::bare(episode(id, season_id))
    }
}

pub fn artwork_set(owner_id: &str) -> Vec<ArtworkRef> {
    vec![
        ArtworkRef {
            id: ArtworkId(format!("{owner_id}-poster")),
            kind: ArtworkKind::Poster,
            widths: vec![
                ArtworkWidth::new(180, format!("/art/{owner_id}-poster/180.jpg")),
                ArtworkWidth::new(480, format!("/art/{owner_id}-poster/480.jpg")),
                ArtworkWidth::new(960, format!("/art/{owner_id}-poster/960.jpg")),
            ],
        },
        ArtworkRef {
            id: ArtworkId(format!("{owner_id}-backdrop")),
            kind: ArtworkKind::Backdrop,
            widths: vec![
                ArtworkWidth::new(480, format!("/art/{owner_id}-backdrop/480.jpg")),
                ArtworkWidth::new(960, format!("/art/{owner_id}-backdrop/960.jpg")),
            ],
        },
    ]
}

pub fn season_art(id: &str, series: &str) -> Season {
    Season { artwork: artwork_set(id), ..season(id, series) }
}

pub fn episode_art(id: &str, season: &str) -> Episode {
    Episode { artwork: artwork_set(id), ..episode(id, season) }
}

pub fn collection_art() -> Collection {
    Collection { artwork: artwork_set("c1"), ..saga_collection() }
}

pub fn catalog_versions() -> Vec<Version> {
    let mut versions: Vec<Version> =
        [("v1", Quality::Sd), ("v2", Quality::Hd), ("v3", Quality::Fhd), ("v4", Quality::Uhd)]
            .into_iter()
            .map(|(vid, quality)| {
                version(vid, TitleId::Movie(MovieId("m1".into())), "lib1", quality)
            })
            .collect();
    versions.push(version("ev1", TitleId::Episode(EpisodeId("e1".into())), "lib1", Quality::Hd));
    versions
}

pub fn version_detail(id: &str) -> VersionDetail {
    let vid = VersionId(id.into());
    VersionDetail {
        version: version(id, TitleId::Movie(MovieId("m1".into())), "lib1", Quality::Sd),
        video: vec![VideoTrack {
            index: 0,
            codec: "hevc".into(),
            width: 3840,
            height: 2160,
            bit_depth: 10,
            hdr: Some(HdrFormat::DolbyVision),
            frame_rate: 24.0,
            bitrate: Some(48_000_000),
        }],
        audio: vec![
            AudioTrack {
                index: 1,
                codec: "eac3".into(),
                channels: 6,
                language: Some(LanguageCode("en".into())),
                bitrate: Some(768_000),
            },
            AudioTrack {
                index: 2,
                codec: "aac".into(),
                channels: 2,
                language: Some(LanguageCode("fr".into())),
                bitrate: None,
            },
        ],
        subtitles: vec![
            EmbeddedSubtitleTrack {
                index: 3,
                language: Some(LanguageCode("en".into())),
                format: SubtitleFormat::Srt,
                forced: false,
                default: true,
            },
            EmbeddedSubtitleTrack {
                index: 4,
                language: Some(LanguageCode("es".into())),
                format: SubtitleFormat::Pgs,
                forced: true,
                default: false,
            },
        ],
        subtitle_files: Vec::new(),
        chapters: vec![
            Chapter { title: "Cold Open".into(), start_ms: 0 },
            Chapter { title: "Main Title".into(), start_ms: 60_000 },
        ],
        markers: DetectedMarkers {
            intros: vec![IntroMarker { version: vid.clone(), start_ms: 60_000, end_ms: 90_000 }],
            credits: vec![CreditsMarker {
                version: vid.clone(),
                start_ms: 900_000,
                end_ms: 960_000,
            }],
        },
        trickplay: vec![TrickplayAsset {
            version: vid,
            interval_ms: 10_000,
            columns: 5,
            rows: 5,
            tile_width: 320,
            tile_height: 180,
            sheet_paths: vec!["sheet-000.jpg".into(), "sheet-001.jpg".into()],
        }],
    }
}

pub fn people() -> Vec<Person> {
    vec![
        Person { id: PersonId("p1".into()), name: "Ada Lovelace".into(), ..Person::default() },
        Person { id: PersonId("p2".into()), name: "Bob Director".into(), ..Person::default() },
    ]
}

fn genre(id: &str, name: &str) -> Genre {
    Genre { id: GenreId(id.into()), name: name.into() }
}

fn credited(credits: &[Credit]) -> Vec<CreditedPerson> {
    let people = people();
    credits
        .iter()
        .filter_map(|credit| {
            let person = people.iter().find(|p| p.id == credit.person)?.clone();
            Some(CreditedPerson {
                person,
                role: credit.role,
                character: credit.character.clone(),
                order: credit.order,
            })
        })
        .collect()
}

pub fn movie_enrichment() -> TitleEnrichment {
    let m1 = TitleRef::Movie(MovieId("m1".into()));
    TitleEnrichment {
        genres: vec![genre("g-action", "Action"), genre("g-drama", "Drama")],
        credits: vec![
            Credit {
                person: PersonId("p1".into()),
                title: m1.clone(),
                role: CreditRole::Actor,
                character: Some("Hero".into()),
                order: 0,
            },
            Credit {
                person: PersonId("p2".into()),
                title: m1,
                role: CreditRole::Director,
                character: None,
                order: 1,
            },
        ],
        studios: vec![Studio { id: StudioId("st-acme".into()), name: "Acme Studios".into() }],
        ratings: vec![Rating { source: "tmdb".into(), value: 8.5 }],
        external_ids: vec![
            ExternalId { source: "tmdb".into(), value: "603".into() },
            ExternalId { source: "imdb".into(), value: "tt0133093".into() },
        ],
        extras: vec![Extra {
            kind: ExtraKind::Trailer,
            title: "Teaser".into(),
            path: "/extras/m1/teaser.mkv".into(),
        }],
    }
}

pub fn series_enrichment() -> TitleEnrichment {
    TitleEnrichment {
        genres: vec![genre("g-action", "Action")],
        credits: vec![Credit {
            person: PersonId("p1".into()),
            title: TitleRef::Series(SeriesId("s1".into())),
            role: CreditRole::Actor,
            character: Some("Lead".into()),
            order: 0,
        }],
        ratings: vec![Rating { source: "tmdb".into(), value: 9.0 }],
        external_ids: vec![ExternalId { source: "tvdb".into(), value: "81189".into() }],
        ..TitleEnrichment::default()
    }
}

pub fn movie_detail() -> MovieDetail {
    let enrichment = movie_enrichment();
    MovieDetail {
        movie: movie_art("m1"),
        genres: enrichment.genres,
        credits: credited(&enrichment.credits),
        studios: enrichment.studios,
        ratings: enrichment.ratings,
        external_ids: enrichment.external_ids,
        extras: enrichment.extras,
    }
}

pub fn series_detail_aggregate() -> SeriesDetail {
    let enrichment = series_enrichment();
    SeriesDetail {
        series: series_art("s1"),
        genres: enrichment.genres,
        credits: credited(&enrichment.credits),
        studios: enrichment.studios,
        ratings: enrichment.ratings,
        external_ids: enrichment.external_ids,
        extras: enrichment.extras,
        episodes_total: 0,
        episodes_with_available_version: 0,
    }
}

pub struct SeedAccount {
    pub username: &'static str,
    pub password: &'static str,
    pub user_id: UserId,
    pub role: Role,
}

pub fn accounts() -> Vec<SeedAccount> {
    vec![
        SeedAccount {
            username: "admin",
            password: "pw",
            user_id: UserId("admin".into()),
            role: Role::Admin,
        },
        SeedAccount {
            username: "user",
            password: "pw",
            user_id: UserId("u1".into()),
            role: Role::User,
        },
    ]
}

pub fn link_token() -> IssuedToken {
    IssuedToken {
        token: "player-token".into(),
        expires_at: None,
        device: DeviceId("device-1".into()),
    }
}
