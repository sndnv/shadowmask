use std::path::Path;

use domain::catalog::{
    Collection, CollectionId, Episode, EpisodeId, Movie, MovieId, Season, SeasonId, Series,
    SeriesId, TitleId, Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, Page, PageRequest, Quality};
use domain::discovery::SearchResult;
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFormat, TrickplayAsset, VideoTrack,
};
use domain::metadata::ContentRating;
use domain::repository::{CatalogRepository, SearchIndex};
use domain::text::normalize_title;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};

use crate::codec::{title_from_parts, title_kind};
use crate::pool::{backend, column, from_millis, open, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/catalog");

#[derive(Clone)]
pub struct SqliteCatalogRepo {
    pool: SqlitePool,
}

impl SqliteCatalogRepo {
    pub async fn connect(path: &Path) -> Result<Self, RepositoryError> {
        Ok(Self {
            pool: open(path, &MIGRATOR).await?,
        })
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn insert_movie(&self, movie: Movie) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT OR REPLACE INTO movies \
             (id, title, year, overview, runtime_minutes, rating_system, rating_code, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(movie.id.0.as_str())
        .bind(movie.title.as_str())
        .bind(movie.year.map(i64::from))
        .bind(movie.overview.as_deref())
        .bind(movie.runtime_minutes.map(i64::from))
        .bind(movie.content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(movie.content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(to_millis(movie.added_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(&mut tx, "movie", movie.id.0.as_str(), movie.title.as_str()).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_series(&self, series: Series) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT OR REPLACE INTO series \
             (id, title, year, overview, rating_system, rating_code, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(series.id.0.as_str())
        .bind(series.title.as_str())
        .bind(series.year.map(i64::from))
        .bind(series.overview.as_deref())
        .bind(series.content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(series.content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(to_millis(series.added_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(
            &mut tx,
            "series",
            series.id.0.as_str(),
            series.title.as_str(),
        )
        .await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_season(&self, season: Season) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT OR REPLACE INTO seasons (id, series_id, number, title, overview) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(season.id.0.as_str())
        .bind(season.series.0.as_str())
        .bind(i64::from(season.number))
        .bind(season.title.as_deref())
        .bind(season.overview.as_deref())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    pub async fn insert_episode(&self, episode: Episode) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT OR REPLACE INTO episodes \
             (id, season_id, number, title, overview, runtime_minutes, air_date, added_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(episode.id.0.as_str())
        .bind(episode.season.0.as_str())
        .bind(i64::from(episode.number))
        .bind(episode.title.as_str())
        .bind(episode.overview.as_deref())
        .bind(episode.runtime_minutes.map(i64::from))
        .bind(episode.air_date.map(to_millis))
        .bind(to_millis(episode.added_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(
            &mut tx,
            "episode",
            episode.id.0.as_str(),
            episode.title.as_str(),
        )
        .await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_collection(&self, collection: Collection) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = collection.id.0.as_str();
        sqlx::query("INSERT OR REPLACE INTO collections (id, name, overview) VALUES (?, ?, ?)")
            .bind(id)
            .bind(collection.name.as_str())
            .bind(collection.overview.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        sqlx::query("DELETE FROM collection_movies WHERE collection_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, movie) in collection.movies.iter().enumerate() {
            sqlx::query(
                "INSERT INTO collection_movies (collection_id, ordinal, movie_id) VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind(ordinal as i64)
            .bind(movie.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_version(&self, version: Version) -> Result<(), RepositoryError> {
        insert_version_row(&self.pool, &version).await
    }

    pub async fn insert_version_detail(
        &self,
        detail: VersionDetail,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let vid = detail.version.id.0.clone();
        insert_version_row(&mut *tx, &detail.version).await?;
        for (ordinal, track) in detail.video.iter().enumerate() {
            sqlx::query(
                "INSERT INTO video_tracks \
                 (version_id, ordinal, stream_index, codec, width, height, bit_depth, hdr, \
                  frame_rate, bitrate) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(i64::from(track.index))
            .bind(track.codec.as_str())
            .bind(i64::from(track.width))
            .bind(i64::from(track.height))
            .bind(i64::from(track.bit_depth))
            .bind(track.hdr.map(hdr_to_str))
            .bind(f64::from(track.frame_rate))
            .bind(track.bitrate.map(|b| b as i64))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, track) in detail.audio.iter().enumerate() {
            sqlx::query(
                "INSERT INTO audio_tracks \
                 (version_id, ordinal, stream_index, codec, channels, language, bitrate) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(i64::from(track.index))
            .bind(track.codec.as_str())
            .bind(i64::from(track.channels))
            .bind(track.language.as_ref().map(|l| l.0.as_str()))
            .bind(track.bitrate.map(|b| b as i64))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, track) in detail.subtitles.iter().enumerate() {
            sqlx::query(
                "INSERT INTO subtitle_tracks \
                 (version_id, ordinal, stream_index, language, format, forced, is_default) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(i64::from(track.index))
            .bind(track.language.as_ref().map(|l| l.0.as_str()))
            .bind(subtitle_format_to_str(track.format))
            .bind(track.forced)
            .bind(track.default)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, chapter) in detail.chapters.iter().enumerate() {
            sqlx::query(
                "INSERT INTO chapters (version_id, ordinal, title, start_ms) VALUES (?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(chapter.title.as_str())
            .bind(chapter.start_ms as i64)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, marker) in detail.markers.intros.iter().enumerate() {
            sqlx::query(
                "INSERT INTO intro_markers (version_id, ordinal, start_ms, end_ms) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(marker.start_ms as i64)
            .bind(marker.end_ms as i64)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, marker) in detail.markers.credits.iter().enumerate() {
            sqlx::query(
                "INSERT INTO credits_markers (version_id, ordinal, start_ms, end_ms) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(marker.start_ms as i64)
            .bind(marker.end_ms as i64)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, asset) in detail.trickplay.iter().enumerate() {
            sqlx::query(
                "INSERT INTO trickplay_assets \
                 (version_id, ordinal, interval_ms, tile_width, tile_height) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(asset.interval_ms as i64)
            .bind(i64::from(asset.tile_width))
            .bind(i64::from(asset.tile_height))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            for (sheet_ordinal, path) in asset.sheet_paths.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO trickplay_sheets \
                     (version_id, asset_ordinal, sheet_ordinal, path) VALUES (?, ?, ?, ?)",
                )
                .bind(vid.as_str())
                .bind(ordinal as i64)
                .bind(sheet_ordinal as i64)
                .bind(path.as_str())
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
            }
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn load_collection(&self, row: &SqliteRow) -> Result<Collection, RepositoryError> {
        let id: String = column(row, "id")?;
        let movie_rows = sqlx::query(
            "SELECT movie_id AS value FROM collection_movies WHERE collection_id = ? ORDER BY ordinal",
        )
        .bind(&id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let movies = movie_rows
            .iter()
            .map(|row| Ok(MovieId(column::<String>(row, "value")?)))
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        Ok(Collection {
            name: column(row, "name")?,
            overview: column(row, "overview")?,
            movies,
            id: CollectionId(id),
        })
    }

    async fn load_video(&self, version_id: &str) -> Result<Vec<VideoTrack>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM video_tracks WHERE version_id = ? ORDER BY ordinal")
            .bind(version_id)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_video).collect()
    }

    async fn load_audio(&self, version_id: &str) -> Result<Vec<AudioTrack>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM audio_tracks WHERE version_id = ? ORDER BY ordinal")
            .bind(version_id)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_audio).collect()
    }

    async fn load_subtitles(
        &self,
        version_id: &str,
    ) -> Result<Vec<EmbeddedSubtitleTrack>, RepositoryError> {
        let rows =
            sqlx::query("SELECT * FROM subtitle_tracks WHERE version_id = ? ORDER BY ordinal")
                .bind(version_id)
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        rows.iter().map(row_to_subtitle).collect()
    }

    async fn load_chapters(&self, version_id: &str) -> Result<Vec<Chapter>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM chapters WHERE version_id = ? ORDER BY ordinal")
            .bind(version_id)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_chapter).collect()
    }

    async fn load_markers(&self, version_id: &str) -> Result<DetectedMarkers, RepositoryError> {
        let intro_rows = sqlx::query(
            "SELECT start_ms, end_ms FROM intro_markers WHERE version_id = ? ORDER BY ordinal",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut intros = Vec::with_capacity(intro_rows.len());
        for row in &intro_rows {
            intros.push(IntroMarker {
                version: VersionId(version_id.to_owned()),
                start_ms: column::<i64>(row, "start_ms")? as u64,
                end_ms: column::<i64>(row, "end_ms")? as u64,
            });
        }
        let credits_rows = sqlx::query(
            "SELECT start_ms, end_ms FROM credits_markers WHERE version_id = ? ORDER BY ordinal",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut credits = Vec::with_capacity(credits_rows.len());
        for row in &credits_rows {
            credits.push(CreditsMarker {
                version: VersionId(version_id.to_owned()),
                start_ms: column::<i64>(row, "start_ms")? as u64,
                end_ms: column::<i64>(row, "end_ms")? as u64,
            });
        }
        Ok(DetectedMarkers { intros, credits })
    }

    async fn load_trickplay(
        &self,
        version_id: &str,
    ) -> Result<Vec<TrickplayAsset>, RepositoryError> {
        let asset_rows = sqlx::query(
            "SELECT ordinal, interval_ms, tile_width, tile_height FROM trickplay_assets \
             WHERE version_id = ? ORDER BY ordinal",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut assets = Vec::with_capacity(asset_rows.len());
        for row in &asset_rows {
            let ordinal: i64 = column(row, "ordinal")?;
            let sheet_rows = sqlx::query(
                "SELECT path AS value FROM trickplay_sheets \
                 WHERE version_id = ? AND asset_ordinal = ? ORDER BY sheet_ordinal",
            )
            .bind(version_id)
            .bind(ordinal)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
            let sheet_paths = sheet_rows
                .iter()
                .map(|row| column::<String>(row, "value"))
                .collect::<Result<Vec<_>, _>>()?;
            assets.push(TrickplayAsset {
                version: VersionId(version_id.to_owned()),
                interval_ms: column::<i64>(row, "interval_ms")? as u64,
                tile_width: column::<i64>(row, "tile_width")? as u32,
                tile_height: column::<i64>(row, "tile_height")? as u32,
                sheet_paths,
            });
        }
        Ok(assets)
    }
}

fn quality_to_str(quality: Quality) -> &'static str {
    match quality {
        Quality::Sd => "sd",
        Quality::Hd => "hd",
        Quality::Fhd => "fhd",
        Quality::Uhd => "uhd",
    }
}

fn quality_from_str(value: &str) -> Result<Quality, RepositoryError> {
    match value {
        "sd" => Ok(Quality::Sd),
        "hd" => Ok(Quality::Hd),
        "fhd" => Ok(Quality::Fhd),
        "uhd" => Ok(Quality::Uhd),
        other => Err(backend(format!("unknown quality: {other}"))),
    }
}

fn hdr_to_str(hdr: HdrFormat) -> &'static str {
    match hdr {
        HdrFormat::Hdr10 => "hdr10",
        HdrFormat::Hdr10Plus => "hdr10plus",
        HdrFormat::DolbyVision => "dolbyvision",
        HdrFormat::Hlg => "hlg",
    }
}

fn hdr_from_str(value: &str) -> Result<HdrFormat, RepositoryError> {
    match value {
        "hdr10" => Ok(HdrFormat::Hdr10),
        "hdr10plus" => Ok(HdrFormat::Hdr10Plus),
        "dolbyvision" => Ok(HdrFormat::DolbyVision),
        "hlg" => Ok(HdrFormat::Hlg),
        other => Err(backend(format!("unknown hdr format: {other}"))),
    }
}

fn subtitle_format_to_str(format: SubtitleFormat) -> &'static str {
    match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Ass => "ass",
        SubtitleFormat::Vtt => "vtt",
        SubtitleFormat::Pgs => "pgs",
        SubtitleFormat::VobSub => "vobsub",
    }
}

fn subtitle_format_from_str(value: &str) -> Result<SubtitleFormat, RepositoryError> {
    match value {
        "srt" => Ok(SubtitleFormat::Srt),
        "ass" => Ok(SubtitleFormat::Ass),
        "vtt" => Ok(SubtitleFormat::Vtt),
        "pgs" => Ok(SubtitleFormat::Pgs),
        "vobsub" => Ok(SubtitleFormat::VobSub),
        other => Err(backend(format!("unknown subtitle format: {other}"))),
    }
}

fn content_rating(row: &SqliteRow) -> Result<Option<ContentRating>, RepositoryError> {
    Ok(
        match (
            column::<Option<String>>(row, "rating_system")?,
            column::<Option<String>>(row, "rating_code")?,
        ) {
            (Some(system), Some(code)) => Some(ContentRating { system, code }),
            _ => None,
        },
    )
}

impl SearchIndex for SqliteCatalogRepo {
    async fn search(
        &self,
        query: &str,
        page: PageRequest,
    ) -> Result<Page<SearchResult>, RepositoryError> {
        let needle = normalize_title(query);
        let mut candidates: Vec<SearchResult> = Vec::new();
        if !needle.is_empty() {
            let match_query = fts_match(&needle);
            let movies = sqlx::query(
                "SELECT m.* FROM search_index s JOIN movies m ON m.id = s.id \
                 WHERE s.kind = 'movie' AND s.title MATCH ?",
            )
            .bind(&match_query)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
            for row in &movies {
                candidates.push(SearchResult::Movie(row_to_movie(row)?));
            }
            let series = sqlx::query(
                "SELECT e.* FROM search_index s JOIN series e ON e.id = s.id \
                 WHERE s.kind = 'series' AND s.title MATCH ?",
            )
            .bind(&match_query)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
            for row in &series {
                candidates.push(SearchResult::Series(row_to_series(row)?));
            }
            let episodes = sqlx::query(
                "SELECT e.* FROM search_index s JOIN episodes e ON e.id = s.id \
                 WHERE s.kind = 'episode' AND s.title MATCH ?",
            )
            .bind(&match_query)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
            for row in &episodes {
                candidates.push(SearchResult::Episode(row_to_episode(row)?));
            }
        }
        Ok(domain::discovery::search(&candidates, query, page))
    }

    async fn rebuild(&self) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM search_index")
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        reindex_kind(&mut tx, "movie", "SELECT id, title FROM movies").await?;
        reindex_kind(&mut tx, "series", "SELECT id, title FROM series").await?;
        reindex_kind(&mut tx, "episode", "SELECT id, title FROM episodes").await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }
}

async fn sync_search_row(
    conn: &mut SqliteConnection,
    kind: &str,
    id: &str,
    title: &str,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM search_index WHERE kind = ? AND id = ?")
        .bind(kind)
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    sqlx::query("INSERT INTO search_index (kind, id, title) VALUES (?, ?, ?)")
        .bind(kind)
        .bind(id)
        .bind(normalize_title(title))
        .execute(&mut *conn)
        .await
        .map_err(backend)?;
    Ok(())
}

async fn reindex_kind(
    conn: &mut SqliteConnection,
    kind: &str,
    select: &str,
) -> Result<(), RepositoryError> {
    let rows = sqlx::query(select)
        .fetch_all(&mut *conn)
        .await
        .map_err(backend)?;
    for row in &rows {
        let id: String = column(row, "id")?;
        let title: String = column(row, "title")?;
        sqlx::query("INSERT INTO search_index (kind, id, title) VALUES (?, ?, ?)")
            .bind(kind)
            .bind(id)
            .bind(normalize_title(&title))
            .execute(&mut *conn)
            .await
            .map_err(backend)?;
    }
    Ok(())
}

fn fts_match(needle: &str) -> String {
    needle
        .split_whitespace()
        .map(|token| format!("{token}*"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn row_to_movie(row: &SqliteRow) -> Result<Movie, RepositoryError> {
    Ok(Movie {
        id: MovieId(column(row, "id")?),
        title: column(row, "title")?,
        year: column::<Option<i64>>(row, "year")?.map(|y| y as u16),
        overview: column(row, "overview")?,
        runtime_minutes: column::<Option<i64>>(row, "runtime_minutes")?.map(|v| v as u32),
        content_rating: content_rating(row)?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_series(row: &SqliteRow) -> Result<Series, RepositoryError> {
    Ok(Series {
        id: SeriesId(column(row, "id")?),
        title: column(row, "title")?,
        year: column::<Option<i64>>(row, "year")?.map(|y| y as u16),
        overview: column(row, "overview")?,
        content_rating: content_rating(row)?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_season(row: &SqliteRow) -> Result<Season, RepositoryError> {
    Ok(Season {
        id: SeasonId(column(row, "id")?),
        series: SeriesId(column(row, "series_id")?),
        number: column::<i64>(row, "number")? as u16,
        title: column(row, "title")?,
        overview: column(row, "overview")?,
    })
}

fn row_to_episode(row: &SqliteRow) -> Result<Episode, RepositoryError> {
    Ok(Episode {
        id: EpisodeId(column(row, "id")?),
        season: SeasonId(column(row, "season_id")?),
        number: column::<i64>(row, "number")? as u16,
        title: column(row, "title")?,
        overview: column(row, "overview")?,
        runtime_minutes: column::<Option<i64>>(row, "runtime_minutes")?.map(|v| v as u32),
        air_date: column::<Option<i64>>(row, "air_date")?
            .map(from_millis)
            .transpose()?,
        added_at: from_millis(column(row, "added_at")?)?,
    })
}

fn row_to_version(row: &SqliteRow) -> Result<Version, RepositoryError> {
    Ok(Version {
        id: VersionId(column(row, "id")?),
        title: title_from_parts(
            &column::<String>(row, "title_kind")?,
            column(row, "title_id")?,
        )?,
        library: LibraryId(column(row, "library_id")?),
        quality: quality_from_str(&column::<String>(row, "quality")?)?,
        container: column(row, "container")?,
        path: column(row, "path")?,
        size_bytes: column::<i64>(row, "size_bytes")? as u64,
        duration_ms: column::<i64>(row, "duration_ms")? as u64,
        edition: column(row, "edition")?,
    })
}

fn row_to_video(row: &SqliteRow) -> Result<VideoTrack, RepositoryError> {
    Ok(VideoTrack {
        index: column::<i64>(row, "stream_index")? as u32,
        codec: column(row, "codec")?,
        width: column::<i64>(row, "width")? as u32,
        height: column::<i64>(row, "height")? as u32,
        bit_depth: column::<i64>(row, "bit_depth")? as u8,
        hdr: column::<Option<String>>(row, "hdr")?
            .map(|value| hdr_from_str(&value))
            .transpose()?,
        frame_rate: column::<f64>(row, "frame_rate")? as f32,
        bitrate: column::<Option<i64>>(row, "bitrate")?.map(|b| b as u64),
    })
}

fn row_to_audio(row: &SqliteRow) -> Result<AudioTrack, RepositoryError> {
    Ok(AudioTrack {
        index: column::<i64>(row, "stream_index")? as u32,
        codec: column(row, "codec")?,
        channels: column::<i64>(row, "channels")? as u8,
        language: column::<Option<String>>(row, "language")?.map(LanguageCode),
        bitrate: column::<Option<i64>>(row, "bitrate")?.map(|b| b as u64),
    })
}

fn row_to_subtitle(row: &SqliteRow) -> Result<EmbeddedSubtitleTrack, RepositoryError> {
    Ok(EmbeddedSubtitleTrack {
        index: column::<i64>(row, "stream_index")? as u32,
        language: column::<Option<String>>(row, "language")?.map(LanguageCode),
        format: subtitle_format_from_str(&column::<String>(row, "format")?)?,
        forced: column(row, "forced")?,
        default: column(row, "is_default")?,
    })
}

fn row_to_chapter(row: &SqliteRow) -> Result<Chapter, RepositoryError> {
    Ok(Chapter {
        title: column(row, "title")?,
        start_ms: column::<i64>(row, "start_ms")? as u64,
    })
}

async fn insert_version_row<'e, E>(executor: E, version: &Version) -> Result<(), RepositoryError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT OR REPLACE INTO versions \
         (id, title_kind, title_id, library_id, quality, container, path, size_bytes, \
          duration_ms, edition) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(version.id.0.as_str())
    .bind(title_kind(&version.title))
    .bind(version.title.id())
    .bind(version.library.0.as_str())
    .bind(quality_to_str(version.quality))
    .bind(version.container.as_str())
    .bind(version.path.as_str())
    .bind(version.size_bytes as i64)
    .bind(version.duration_ms as i64)
    .bind(version.edition.as_deref())
    .execute(executor)
    .await
    .map_err(backend)?;
    Ok(())
}

async fn count_all(pool: &SqlitePool, query: &str) -> Result<u64, RepositoryError> {
    let row = sqlx::query(query).fetch_one(pool).await.map_err(backend)?;
    Ok(column::<i64>(&row, "n")? as u64)
}

impl CatalogRepository for SqliteCatalogRepo {
    async fn list_movies(&self, page: PageRequest) -> Result<Page<Movie>, RepositoryError> {
        let total = count_all(&self.pool, "SELECT COUNT(*) AS n FROM movies").await?;
        let rows =
            sqlx::query("SELECT * FROM movies ORDER BY added_at ASC, id ASC LIMIT ? OFFSET ?")
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let items = rows
            .iter()
            .map(row_to_movie)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM movies WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_movie).transpose()
    }

    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError> {
        let total = count_all(&self.pool, "SELECT COUNT(*) AS n FROM series").await?;
        let rows =
            sqlx::query("SELECT * FROM series ORDER BY added_at ASC, id ASC LIMIT ? OFFSET ?")
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let items = rows
            .iter()
            .map(row_to_series)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM series WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_series).transpose()
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM seasons WHERE series_id = ? ORDER BY number, id")
            .bind(series.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_season).collect()
    }

    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM episodes WHERE season_id = ? ORDER BY number, id")
            .bind(season.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter().map(row_to_episode).collect()
    }

    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM episodes WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_episode).transpose()
    }

    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError> {
        let total = count_all(&self.pool, "SELECT COUNT(*) AS n FROM collections").await?;
        let rows = sqlx::query("SELECT * FROM collections ORDER BY id LIMIT ? OFFSET ?")
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(self.load_collection(row).await?);
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_collection(
        &self,
        id: &CollectionId,
    ) -> Result<Option<Collection>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM collections WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        match row {
            Some(row) => Ok(Some(self.load_collection(&row).await?)),
            None => Ok(None),
        }
    }

    async fn upsert_collection(&self, collection: Collection) -> Result<(), RepositoryError> {
        self.insert_collection(collection).await
    }

    async fn delete_collection(&self, id: &CollectionId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn get_season(&self, id: &SeasonId) -> Result<Option<Season>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM seasons WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_season).transpose()
    }

    async fn list_versions(
        &self,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        let count_row =
            sqlx::query("SELECT COUNT(*) AS n FROM versions WHERE title_kind = ? AND title_id = ?")
                .bind(title_kind(title))
                .bind(title.id())
                .fetch_one(&self.pool)
                .await
                .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(
            "SELECT * FROM versions WHERE title_kind = ? AND title_id = ? \
             ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(title_kind(title))
        .bind(title.id())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let items = rows
            .iter()
            .map(row_to_version)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn list_library_versions(
        &self,
        library: &LibraryId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM versions WHERE library_id = ?")
            .bind(library.0.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows =
            sqlx::query("SELECT * FROM versions WHERE library_id = ? ORDER BY id LIMIT ? OFFSET ?")
                .bind(library.0.as_str())
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let items = rows
            .iter()
            .map(row_to_version)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn version_detail(
        &self,
        id: &VersionId,
    ) -> Result<Option<VersionDetail>, RepositoryError> {
        let row = sqlx::query("SELECT * FROM versions WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let version = row_to_version(&row)?;
        let vid = version.id.0.clone();
        let video = self.load_video(&vid).await?;
        let audio = self.load_audio(&vid).await?;
        let subtitles = self.load_subtitles(&vid).await?;
        let chapters = self.load_chapters(&vid).await?;
        let markers = self.load_markers(&vid).await?;
        let trickplay = self.load_trickplay(&vid).await?;
        Ok(Some(VersionDetail {
            version,
            video,
            audio,
            subtitles,
            chapters,
            markers,
            trickplay,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_round_trips_and_rejects_unknown() {
        for quality in [Quality::Sd, Quality::Hd, Quality::Fhd, Quality::Uhd] {
            assert_eq!(quality_from_str(quality_to_str(quality)).unwrap(), quality);
        }
        for hdr in [
            HdrFormat::Hdr10,
            HdrFormat::Hdr10Plus,
            HdrFormat::DolbyVision,
            HdrFormat::Hlg,
        ] {
            assert_eq!(hdr_from_str(hdr_to_str(hdr)).unwrap(), hdr);
        }
        for format in [
            SubtitleFormat::Srt,
            SubtitleFormat::Ass,
            SubtitleFormat::Vtt,
            SubtitleFormat::Pgs,
            SubtitleFormat::VobSub,
        ] {
            assert_eq!(
                subtitle_format_from_str(subtitle_format_to_str(format)).unwrap(),
                format
            );
        }
        assert!(quality_from_str("nope").is_err());
        assert!(hdr_from_str("nope").is_err());
        assert!(subtitle_format_from_str("nope").is_err());
    }

    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
            .await
            .unwrap();
        repo.pool.close().await;
        assert!(
            repo.list_movies(PageRequest {
                offset: 0,
                limit: 10
            })
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_corrupt_title_kind() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO versions \
             (id, title_kind, title_id, library_id, quality, container, path, size_bytes, \
              duration_ms, edition) \
             VALUES ('vx', 'bogus', 'm1', 'lib1', 'sd', 'mkv', '/x.mkv', 1, 1, NULL)",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.version_detail(&VersionId("vx".into())).await.is_err());
    }
}
