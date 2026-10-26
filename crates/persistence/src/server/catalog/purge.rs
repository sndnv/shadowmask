use std::collections::HashSet;

use domain::catalog::{
    CatalogClean, CleanScope, EpisodeId, LibraryFootprint, LibraryPurge, MovieId, SeasonId,
    SeriesId, TitleId, TitleSet, VersionId,
};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::SubtitleSource;
use sqlx::sqlite::SqliteRow;
use sqlx::{AssertSqlSafe, SqliteConnection, SqlitePool};

use super::{BIND_CHUNK, subtitle_source_to_str};
use crate::codec::title_from_parts;
use crate::pool::{backend, column};

const TITLE_ROW_CLEANUP: [&str; 8] = [
    "DELETE FROM artwork WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM credits WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM title_genres WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM title_studios WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM title_ratings WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM title_external_ids WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM title_extras WHERE title_kind = ? AND title_id IN ({ids})",
    "DELETE FROM search_index WHERE kind = ? AND id IN ({ids})",
];

const UNSHARED_VERSIONS_SQL: &str = "SELECT v.id, v.title_kind, v.title_id FROM versions v \
     WHERE v.library_id = ? AND NOT EXISTS (SELECT 1 FROM versions o \
     WHERE o.title_kind = v.title_kind AND o.title_id = v.title_id \
     AND o.library_id <> v.library_id)";

const FOOTPRINT_SQL: &str = "SELECT v.id, v.title_kind, v.title_id, e.season_id, s.series_id \
     FROM versions v \
     LEFT JOIN episodes e ON v.title_kind = 'episode' AND e.id = v.title_id \
     LEFT JOIN seasons s ON s.id = e.season_id \
     WHERE v.library_id = ?";

const UNTITLED_SQL: &str = "SELECT v.id FROM versions v \
     WHERE (v.library_id = ? OR v.available = 0) \
     AND NOT EXISTS (SELECT 1 FROM movies m WHERE v.title_kind = 'movie' AND m.id = v.title_id) \
     AND NOT EXISTS \
     (SELECT 1 FROM episodes e WHERE v.title_kind = 'episode' AND e.id = v.title_id)";

const SEASONS_OF_EPISODES_SQL: &str =
    "SELECT DISTINCT season_id AS id FROM episodes WHERE id IN ({ids})";
const SERIES_OF_SEASONS_SQL: &str =
    "SELECT DISTINCT series_id AS id FROM seasons WHERE id IN ({ids})";
const DELETE_MOVIES_SQL: &str = "DELETE FROM movies WHERE id IN ({ids}) RETURNING id";
const DELETE_EPISODES_SQL: &str = "DELETE FROM episodes WHERE id IN ({ids}) RETURNING id";
const EMPTIED_SERIES_SQL: &str = "SELECT id FROM series WHERE id IN ({ids}) AND NOT EXISTS \
     (SELECT 1 FROM seasons sn JOIN episodes e ON e.season_id = sn.id \
     WHERE sn.series_id = series.id)";
const DELETE_EMPTIED_SEASONS_SQL: &str = "DELETE FROM seasons WHERE id IN ({ids}) \
     AND NOT EXISTS (SELECT 1 FROM episodes e WHERE e.season_id = seasons.id) RETURNING id";
const DELETE_SEASONS_OF_SERIES_SQL: &str =
    "DELETE FROM seasons WHERE series_id IN ({ids}) RETURNING id";
const DELETE_GONE_SERIES_SQL: &str = "DELETE FROM series WHERE id IN ({ids})";
const DELETE_COLLECTION_LINKS_SQL: &str = "DELETE FROM collection_movies WHERE movie_id IN ({ids})";
const DELETE_TRANSLATIONS_SQL: &str = "DELETE FROM subtitle_files \
     WHERE translated_from IN (SELECT id FROM subtitle_files WHERE source = ?) \
     AND version_id IN ({ids})";
const DELETE_DOWNLOADS_SQL: &str =
    "DELETE FROM subtitle_files WHERE source = ? AND version_id IN ({ids})";
const DELETE_UNAVAILABLE_SQL: &str = "DELETE FROM versions WHERE library_id = ? \
     AND available = 0 RETURNING id, title_kind, title_id";
const DELETE_EVERY_UNAVAILABLE_SQL: &str =
    "DELETE FROM versions WHERE available = 0 RETURNING id, title_kind, title_id";
const DELETE_EMPTY_MOVIES_SQL: &str = "DELETE FROM movies WHERE id IN ({ids}) AND NOT EXISTS \
     (SELECT 1 FROM versions v WHERE v.title_kind = 'movie' AND v.title_id = movies.id) \
     RETURNING id";
