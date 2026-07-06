use jiff::Timestamp;

use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionId,
};
use domain::common::Quality;
use domain::library::{Library, LibraryId, LibraryKind, WatcherStrategy};
use domain::metadata::ContentRating;
use domain::user::{IssuedToken, Role, UserId};

pub const EPOCH: i64 = 1_700_000_000;

pub const LINK_CODE: &str = "CODE";

pub fn ts(offset: i64) -> Timestamp {
    Timestamp::from_second(EPOCH + offset).expect("valid fixture timestamp")
}

pub fn movie(id: &str) -> Movie {
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

pub fn series(id: &str) -> Series {
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

pub fn season(id: &str, series: &str) -> Season {
    Season {
        id: SeasonId(id.into()),
        series: SeriesId(series.into()),
        number: 1,
        title: Some("Season 1".into()),
        overview: None,
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
        added_at: ts(4),
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
        edition: None,
    }
}

pub fn library(id: &str) -> Library {
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

pub fn saga_collection() -> Collection {
    Collection {
        id: CollectionId("c1".into()),
        name: "Saga".into(),
        overview: Some("epic".into()),
        movies: vec![MovieId("m1".into())],
    }
}

pub fn catalog_versions() -> Vec<Version> {
    let mut versions: Vec<Version> = [
        ("v1", Quality::Sd),
        ("v2", Quality::Hd),
        ("v3", Quality::Fhd),
        ("v4", Quality::Uhd),
    ]
    .into_iter()
    .map(|(vid, quality)| version(vid, TitleId::Movie(MovieId("m1".into())), "lib1", quality))
    .collect();
    versions.push(version(
        "ev1",
        TitleId::Episode(EpisodeId("e1".into())),
        "lib1",
        Quality::Hd,
    ));
    versions
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
    }
}
