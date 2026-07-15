use std::path::Path;

use domain::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, Collection, CollectionId, Episode, EpisodeId, Movie,
    MovieDetail, MovieId, Season, SeasonId, Series, SeriesDetail, SeriesId, TitleId, TitleKind,
    TitleRef, Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, Page, PageRequest, Quality};
use domain::discovery::{SearchKind, SearchResult};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFormat, TrickplayAsset, VideoTrack,
};
use domain::metadata::{
    ContentRating, Credit, CreditedPerson, ExternalId, Extra, Genre, GenreId, Person, PersonId,
    Rating, Studio, StudioId, TitleEnrichment,
};
use domain::repository::{CatalogRepository, SearchIndex};
use domain::text::normalize_title;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};

use crate::codec::{
    artwork_kind_from_str, artwork_kind_to_str, artwork_owner_parts, credit_role_from_str,
    credit_role_to_str, extra_kind_from_str, extra_kind_to_str, title_from_parts, title_kind,
    title_kind_to_str, title_ref_from_parts,
};
use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

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

    pub async fn ping(&self) -> Result<(), RepositoryError> {
        ping(&self.pool).await
    }

    pub async fn close(&self) {
        checkpoint(&self.pool).await;
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
                 (version_id, ordinal, interval_ms, grid_columns, grid_rows, tile_width, \
                  tile_height) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(asset.interval_ms as i64)
            .bind(i64::from(asset.columns))
            .bind(i64::from(asset.rows))
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
        let artwork = self.load_artwork("collection", &id).await?;
        Ok(Collection {
            name: column(row, "name")?,
            overview: column(row, "overview")?,
            movies,
            id: CollectionId(id),
            artwork,
        })
    }

    async fn load_artwork(
        &self,
        title_kind: &str,
        title_id: &str,
    ) -> Result<Vec<ArtworkRef>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT artwork_id, kind FROM artwork \
             WHERE title_kind = ? AND title_id = ? ORDER BY ordinal",
        )
        .bind(title_kind)
        .bind(title_id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut refs = Vec::with_capacity(rows.len());
        for row in &rows {
            let artwork_id: String = column(row, "artwork_id")?;
            let width_rows = sqlx::query(
                "SELECT width AS value FROM artwork_widths WHERE artwork_id = ? ORDER BY ordinal",
            )
            .bind(&artwork_id)
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
            let widths = width_rows
                .iter()
                .map(|row| Ok(column::<i64>(row, "value")? as u32))
                .collect::<Result<Vec<_>, RepositoryError>>()?;
            refs.push(ArtworkRef {
                kind: artwork_kind_from_str(&column::<String>(row, "kind")?)?,
                id: ArtworkId(artwork_id),
                widths,
            });
        }
        Ok(refs)
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
            "SELECT ordinal, interval_ms, grid_columns, grid_rows, tile_width, tile_height \
             FROM trickplay_assets WHERE version_id = ? ORDER BY ordinal",
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
                columns: column::<i64>(row, "grid_columns")? as u32,
                rows: column::<i64>(row, "grid_rows")? as u32,
                tile_width: column::<i64>(row, "tile_width")? as u32,
                tile_height: column::<i64>(row, "tile_height")? as u32,
                sheet_paths,
            });
        }
        Ok(assets)
    }

    async fn load_genres(&self, kind: &str, id: &str) -> Result<Vec<Genre>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT g.id, g.name FROM title_genres tg JOIN genres g ON g.id = tg.genre_id \
             WHERE tg.title_kind = ? AND tg.title_id = ? ORDER BY tg.ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Genre {
                    id: GenreId(column(row, "id")?),
                    name: column(row, "name")?,
                })
            })
            .collect()
    }

    async fn load_credits(
        &self,
        kind: &str,
        id: &str,
    ) -> Result<Vec<CreditedPerson>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT p.id AS person_id, p.name AS person_name, c.role, c.character, c.credit_order \
             FROM credits c JOIN people p ON p.id = c.person_id \
             WHERE c.title_kind = ? AND c.title_id = ? ORDER BY c.ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(CreditedPerson {
                    person: Person {
                        id: PersonId(column(row, "person_id")?),
                        name: column(row, "person_name")?,
                    },
                    role: credit_role_from_str(&column::<String>(row, "role")?)?,
                    character: column(row, "character")?,
                    order: column::<i64>(row, "credit_order")? as u32,
                })
            })
            .collect()
    }

    async fn load_studios(&self, kind: &str, id: &str) -> Result<Vec<Studio>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT s.id, s.name FROM title_studios ts JOIN studios s ON s.id = ts.studio_id \
             WHERE ts.title_kind = ? AND ts.title_id = ? ORDER BY ts.ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Studio {
                    id: StudioId(column(row, "id")?),
                    name: column(row, "name")?,
                })
            })
            .collect()
    }

    async fn load_ratings(&self, kind: &str, id: &str) -> Result<Vec<Rating>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT source, value FROM title_ratings \
             WHERE title_kind = ? AND title_id = ? ORDER BY ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Rating {
                    source: column(row, "source")?,
                    value: column::<f64>(row, "value")? as f32,
                })
            })
            .collect()
    }

    async fn load_external_ids(
        &self,
        kind: &str,
        id: &str,
    ) -> Result<Vec<ExternalId>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT source, value FROM title_external_ids \
             WHERE title_kind = ? AND title_id = ? ORDER BY ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(ExternalId {
                    source: column(row, "source")?,
                    value: column(row, "value")?,
                })
            })
            .collect()
    }

    async fn load_extras(&self, kind: &str, id: &str) -> Result<Vec<Extra>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT kind, title, path FROM title_extras \
             WHERE title_kind = ? AND title_id = ? ORDER BY ordinal",
        )
        .bind(kind)
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Extra {
                    kind: extra_kind_from_str(&column::<String>(row, "kind")?)?,
                    title: column(row, "title")?,
                    path: column(row, "path")?,
                })
            })
            .collect()
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
        types: &[SearchKind],
        page: PageRequest,
    ) -> Result<Page<SearchResult>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "search");
        let needle = normalize_title(query);
        let mut candidates: Vec<SearchResult> = Vec::new();
        if !needle.is_empty() {
            let match_query = fts_match(&needle);
            let want = |kind: SearchKind| types.is_empty() || types.contains(&kind);
            if want(SearchKind::Movie) {
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
            }
            if want(SearchKind::Series) {
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
            }
            if want(SearchKind::Episode) {
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
            if want(SearchKind::Person) {
                let people = sqlx::query(
                    "SELECT p.id, p.name FROM search_index s JOIN people p ON p.id = s.id \
                     WHERE s.kind = 'person' AND s.title MATCH ?",
                )
                .bind(&match_query)
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
                for row in &people {
                    candidates.push(SearchResult::Person(Person {
                        id: PersonId(column(row, "id")?),
                        name: column(row, "name")?,
                    }));
                }
            }
        }
        Ok(domain::discovery::search(&candidates, query, types, page))
    }

    async fn rebuild(&self) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "rebuild");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM search_index")
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        reindex_kind(&mut tx, "movie", "SELECT id, title FROM movies").await?;
        reindex_kind(&mut tx, "series", "SELECT id, title FROM series").await?;
        reindex_kind(&mut tx, "episode", "SELECT id, title FROM episodes").await?;
        reindex_kind(&mut tx, "person", "SELECT id, name AS title FROM people").await?;
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
        artwork: Vec::new(),
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
        artwork: Vec::new(),
    })
}