const DELETE_EMPTY_EPISODES_SQL: &str = "DELETE FROM episodes WHERE id IN ({ids}) \
     AND NOT EXISTS (SELECT 1 FROM versions v \
     WHERE v.title_kind = 'episode' AND v.title_id = episodes.id) RETURNING id";
const EVERY_MOVIE_SQL: &str = "SELECT id FROM movies ORDER BY id";
const EVERY_EPISODE_SQL: &str = "SELECT id FROM episodes ORDER BY id";
const EVERY_SEASON_SQL: &str = "SELECT id FROM seasons ORDER BY id";
const EVERY_SERIES_SQL: &str = "SELECT id FROM series ORDER BY id";

struct Candidates {
    movies: Vec<String>,
    episodes: Vec<String>,
    seasons: Vec<String>,
    series: Vec<String>,
}

struct Removed {
    versions: HashSet<VersionId>,
    movies: Vec<String>,
    episodes: Vec<String>,
}

fn chunk_sql(template: &str, len: usize) -> AssertSqlSafe<String> {
    AssertSqlSafe(template.replace("{ids}", &vec!["?"; len].join(", ")))
}

async fn ids_in(
    conn: &mut SqliteConnection,
    template: &str,
    ids: &[String],
) -> Result<Vec<String>, RepositoryError> {
    let mut found = Vec::new();
    for chunk in ids.chunks(BIND_CHUNK) {
        let mut query = sqlx::query(chunk_sql(template, chunk.len()));
        for id in chunk {
            query = query.bind(id.as_str());
        }
        for row in query.fetch_all(&mut *conn).await.map_err(backend)? {
            found.push(column::<String>(&row, "id")?);
        }
    }
    Ok(found)
}

async fn run_in(
    conn: &mut SqliteConnection,
    template: &str,
    lead: &[&str],
    ids: &[String],
) -> Result<u64, RepositoryError> {
    let mut affected = 0;
    for chunk in ids.chunks(BIND_CHUNK) {
        let mut query = sqlx::query(chunk_sql(template, chunk.len()));
        for value in lead {
            query = query.bind(*value);
        }
        for id in chunk {
            query = query.bind(id.as_str());
        }
        affected += query.execute(&mut *conn).await.map_err(backend)?.rows_affected();
    }
    Ok(affected)
}

pub(super) async fn clean_title_rows(
    conn: &mut SqliteConnection,
    kind: &str,
    ids: &[String],
) -> Result<(), RepositoryError> {
    for template in TITLE_ROW_CLEANUP {
        run_in(conn, template, &[kind], ids).await?;
    }
    if kind == "movie" {
        run_in(conn, DELETE_COLLECTION_LINKS_SQL, &[], ids).await?;
    }
    Ok(())
}

fn sorted(ids: HashSet<String>) -> Vec<String> {
    let mut ids: Vec<String> = ids.into_iter().collect();
    ids.sort();
    ids
}

fn split_versions(rows: &[SqliteRow]) -> Result<Removed, RepositoryError> {
    let mut versions = HashSet::new();
    let (mut movies, mut episodes) = (HashSet::new(), HashSet::new());
    for row in rows {
        versions.insert(VersionId(column(row, "id")?));
        match title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)? {
            TitleId::Movie(id) => movies.insert(id.0),
            TitleId::Episode(id) => episodes.insert(id.0),
        };
    }
    Ok(Removed { versions, movies: sorted(movies), episodes: sorted(episodes) })
}

async fn parents_of(
    conn: &mut SqliteConnection,
    movies: Vec<String>,
    episodes: Vec<String>,
) -> Result<Candidates, RepositoryError> {
    let seasons = ids_in(conn, SEASONS_OF_EPISODES_SQL, &episodes).await?;
    let series = ids_in(conn, SERIES_OF_SEASONS_SQL, &seasons).await?;
    Ok(Candidates { movies, episodes, seasons, series })
}

async fn every_id(
    conn: &mut SqliteConnection,
    sql: &'static str,
) -> Result<Vec<String>, RepositoryError> {
    let rows = sqlx::query(sql).fetch_all(&mut *conn).await.map_err(backend)?;
    rows.iter().map(|row| column(row, "id")).collect()
}

async fn every_title(conn: &mut SqliteConnection) -> Result<Candidates, RepositoryError> {
    Ok(Candidates {
        movies: every_id(conn, EVERY_MOVIE_SQL).await?,
        episodes: every_id(conn, EVERY_EPISODE_SQL).await?,
        seasons: every_id(conn, EVERY_SEASON_SQL).await?,
        series: every_id(conn, EVERY_SERIES_SQL).await?,
    })
}

