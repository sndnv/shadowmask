use std::collections::HashMap;

use domain::catalog::{
    ArtworkOwner, Episode, EpisodeId, MovieId, Season, SeasonId, SeriesId, TitleId, TitleOutline,
    Version, VersionCounts,
};
use domain::common::Quality;
use domain::library::LibraryId;
use domain::repository::CatalogRepository;

use crate::fixture;

const NEAR: &str = "outline-near";
const FAR: &str = "outline-far";

fn movie(id: &str) -> TitleId {
    TitleId::Movie(MovieId(id.into()))
}

fn episode(id: &str) -> TitleId {
    TitleId::Episode(EpisodeId(id.into()))
}

fn version(id: &str, title: TitleId, library: &str, available: bool) -> Version {
    Version { available, ..fixture::version(id, title, library, Quality::Hd) }
}

async fn seed<R: CatalogRepository>(repo: &R) {
    repo.upsert_movie(fixture::movie_art("om-held")).await.unwrap();
    for id in ["om-gone", "om-empty", "om-far"] {
        repo.upsert_movie(fixture::movie(id)).await.unwrap();
    }
    for (owner, prefix) in [
        (ArtworkOwner::Movie(MovieId("om-held".into())), "oa-movie"),
        (ArtworkOwner::Series(SeriesId("os-show".into())), "oa-series"),
        (ArtworkOwner::Season(SeasonId("os-one".into())), "oa-season"),
        (ArtworkOwner::Episode(EpisodeId("oe-held".into())), "oa-episode"),
    ] {
        repo.set_artwork(&owner, &fixture::artwork_set(prefix)).await.unwrap();
    }
    repo.upsert_series(fixture::series_art("os-show")).await.unwrap();
    repo.upsert_series(fixture::series("os-bare")).await.unwrap();
    repo.upsert_season(fixture::season_art("os-one", "os-show")).await.unwrap();
    repo.upsert_season(Season { number: 2, ..fixture::season("os-two", "os-show") }).await.unwrap();
    repo.upsert_episode(fixture::episode_art("oe-held", "os-one")).await.unwrap();
    repo.upsert_episode(Episode { number: 2, ..fixture::episode("oe-empty", "os-one") })
        .await
        .unwrap();
    for held in [
        version("ov-near", movie("om-held"), NEAR, true),
        version("ov-far", movie("om-held"), FAR, true),
        version("ov-gone", movie("om-gone"), NEAR, false),
        version("ov-only-far", movie("om-far"), FAR, false),
        version("ov-episode", episode("oe-held"), NEAR, true),
    ] {
        repo.upsert_version(held).await.unwrap();
    }
}

fn ids<T>(items: &[T], id: impl Fn(&T) -> &str) -> Vec<String> {
    let mut found: Vec<String> = items.iter().map(|item| id(item).to_owned()).collect();
    found.sort();
    found
}

fn counts(total: u32, granted: u32, present: u32) -> VersionCounts {
    VersionCounts { total, granted, present }
}

async fn outline<R: CatalogRepository>(repo: &R, libraries: &[&str]) -> TitleOutline {
    let libraries: Vec<LibraryId> =
        libraries.iter().map(|library| LibraryId((*library).into())).collect();
    repo.title_outline(&libraries).await.unwrap()
}

pub async fn title_outline_contract<R: CatalogRepository>(repo: R) {
    let empty = outline(&repo, &[NEAR]).await;
    assert!(empty.movies.is_empty() && empty.series.is_empty() && empty.versions.is_empty());
    assert!(empty.seasons.is_empty() && empty.episodes.is_empty());

    seed(&repo).await;

    let near = outline(&repo, &[NEAR]).await;
    assert_eq!(
        ids(&near.movies, |movie| movie.id.0.as_str()),
        ["om-empty", "om-far", "om-gone", "om-held"],
        "every movie is outlined, with or without a version"
    );
    assert_eq!(ids(&near.series, |series| series.id.0.as_str()), ["os-bare", "os-show"]);
    assert_eq!(ids(&near.seasons, |season| season.id.0.as_str()), ["os-one", "os-two"]);
    assert_eq!(ids(&near.episodes, |episode| episode.id.0.as_str()), ["oe-empty", "oe-held"]);
    assert!(
        near.movies.iter().all(|movie| movie.artwork.is_empty())
            && near.series.iter().all(|series| series.artwork.is_empty())
            && near.seasons.iter().all(|season| season.artwork.is_empty())
            && near.episodes.iter().all(|episode| episode.artwork.is_empty()),
        "the outline reads no artwork"
    );

    let held = near.movies.iter().find(|movie| movie.id.0 == "om-held").unwrap();
    assert_eq!(
        (held.title.as_str(), held.sort_title.as_str(), held.year, held.added_at),
        ("Alpha om-held", "alpha om-held", Some(2020), fixture::ts(1))
    );
    let season = near.seasons.iter().find(|season| season.id.0 == "os-one").unwrap();
    assert_eq!((&season.series, season.number), (&SeriesId("os-show".into()), 1));
    let found = near.episodes.iter().find(|episode| episode.id.0 == "oe-held").unwrap();
    assert_eq!((&found.season, found.number), (&SeasonId("os-one".into()), 1));
    assert_eq!(found.title, "Alpha oe-held");

    assert_eq!(
        near.versions,
        HashMap::from([
            (movie("om-held"), counts(2, 1, 1)),
            (movie("om-gone"), counts(1, 1, 0)),
            (movie("om-far"), counts(1, 0, 0)),
            (episode("oe-held"), counts(1, 1, 1)),
        ]),
        "a title with no version has no counts; granted and present count only the given libraries"
    );

    let both = outline(&repo, &[NEAR, FAR]).await;
    assert_eq!(both.versions[&movie("om-held")], counts(2, 2, 2));
    assert_eq!(both.versions[&movie("om-far")], counts(1, 1, 0));

    let none = outline(&repo, &[]).await;
    assert_eq!(none.versions[&movie("om-held")], counts(2, 0, 0));
    assert_eq!(none.versions[&episode("oe-held")], counts(1, 0, 0));
}