fn row_to_season(row: &SqliteRow) -> Result<Season, RepositoryError> {
    Ok(Season {
        id: SeasonId(column(row, "id")?),
        series: SeriesId(column(row, "series_id")?),
        number: column::<i64>(row, "number")? as u16,
        title: column(row, "title")?,
        overview: column(row, "overview")?,
        artwork: Vec::new(),
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
        artwork: Vec::new(),
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
        available: column(row, "available")?,
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
          duration_ms, edition, available) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(version.available)
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
        let _op = DbOpGuard::new("catalog", "list_movies");
        let total = count_all(&self.pool, "SELECT COUNT(*) AS n FROM movies").await?;
        let rows =
            sqlx::query("SELECT * FROM movies ORDER BY added_at ASC, id ASC LIMIT ? OFFSET ?")
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let mut items = rows
            .iter()
            .map(row_to_movie)
            .collect::<Result<Vec<_>, _>>()?;
        for movie in &mut items {
            movie.artwork = self.load_artwork("movie", &movie.id.0).await?;
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_movie(&self, id: &MovieId) -> Result<Option<Movie>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_movie");
        let row = sqlx::query("SELECT * FROM movies WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let mut movie = row_to_movie(&row)?;
        movie.artwork = self.load_artwork("movie", &movie.id.0).await?;
        Ok(Some(movie))
    }

    async fn list_series(&self, page: PageRequest) -> Result<Page<Series>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_series");
        let total = count_all(&self.pool, "SELECT COUNT(*) AS n FROM series").await?;
        let rows =
            sqlx::query("SELECT * FROM series ORDER BY added_at ASC, id ASC LIMIT ? OFFSET ?")
                .bind(i64::from(page.limit))
                .bind(i64::from(page.offset))
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        let mut items = rows
            .iter()
            .map(row_to_series)
            .collect::<Result<Vec<_>, _>>()?;
        for series in &mut items {
            series.artwork = self.load_artwork("series", &series.id.0).await?;
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn get_series(&self, id: &SeriesId) -> Result<Option<Series>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_series");
        let row = sqlx::query("SELECT * FROM series WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let mut series = row_to_series(&row)?;
        series.artwork = self.load_artwork("series", &series.id.0).await?;
        Ok(Some(series))
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_seasons");
        let rows = sqlx::query("SELECT * FROM seasons WHERE series_id = ? ORDER BY number, id")
            .bind(series.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut seasons = rows
            .iter()
            .map(row_to_season)
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        for season in &mut seasons {
            season.artwork = self.load_artwork("season", &season.id.0).await?;
        }
        Ok(seasons)
    }

    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_episodes");
        let rows = sqlx::query("SELECT * FROM episodes WHERE season_id = ? ORDER BY number, id")
            .bind(season.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut episodes = rows
            .iter()
            .map(row_to_episode)
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        for episode in &mut episodes {
            episode.artwork = self.load_artwork("episode", &episode.id.0).await?;
        }
        Ok(episodes)
    }

    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_episode");
        let row = sqlx::query("SELECT * FROM episodes WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let mut episode = row_to_episode(&row)?;
        episode.artwork = self.load_artwork("episode", &episode.id.0).await?;
        Ok(Some(episode))
    }

    async fn list_collections(
        &self,
        page: PageRequest,
    ) -> Result<Page<Collection>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_collections");
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
        let _op = DbOpGuard::new("catalog", "get_collection");
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
        let _op = DbOpGuard::new("catalog", "upsert_collection");
        self.insert_collection(collection).await
    }

    async fn delete_collection(&self, id: &CollectionId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_collection");
        sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn get_season(&self, id: &SeasonId) -> Result<Option<Season>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_season");
        let row = sqlx::query("SELECT * FROM seasons WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let mut season = row_to_season(&row)?;
        season.artwork = self.load_artwork("season", &season.id.0).await?;
        Ok(Some(season))
    }

    async fn list_versions(
        &self,
        title: &TitleId,
        page: PageRequest,
    ) -> Result<Page<Version>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_versions");
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
        let _op = DbOpGuard::new("catalog", "list_library_versions");
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
        let _op = DbOpGuard::new("catalog", "version_detail");
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

    async fn upsert_movie(&self, movie: Movie) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_movie");
        self.insert_movie(movie).await
    }

    async fn upsert_series(&self, series: Series) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_series");
        self.insert_series(series).await
    }

    async fn upsert_season(&self, season: Season) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_season");
        self.insert_season(season).await
    }

    async fn upsert_episode(&self, episode: Episode) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_episode");
        self.insert_episode(episode).await
    }

    async fn upsert_version(&self, version: Version) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_version");
        self.insert_version(version).await
    }

    async fn reconcile_library_versions(
        &self,
        library: &LibraryId,
        present_paths: &[String],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "reconcile_library_versions");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("UPDATE versions SET available = 0 WHERE library_id = ?")
            .bind(library.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for path in present_paths {
            sqlx::query("UPDATE versions SET available = 1 WHERE library_id = ? AND path = ?")
                .bind(library.0.as_str())
                .bind(path.as_str())
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn set_artwork(
        &self,
        owner: &ArtworkOwner,
        refs: &[ArtworkRef],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "set_artwork");
        let (title_kind, title_id) = artwork_owner_parts(owner);
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM artwork WHERE title_kind = ? AND title_id = ?")
            .bind(title_kind)
            .bind(title_id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, art) in refs.iter().enumerate() {
            sqlx::query(
                "INSERT INTO artwork (artwork_id, title_kind, title_id, ordinal, kind) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(art.id.0.as_str())
            .bind(title_kind)
            .bind(title_id)
            .bind(ordinal as i64)
            .bind(artwork_kind_to_str(art.kind))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            for (width_ordinal, width) in art.widths.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO artwork_widths (artwork_id, ordinal, width) VALUES (?, ?, ?)",
                )
                .bind(art.id.0.as_str())
                .bind(width_ordinal as i64)
                .bind(i64::from(*width))
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
            }
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn list_artwork(&self, owner: &ArtworkOwner) -> Result<Vec<ArtworkRef>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_artwork");
        let (title_kind, title_id) = artwork_owner_parts(owner);
        self.load_artwork(title_kind, title_id).await
    }

    async fn set_version_tracks(
        &self,
        version: &VersionId,
        video: &[VideoTrack],
        audio: &[AudioTrack],
        subtitles: &[EmbeddedSubtitleTrack],
        chapters: &[Chapter],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "set_version_tracks");
        let vid = version.0.as_str();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        for table in [
            "video_tracks",
            "audio_tracks",
            "subtitle_tracks",
            "chapters",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE version_id = ?"))
                .bind(vid)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        for (ordinal, track) in video.iter().enumerate() {
            sqlx::query(
                "INSERT INTO video_tracks \
                 (version_id, ordinal, stream_index, codec, width, height, bit_depth, hdr, \
                  frame_rate, bitrate) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid)
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
        for (ordinal, track) in audio.iter().enumerate() {
            sqlx::query(
                "INSERT INTO audio_tracks \
                 (version_id, ordinal, stream_index, codec, channels, language, bitrate) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid)
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
        for (ordinal, track) in subtitles.iter().enumerate() {
            sqlx::query(
                "INSERT INTO subtitle_tracks \
                 (version_id, ordinal, stream_index, language, format, forced, is_default) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid)
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
        for (ordinal, chapter) in chapters.iter().enumerate() {
            sqlx::query(
                "INSERT INTO chapters (version_id, ordinal, title, start_ms) VALUES (?, ?, ?, ?)",
            )
            .bind(vid)
            .bind(ordinal as i64)
            .bind(chapter.title.as_str())
            .bind(chapter.start_ms as i64)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn set_trickplay(
        &self,
        version: &VersionId,
        assets: &[TrickplayAsset],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "set_trickplay");
        let vid = version.0.as_str();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        for table in ["trickplay_sheets", "trickplay_assets"] {
            sqlx::query(&format!("DELETE FROM {table} WHERE version_id = ?"))
                .bind(vid)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        for (ordinal, asset) in assets.iter().enumerate() {
            sqlx::query(
                "INSERT INTO trickplay_assets \
                 (version_id, ordinal, interval_ms, grid_columns, grid_rows, tile_width, \
                  tile_height) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(vid)
            .bind(ordinal as i64)
            .bind(asset.interval_ms as i64)
            .bind(i64::from(asset.columns))
            .bind(i64::from(asset.rows))
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
                .bind(vid)
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

    async fn upsert_person(&self, person: Person) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_person");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT INTO people (id, name) VALUES (?, ?) \
             ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        )
        .bind(person.id.0.as_str())
        .bind(person.name.as_str())
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(
            &mut tx,
            "person",
            person.id.0.as_str(),
            person.name.as_str(),
        )
        .await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn get_person(&self, id: &PersonId) -> Result<Option<Person>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_person");
        let row = sqlx::query("SELECT id, name FROM people WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(Person {
            id: PersonId(column(&row, "id")?),
            name: column(&row, "name")?,
        }))
    }

    async fn set_title_enrichment(
        &self,
        owner: &TitleRef,
        enrichment: &TitleEnrichment,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "set_title_enrichment");
        let kind = title_kind_to_str(owner.kind());
        let id = owner.id();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        for table in [
            "credits",
            "title_genres",
            "title_studios",
            "title_ratings",
            "title_external_ids",
            "title_extras",
        ] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE title_kind = ? AND title_id = ?"
            ))
            .bind(kind)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, genre) in enrichment.genres.iter().enumerate() {
            sqlx::query(
                "INSERT INTO genres (id, name) VALUES (?, ?) \
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            )
            .bind(genre.id.0.as_str())
            .bind(genre.name.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            sqlx::query(
                "INSERT INTO title_genres (title_kind, title_id, ordinal, genre_id) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(genre.id.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, studio) in enrichment.studios.iter().enumerate() {
            sqlx::query(
                "INSERT INTO studios (id, name) VALUES (?, ?) \
                 ON CONFLICT(id) DO UPDATE SET name = excluded.name",
            )
            .bind(studio.id.0.as_str())
            .bind(studio.name.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
            sqlx::query(
                "INSERT INTO title_studios (title_kind, title_id, ordinal, studio_id) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(studio.id.0.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, credit) in enrichment.credits.iter().enumerate() {
            sqlx::query(
                "INSERT INTO credits \
                 (title_kind, title_id, ordinal, person_id, role, character, credit_order) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(credit.person.0.as_str())
            .bind(credit_role_to_str(credit.role))
            .bind(credit.character.as_deref())
            .bind(i64::from(credit.order))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, rating) in enrichment.ratings.iter().enumerate() {
            sqlx::query(
                "INSERT INTO title_ratings (title_kind, title_id, ordinal, source, value) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(rating.source.as_str())
            .bind(f64::from(rating.value))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, external) in enrichment.external_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO title_external_ids (title_kind, title_id, ordinal, source, value) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(external.source.as_str())
            .bind(external.value.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        for (ordinal, extra) in enrichment.extras.iter().enumerate() {
            sqlx::query(
                "INSERT INTO title_extras (title_kind, title_id, ordinal, kind, title, path) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(kind)
            .bind(id)
            .bind(ordinal as i64)
            .bind(extra_kind_to_str(extra.kind))
            .bind(extra.title.as_str())
            .bind(extra.path.as_str())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn movie_detail(&self, id: &MovieId) -> Result<Option<MovieDetail>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "movie_detail");
        let Some(movie) = self.get_movie(id).await? else {
            return Ok(None);
        };
        let tid = id.0.as_str();
        Ok(Some(MovieDetail {
            genres: self.load_genres("movie", tid).await?,
            credits: self.load_credits("movie", tid).await?,
            studios: self.load_studios("movie", tid).await?,
            ratings: self.load_ratings("movie", tid).await?,
            external_ids: self.load_external_ids("movie", tid).await?,
            extras: self.load_extras("movie", tid).await?,
            movie,
        }))
    }

    async fn series_detail(&self, id: &SeriesId) -> Result<Option<SeriesDetail>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "series_detail");
        let Some(series) = self.get_series(id).await? else {
            return Ok(None);
        };
        let tid = id.0.as_str();
        Ok(Some(SeriesDetail {
            genres: self.load_genres("series", tid).await?,
            credits: self.load_credits("series", tid).await?,
            studios: self.load_studios("series", tid).await?,
            ratings: self.load_ratings("series", tid).await?,
            external_ids: self.load_external_ids("series", tid).await?,
            extras: self.load_extras("series", tid).await?,
            series,
        }))
    }

    async fn filmography(&self, id: &PersonId) -> Result<Vec<Credit>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "filmography");
        let rows = sqlx::query(
            "SELECT title_kind, title_id, role, character, credit_order FROM credits \
             WHERE person_id = ? ORDER BY title_kind, title_id, ordinal",
        )
        .bind(id.0.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Credit {
                    person: id.clone(),
                    title: title_ref_from_parts(
                        &column::<String>(row, "title_kind")?,
                        column(row, "title_id")?,
                    )?,
                    role: credit_role_from_str(&column::<String>(row, "role")?)?,
                    character: column(row, "character")?,
                    order: column::<i64>(row, "credit_order")? as u32,
                })
            })
            .collect()
    }

    async fn list_genres(&self) -> Result<Vec<Genre>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_genres");
        let rows = sqlx::query("SELECT id, name FROM genres ORDER BY name, id")
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        rows.iter()
            .map(|row| {
                Ok(Genre {
                    id: GenreId(column(row, "id")?),
                    name: column(row, "name")?,
                })
            })
            .collect()
    }

    async fn list_movies_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> Result<Page<Movie>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_movies_by_genre");
        let count_row = sqlx::query(
            "SELECT COUNT(*) AS n FROM movies m JOIN title_genres tg ON tg.title_id = m.id \
             WHERE tg.title_kind = 'movie' AND tg.genre_id = ?",
        )
        .bind(genre.0.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(
            "SELECT m.* FROM movies m JOIN title_genres tg ON tg.title_id = m.id \
             WHERE tg.title_kind = 'movie' AND tg.genre_id = ? \
             ORDER BY m.added_at ASC, m.id ASC LIMIT ? OFFSET ?",
        )
        .bind(genre.0.as_str())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut items = rows
            .iter()
            .map(row_to_movie)
            .collect::<Result<Vec<_>, _>>()?;
        for movie in &mut items {
            movie.artwork = self.load_artwork("movie", &movie.id.0).await?;
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn list_series_by_genre(
        &self,
        genre: &GenreId,
        page: PageRequest,
    ) -> Result<Page<Series>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_series_by_genre");
        let count_row = sqlx::query(
            "SELECT COUNT(*) AS n FROM series e JOIN title_genres tg ON tg.title_id = e.id \
             WHERE tg.title_kind = 'series' AND tg.genre_id = ?",
        )
        .bind(genre.0.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query(
            "SELECT e.* FROM series e JOIN title_genres tg ON tg.title_id = e.id \
             WHERE tg.title_kind = 'series' AND tg.genre_id = ? \
             ORDER BY e.added_at ASC, e.id ASC LIMIT ? OFFSET ?",
        )
        .bind(genre.0.as_str())
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut items = rows
            .iter()
            .map(row_to_series)
            .collect::<Result<Vec<_>, _>>()?;
        for series in &mut items {
            series.artwork = self.load_artwork("series", &series.id.0).await?;
        }
        Ok(Page {
            items,
            total,
            offset: page.offset,
            limit: page.limit,
        })
    }

    async fn titles_in_library(
        &self,
        kind: TitleKind,
        library: &LibraryId,
    ) -> Result<Vec<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "titles_in_library");
        let rows = match kind {
            TitleKind::Movie => sqlx::query(
                "SELECT DISTINCT title_id AS value FROM versions \
                 WHERE title_kind = 'movie' AND library_id = ? ORDER BY title_id",
            )
            .bind(library.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?,
            TitleKind::Series => sqlx::query(
                "SELECT DISTINCT se.series_id AS value FROM versions v \
                 JOIN episodes e ON e.id = v.title_id \
                 JOIN seasons se ON se.id = e.season_id \
                 WHERE v.title_kind = 'episode' AND v.library_id = ? ORDER BY se.series_id",
            )
            .bind(library.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?,
        };
        rows.iter().map(|row| column(row, "value")).collect()
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

    #[tokio::test]
    async fn rejects_corrupt_credit_role() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
            .await
            .unwrap();
        sqlx::query("INSERT INTO people (id, name) VALUES ('p1', 'Person')")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO credits \
             (title_kind, title_id, ordinal, person_id, role, character, credit_order) \
             VALUES ('movie', 'm1', 0, 'p1', 'bogus', NULL, 0)",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.filmography(&PersonId("p1".into())).await.is_err());
    }

    #[tokio::test]
    async fn rejects_corrupt_filmography_title_kind() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
            .await
            .unwrap();
        sqlx::query("INSERT INTO people (id, name) VALUES ('p1', 'Person')")
            .execute(&repo.pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO credits \
             (title_kind, title_id, ordinal, person_id, role, character, credit_order) \
             VALUES ('bogus', 'm1', 0, 'p1', 'actor', NULL, 0)",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.filmography(&PersonId("p1".into())).await.is_err());
    }

    #[tokio::test]
    async fn rejects_corrupt_extra_kind() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db"))
            .await
            .unwrap();
        repo.insert_movie(Movie {
            id: MovieId("m1".into()),
            title: "Alpha".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            added_at: from_millis(0).unwrap(),
            artwork: Vec::new(),
        })
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO title_extras (title_kind, title_id, ordinal, kind, title, path) \
             VALUES ('movie', 'm1', 0, 'bogus', 'Trailer', '/x.mkv')",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.movie_detail(&MovieId("m1".into())).await.is_err());
    }
}