async fn remove_titles(
    conn: &mut SqliteConnection,
    candidates: &Candidates,
    delete_movies: &str,
    delete_episodes: &str,
) -> Result<TitleSet, RepositoryError> {
    let gone_movies = ids_in(conn, delete_movies, &candidates.movies).await?;
    let gone_episodes = ids_in(conn, delete_episodes, &candidates.episodes).await?;
    let gone_series = ids_in(conn, EMPTIED_SERIES_SQL, &candidates.series).await?;
    let mut gone_seasons = ids_in(conn, DELETE_EMPTIED_SEASONS_SQL, &candidates.seasons).await?;
    gone_seasons.extend(ids_in(conn, DELETE_SEASONS_OF_SERIES_SQL, &gone_series).await?);
    run_in(conn, DELETE_GONE_SERIES_SQL, &[], &gone_series).await?;
    for (kind, ids) in [
        ("movie", &gone_movies),
        ("episode", &gone_episodes),
        ("season", &gone_seasons),
        ("series", &gone_series),
    ] {
        clean_title_rows(conn, kind, ids).await?;
    }
    Ok(TitleSet {
        movies: gone_movies.into_iter().map(MovieId).collect(),
        series: gone_series.into_iter().map(SeriesId).collect(),
        seasons: gone_seasons.into_iter().map(SeasonId).collect(),
        episodes: gone_episodes.into_iter().map(EpisodeId).collect(),
    })
}

pub(super) async fn purge_library(
    conn: &mut SqliteConnection,
    library: &LibraryId,
) -> Result<LibraryPurge, RepositoryError> {
    let rows = sqlx::query(UNSHARED_VERSIONS_SQL)
        .bind(library.0.as_str())
        .fetch_all(&mut *conn)
        .await
        .map_err(backend)?;
    let Removed { versions, movies, episodes } = split_versions(&rows)?;
    let candidates = parents_of(conn, movies, episodes).await?;
    let titles = remove_titles(conn, &candidates, DELETE_MOVIES_SQL, DELETE_EPISODES_SQL).await?;
    let version_ids = sorted(versions.iter().map(|id| id.0.clone()).collect());
    let source = subtitle_source_to_str(SubtitleSource::OpenSubtitles);
    let translations = run_in(conn, DELETE_TRANSLATIONS_SQL, &[source], &version_ids).await?;
    let downloads = run_in(conn, DELETE_DOWNLOADS_SQL, &[source], &version_ids).await?;
    Ok(LibraryPurge { titles, versions, subtitles: translations + downloads })
}

pub(super) async fn clean_catalog(
    conn: &mut SqliteConnection,
    scope: &CleanScope,
) -> Result<CatalogClean, RepositoryError> {
    let removed = match scope {
        CleanScope::Library(library) => {
            sqlx::query(DELETE_UNAVAILABLE_SQL).bind(library.0.as_str())
        }
        CleanScope::All => sqlx::query(DELETE_EVERY_UNAVAILABLE_SQL),
    };
    let rows = removed.fetch_all(&mut *conn).await.map_err(backend)?;
    let Removed { versions, movies, episodes } = split_versions(&rows)?;
    let candidates = match scope {
        CleanScope::Library(_) => parents_of(conn, movies, episodes).await?,
        CleanScope::All => every_title(conn).await?,
    };
    let titles =
        remove_titles(conn, &candidates, DELETE_EMPTY_MOVIES_SQL, DELETE_EMPTY_EPISODES_SQL)
            .await?;
    Ok(CatalogClean { titles, versions })
}

pub(super) async fn library_footprint(
    pool: &SqlitePool,
    library: &LibraryId,
) -> Result<LibraryFootprint, RepositoryError> {
    let rows = sqlx::query(FOOTPRINT_SQL)
        .bind(library.0.as_str())
        .fetch_all(pool)
        .await
        .map_err(backend)?;
    let mut footprint = LibraryFootprint::default();
    for row in &rows {
        footprint.versions.insert(VersionId(column(row, "id")?));
        match title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)? {
            TitleId::Movie(id) => footprint.titles.movies.insert(id),
            TitleId::Episode(id) => footprint.titles.episodes.insert(id),
        };
        if let Some(season) = column::<Option<String>>(row, "season_id")? {
            footprint.titles.seasons.insert(SeasonId(season));
        }
        if let Some(series) = column::<Option<String>>(row, "series_id")? {
            footprint.titles.series.insert(SeriesId(series));
        }
    }
    Ok(footprint)
}

pub(super) async fn untitled_versions(
    pool: &SqlitePool,
    library: &LibraryId,
) -> Result<HashSet<VersionId>, RepositoryError> {
    let rows = sqlx::query(UNTITLED_SQL)
        .bind(library.0.as_str())
        .fetch_all(pool)
        .await
        .map_err(backend)?;
    rows.iter().map(|row| Ok(VersionId(column(row, "id")?))).collect()
}
