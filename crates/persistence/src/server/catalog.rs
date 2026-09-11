use std::collections::{HashMap, HashSet};
use std::path::Path;

use domain::catalog::{
    ArtworkId, ArtworkOwner, ArtworkRef, ArtworkWidth, Collection, CollectionId, Episode,
    EpisodeContext, EpisodeId, Movie, MovieDetail, MovieId, RandomScope, Season, SeasonId, Series,
    SeriesDetail, SeriesId, SortOrder, TitleId, TitleKind, TitleListFilter, TitleRef, TitleSort,
    Version, VersionDetail, VersionId,
};
use domain::common::{LanguageCode, Page, PageRequest, Quality};
use domain::discovery::{SearchKind, SearchResult};
use domain::error::RepositoryError;
use domain::library::LibraryId;
use domain::media::{
    AudioTrack, Chapter, CreditsMarker, DetectedMarkers, EmbeddedSubtitleTrack, HdrFormat,
    IntroMarker, SubtitleFile, SubtitleFileId, SubtitleFormat, SubtitleSource, TrickplayAsset,
    VideoTrack,
};
use domain::metadata::{
    ContentRating, Credit, CreditedPerson, ExternalId, Extra, Genre, GenreId, Person, PersonId,
    Rating, Studio, StudioId, TitleEnrichment,
};
use domain::repository::{CatalogRepository, SearchIndex};
use domain::text::normalize_title;
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqliteRow;
use sqlx::{AssertSqlSafe, Executor, Sqlite, SqliteConnection, SqlitePool};

use crate::codec::{
    artwork_kind_from_str, artwork_kind_to_str, artwork_owner_parts, credit_role_from_str,
    credit_role_to_str, extra_kind_from_str, extra_kind_to_str, title_from_parts, title_kind,
    title_kind_to_str, title_ref_from_parts,
};
use crate::metrics::DbOpGuard;
use crate::pool::{backend, checkpoint, column, from_millis, open, ping, to_millis};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/catalog");

const LIST_SEASONS_SQL: &str = "SELECT * FROM seasons WHERE series_id = ? ORDER BY number, id";
const LIST_EPISODES_SQL: &str = "SELECT * FROM episodes WHERE season_id = ? ORDER BY number, id";
const LIST_VERSIONS_SQL: &str =
    "SELECT * FROM versions WHERE title_kind = ? AND title_id = ? ORDER BY id LIMIT ? OFFSET ?";
const EPISODES_BY_SERIES_SQL: &str = "SELECT e.id AS episode_id, sn.series_id AS parent \
     FROM episodes e JOIN seasons sn ON sn.id = e.season_id \
     WHERE sn.series_id IN ({}) ORDER BY sn.number, e.number, e.id";
const EPISODES_BY_SEASON_SQL: &str = "SELECT e.id AS episode_id, e.season_id AS parent \
     FROM episodes e WHERE e.season_id IN ({}) ORDER BY e.number, e.id";

const BIND_CHUNK: usize = 500;

const REINDEX_CHUNK: usize = 1_000;

const DELETE_MOVIE_SQL: &str = "DELETE FROM movies WHERE id = ? AND NOT EXISTS \
     (SELECT 1 FROM versions WHERE title_kind = 'movie' AND title_id = ?)";
const DELETE_SERIES_SQL: &str = "DELETE FROM series WHERE id = ? AND NOT EXISTS \
     (SELECT 1 FROM seasons WHERE series_id = ?)";
const DELETE_SEASON_SQL: &str = "DELETE FROM seasons WHERE id = ? AND NOT EXISTS \
     (SELECT 1 FROM episodes WHERE season_id = ?)";
const DELETE_EPISODE_SQL: &str = "DELETE FROM episodes WHERE id = ? AND NOT EXISTS \
     (SELECT 1 FROM versions WHERE title_kind = 'episode' AND title_id = ?)";

const TITLE_ATTRIBUTE_CLEANUP: [&str; 8] = [
    "DELETE FROM artwork WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM credits WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM title_genres WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM title_studios WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM title_ratings WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM title_external_ids WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM title_extras WHERE title_kind = ? AND title_id = ?",
    "DELETE FROM search_index WHERE kind = ? AND id = ?",
];

const SERIES_EPISODE_COUNTS_SQL: &str = "SELECT COUNT(*) AS total, \
     COALESCE(SUM(CASE WHEN EXISTS \
         (SELECT 1 FROM versions v \
          WHERE v.title_kind = 'episode' AND v.title_id = e.id AND v.available = 1) \
     THEN 1 ELSE 0 END), 0) AS playable \
     FROM episodes e JOIN seasons sn ON sn.id = e.season_id \
     WHERE sn.series_id = ?";

fn mark_present_sql(paths: usize) -> String {
    let placeholders = vec!["?"; paths].join(", ");
    format!("UPDATE versions SET available = 1 WHERE library_id = ? AND path IN ({placeholders})")
}

const EPISODE_CONTEXT_FROM: &str = "SELECT e.*, sn.number AS season_no, sn.title AS season_name, \
     sr.id AS series_ref \
     FROM episodes e \
     JOIN seasons sn ON sn.id = e.season_id \
     JOIN series sr ON sr.id = sn.series_id";

#[derive(Clone)]
pub struct SqliteCatalogRepo {
    pool: SqlitePool,
}

impl SqliteCatalogRepo {
    pub async fn connect(path: &Path) -> Result<Self, RepositoryError> {
        Ok(Self { pool: open(path, &MIGRATOR).await? })
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
            "INSERT INTO movies \
             (id, title, sort_title, year, overview, runtime_minutes, rating_system, rating_code, \
              manually_edited, added_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 title = excluded.title, \
                 sort_title = excluded.sort_title, \
                 year = excluded.year, \
                 overview = excluded.overview, \
                 runtime_minutes = excluded.runtime_minutes, \
                 rating_system = excluded.rating_system, \
                 rating_code = excluded.rating_code, \
                 manually_edited = excluded.manually_edited, \
                 updated_at = excluded.updated_at",
        )
        .bind(movie.id.0.as_str())
        .bind(movie.title.as_str())
        .bind(movie.sort_title.as_str())
        .bind(movie.year.map(i64::from))
        .bind(movie.overview.as_deref())
        .bind(movie.runtime_minutes.map(i64::from))
        .bind(movie.content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(movie.content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(i64::from(movie.manually_edited))
        .bind(to_millis(movie.added_at))
        .bind(to_millis(movie.updated_at))
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
            "INSERT INTO series \
             (id, title, sort_title, year, overview, rating_system, rating_code, \
              manually_edited, added_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 title = excluded.title, \
                 sort_title = excluded.sort_title, \
                 year = excluded.year, \
                 overview = excluded.overview, \
                 rating_system = excluded.rating_system, \
                 rating_code = excluded.rating_code, \
                 manually_edited = excluded.manually_edited, \
                 updated_at = excluded.updated_at",
        )
        .bind(series.id.0.as_str())
        .bind(series.title.as_str())
        .bind(series.sort_title.as_str())
        .bind(series.year.map(i64::from))
        .bind(series.overview.as_deref())
        .bind(series.content_rating.as_ref().map(|r| r.system.as_str()))
        .bind(series.content_rating.as_ref().map(|r| r.code.as_str()))
        .bind(i64::from(series.manually_edited))
        .bind(to_millis(series.added_at))
        .bind(to_millis(series.updated_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(&mut tx, "series", series.id.0.as_str(), series.title.as_str()).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_season(&self, season: Season) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO seasons \
             (id, series_id, number, title, overview, added_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 series_id = excluded.series_id, \
                 number = excluded.number, \
                 title = excluded.title, \
                 overview = excluded.overview, \
                 updated_at = excluded.updated_at",
        )
        .bind(season.id.0.as_str())
        .bind(season.series.0.as_str())
        .bind(i64::from(season.number))
        .bind(season.title.as_deref())
        .bind(season.overview.as_deref())
        .bind(to_millis(season.added_at))
        .bind(to_millis(season.updated_at))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    pub async fn insert_episode(&self, episode: Episode) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT INTO episodes \
             (id, season_id, number, title, overview, runtime_minutes, air_date, \
              manually_edited, added_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 season_id = excluded.season_id, \
                 number = excluded.number, \
                 title = excluded.title, \
                 overview = excluded.overview, \
                 runtime_minutes = excluded.runtime_minutes, \
                 air_date = excluded.air_date, \
                 manually_edited = excluded.manually_edited, \
                 updated_at = excluded.updated_at",
        )
        .bind(episode.id.0.as_str())
        .bind(episode.season.0.as_str())
        .bind(i64::from(episode.number))
        .bind(episode.title.as_str())
        .bind(episode.overview.as_deref())
        .bind(episode.runtime_minutes.map(i64::from))
        .bind(episode.air_date.map(to_millis))
        .bind(i64::from(episode.manually_edited))
        .bind(to_millis(episode.added_at))
        .bind(to_millis(episode.updated_at))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(&mut tx, "episode", episode.id.0.as_str(), episode.title.as_str()).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    pub async fn insert_collection(&self, collection: Collection) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let id = collection.id.0.as_str();
        sqlx::query(
            "INSERT INTO collections (id, name, overview, added_at, updated_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 name = excluded.name, \
                 overview = excluded.overview, \
                 updated_at = excluded.updated_at",
        )
        .bind(id)
        .bind(collection.name.as_str())
        .bind(collection.overview.as_deref())
        .bind(to_millis(collection.added_at))
        .bind(to_millis(collection.updated_at))
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
        for (ordinal, file) in detail.subtitle_files.iter().enumerate() {
            sqlx::query(
                "INSERT INTO subtitle_files \
                 (id, version_id, ordinal, language, format, source, path, translated_from, \
                  label, pinned) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(file.id.0.as_str())
            .bind(vid.as_str())
            .bind(ordinal as i64)
            .bind(file.language.as_ref().map(|l| l.0.as_str()))
            .bind(subtitle_format_to_str(file.format))
            .bind(subtitle_source_to_str(file.source))
            .bind(file.path.as_str())
            .bind(file.translated_from.as_ref().map(|id| id.0.as_str()))
            .bind(file.label.as_deref())
            .bind(file.pinned as i64)
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
        Ok(self.load_collections(std::slice::from_ref(row)).await?.remove(0))
    }

    async fn load_collections(
        &self,
        rows: &[SqliteRow],
    ) -> Result<Vec<Collection>, RepositoryError> {
        let ids =
            rows.iter().map(|row| column::<String>(row, "id")).collect::<Result<Vec<_>, _>>()?;
        let mut members = self.collection_members(&ids).await?;
        let mut artwork = self.load_artwork_batch("collection", &ids).await?;
        rows.iter()
            .zip(ids)
            .map(|(row, id)| {
                Ok(Collection {
                    name: column(row, "name")?,
                    overview: column(row, "overview")?,
                    movies: members.remove(&id).unwrap_or_default(),
                    added_at: from_millis(column(row, "added_at")?)?,
                    updated_at: from_millis(column(row, "updated_at")?)?,
                    artwork: artwork.remove(&id).unwrap_or_default(),
                    id: CollectionId(id),
                })
            })
            .collect()
    }

    async fn live_ids(
        &self,
        select: &str,
        ids: &[String],
    ) -> Result<HashSet<String>, RepositoryError> {
        let mut live = HashSet::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let mut query = sqlx::query(AssertSqlSafe(format!("{select} ({placeholders})")));
            for id in chunk {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for row in &rows {
                live.insert(column::<String>(row, "id")?);
            }
        }
        Ok(live)
    }

    async fn collection_members(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, Vec<MovieId>>, RepositoryError> {
        let mut out: HashMap<String, Vec<MovieId>> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT collection_id, movie_id FROM collection_movies \
                 WHERE collection_id IN ({placeholders}) ORDER BY collection_id, ordinal"
            );
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for row in &rows {
                out.entry(column(row, "collection_id")?)
                    .or_default()
                    .push(MovieId(column(row, "movie_id")?));
            }
        }
        Ok(out)
    }

    async fn load_artwork(
        &self,
        title_kind: &str,
        title_id: &str,
    ) -> Result<Vec<ArtworkRef>, RepositoryError> {
        Ok(self
            .load_artwork_batch(title_kind, std::slice::from_ref(&title_id.to_owned()))
            .await?
            .remove(title_id)
            .unwrap_or_default())
    }

    async fn search_total(
        &self,
        branch: &SearchBranch,
        gate: &[String],
    ) -> Result<u64, RepositoryError> {
        let mut query = sqlx::query(AssertSqlSafe(branch.count.clone())).bind(&branch.match_query);
        for _ in 0..branch.rank_binds {
            query = query.bind(&branch.needle);
        }
        for bind in gate {
            query = query.bind(bind);
        }
        let row = query.fetch_one(&self.pool).await.map_err(backend)?;
        Ok(column::<i64>(&row, "n")?.max(0) as u64)
    }

    async fn search_page(
        &self,
        branch: &SearchBranch,
        gate: &[String],
        ceiling: i64,
    ) -> Result<Vec<SqliteRow>, RepositoryError> {
        let mut query = sqlx::query(AssertSqlSafe(branch.page.clone()))
            .bind(&branch.needle)
            .bind(&branch.needle)
            .bind(&branch.match_query);
        for _ in 0..branch.rank_binds {
            query = query.bind(&branch.needle);
        }
        for bind in gate {
            query = query.bind(bind);
        }
        query.bind(ceiling).fetch_all(&self.pool).await.map_err(backend)
    }

    async fn hydrate_search_artwork(
        &self,
        items: &mut [SearchResult],
    ) -> Result<(), RepositoryError> {
        let mut ids: HashMap<&str, Vec<String>> = HashMap::new();
        for item in items.iter() {
            let (kind, id) = search_artwork_owner(item);
            ids.entry(kind).or_default().push(id.to_owned());
        }
        let mut loaded: HashMap<&str, HashMap<String, Vec<ArtworkRef>>> = HashMap::new();
        for (kind, group) in &ids {
            loaded.insert(kind, self.load_artwork_batch(kind, group).await?);
        }
        for item in items.iter_mut() {
            let (kind, id) = search_artwork_owner(item);
            let found = loaded.get_mut(kind).and_then(|group| group.remove(id)).unwrap_or_default();
            match item {
                SearchResult::Movie(movie) => movie.artwork = found,
                SearchResult::Series(series) => series.artwork = found,
                SearchResult::Episode(episode) => episode.artwork = found,
                SearchResult::Person(person) => person.artwork = found,
            }
        }
        Ok(())
    }

    async fn grouped_episode_ids(
        &self,
        template: &str,
        ids: &[&str],
    ) -> Result<HashMap<String, Vec<EpisodeId>>, RepositoryError> {
        let mut out: HashMap<String, Vec<EpisodeId>> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let mut query =
                sqlx::query(AssertSqlSafe(template.replace("{}", &placeholders).to_string()));
            for id in chunk {
                query = query.bind(*id);
            }
            for row in &query.fetch_all(&self.pool).await.map_err(backend)? {
                out.entry(column(row, "parent")?)
                    .or_default()
                    .push(EpisodeId(column(row, "episode_id")?));
            }
        }
        Ok(out)
    }

    async fn load_artwork_batch(
        &self,
        title_kind: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Vec<ArtworkRef>>, RepositoryError> {
        let mut out: HashMap<String, Vec<ArtworkRef>> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            out.extend(self.load_artwork_chunk(title_kind, chunk).await?);
        }
        Ok(out)
    }

    async fn load_artwork_widths(
        &self,
        artwork_ids: &[String],
    ) -> Result<HashMap<String, Vec<ArtworkWidth>>, RepositoryError> {
        let mut widths: HashMap<String, Vec<ArtworkWidth>> = HashMap::new();
        for chunk in artwork_ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT artwork_id, width, path FROM artwork_widths \
                 WHERE artwork_id IN ({placeholders}) ORDER BY artwork_id, ordinal"
            );
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for row in &rows {
                let artwork_id: String = column(row, "artwork_id")?;
                let width = column::<i64>(row, "width")? as u32;
                widths
                    .entry(artwork_id)
                    .or_default()
                    .push(ArtworkWidth::new(width, column::<String>(row, "path")?));
            }
        }
        Ok(widths)
    }

    async fn load_artwork_chunk(
        &self,
        title_kind: &str,
        ids: &[String],
    ) -> Result<HashMap<String, Vec<ArtworkRef>>, RepositoryError> {
        let placeholders = vec!["?"; ids.len()].join(", ");
        let art_sql = format!(
            "SELECT title_id, artwork_id, kind FROM artwork \
             WHERE title_kind = ? AND title_id IN ({placeholders}) ORDER BY title_id, ordinal"
        );
        let mut art_query = sqlx::query(AssertSqlSafe(art_sql)).bind(title_kind);
        for id in ids {
            art_query = art_query.bind(id);
        }
        let art_rows = art_query.fetch_all(&self.pool).await.map_err(backend)?;
        let artwork_ids = art_rows
            .iter()
            .map(|row| column::<String>(row, "artwork_id"))
            .collect::<Result<Vec<_>, _>>()?;
        let widths = self.load_artwork_widths(&artwork_ids).await?;
        let mut out: HashMap<String, Vec<ArtworkRef>> = HashMap::new();
        for row in &art_rows {
            let title_id: String = column(row, "title_id")?;
            let artwork_id: String = column(row, "artwork_id")?;
            let kind = artwork_kind_from_str(&column::<String>(row, "kind")?)?;
            let widths = widths.get(&artwork_id).cloned().unwrap_or_default();
            out.entry(title_id).or_default().push(ArtworkRef {
                kind,
                id: ArtworkId(artwork_id),
                widths,
            });
        }
        Ok(out)
    }

    async fn hydrate_contexts(
        &self,
        rows: Vec<SqliteRow>,
    ) -> Result<Vec<EpisodeContext>, RepositoryError> {
        let mut items = rows
            .iter()
            .map(|row| {
                Ok(EpisodeContext {
                    season: SeasonId(column(row, "season_id")?),
                    season_number: column::<i64>(row, "season_no")? as u16,
                    season_title: column(row, "season_name")?,
                    series: SeriesId(column(row, "series_ref")?),
                    episode: row_to_episode(row)?,
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        let ids: Vec<String> = items.iter().map(|item| item.episode.id.0.clone()).collect();
        let mut artwork = self.load_artwork_batch("episode", &ids).await?;
        for item in &mut items {
            item.episode.artwork = artwork.remove(&item.episode.id.0).unwrap_or_default();
        }
        Ok(items)
    }

    async fn hydrate_movies(&self, rows: Vec<SqliteRow>) -> Result<Vec<Movie>, RepositoryError> {
        let mut items = rows.iter().map(row_to_movie).collect::<Result<Vec<_>, _>>()?;
        let ids: Vec<String> = items.iter().map(|movie| movie.id.0.clone()).collect();
        let mut artwork = self.load_artwork_batch("movie", &ids).await?;
        for movie in &mut items {
            movie.artwork = artwork.remove(&movie.id.0).unwrap_or_default();
        }
        Ok(items)
    }

    async fn hydrate_series(&self, rows: Vec<SqliteRow>) -> Result<Vec<Series>, RepositoryError> {
        let mut items = rows.iter().map(row_to_series).collect::<Result<Vec<_>, _>>()?;
        let ids: Vec<String> = items.iter().map(|series| series.id.0.clone()).collect();
        let mut artwork = self.load_artwork_batch("series", &ids).await?;
        for series in &mut items {
            series.artwork = artwork.remove(&series.id.0).unwrap_or_default();
        }
        Ok(items)
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

    async fn load_subtitle_files(
        &self,
        version_id: &str,
    ) -> Result<Vec<SubtitleFile>, RepositoryError> {
        let rows =
            sqlx::query("SELECT * FROM subtitle_files WHERE version_id = ? ORDER BY ordinal")
                .bind(version_id)
                .fetch_all(&self.pool)
                .await
                .map_err(backend)?;
        rows.iter().map(row_to_subtitle_file).collect()
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

    async fn delete_empty_title(
        &self,
        delete_sql: &'static str,
        kind: &str,
        id: &str,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        let removed = sqlx::query(delete_sql)
            .bind(id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(backend)?
            .rows_affected();
        if removed == 0 {
            return Ok(false);
        }
        for sql in TITLE_ATTRIBUTE_CLEANUP {
            sqlx::query(sql).bind(kind).bind(id).execute(&mut *tx).await.map_err(backend)?;
        }
        if kind == "movie" {
            sqlx::query("DELETE FROM collection_movies WHERE movie_id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(true)
    }

    async fn series_episode_counts(&self, series: &str) -> Result<(u32, u32), RepositoryError> {
        let row = sqlx::query(SERIES_EPISODE_COUNTS_SQL)
            .bind(series)
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&row, "total")? as u32;
        let playable = column::<i64>(&row, "playable")? as u32;
        Ok((total, playable))
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
            .map(|row| Ok(Genre { id: GenreId(column(row, "id")?), name: column(row, "name")? }))
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
                        ..Person::default()
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
            .map(|row| Ok(Studio { id: StudioId(column(row, "id")?), name: column(row, "name")? }))
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
                Ok(ExternalId { source: column(row, "source")?, value: column(row, "value")? })
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

fn subtitle_source_to_str(source: SubtitleSource) -> &'static str {
    match source {
        SubtitleSource::OpenSubtitles => "opensubtitles",
        SubtitleSource::External => "external",
        SubtitleSource::Generated => "generated",
        SubtitleSource::MachineTranslated => "machine_translated",
        SubtitleSource::Combined => "combined",
    }
}

fn subtitle_source_from_str(value: &str) -> Result<SubtitleSource, RepositoryError> {
    match value {
        "opensubtitles" => Ok(SubtitleSource::OpenSubtitles),
        "external" => Ok(SubtitleSource::External),
        "generated" => Ok(SubtitleSource::Generated),
        "machine_translated" => Ok(SubtitleSource::MachineTranslated),
        "combined" => Ok(SubtitleSource::Combined),
        other => Err(backend(format!("unknown subtitle source: {other}"))),
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
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> Result<Page<SearchResult>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "search");
        let needle = normalize_title(query);
        let mut candidates: Vec<SearchResult> = Vec::new();
        let mut total = 0;
        if !needle.is_empty() {
            let ceiling = i64::from(page.offset) + i64::from(page.limit);
            let want = |kind: SearchKind| types.is_empty() || types.contains(&kind);
            if want(SearchKind::Movie) {
                let mut gate_binds = Vec::new();
                let gate = movie_gate(filter, &mut gate_binds);
                let branch = search_branch("movie", "movies m", "m", "", "m.*", &needle, &gate);
                total += self.search_total(&branch, &gate_binds).await?;
                for row in &self.search_page(&branch, &gate_binds, ceiling).await? {
                    candidates.push(SearchResult::Movie(row_to_movie(row)?));
                }
            }
            if want(SearchKind::Series) {
                let mut gate_binds = Vec::new();
                let gate = series_gate(filter, &mut gate_binds);
                let branch = search_branch("series", "series e", "e", "", "e.*", &needle, &gate);
                total += self.search_total(&branch, &gate_binds).await?;
                for row in &self.search_page(&branch, &gate_binds, ceiling).await? {
                    candidates.push(SearchResult::Series(row_to_series(row)?));
                }
            }
            if want(SearchKind::Episode) {
                let mut gate_binds = Vec::new();
                let gate = episode_gate(filter, &mut gate_binds);
                let branch = search_branch(
                    "episode",
                    "episodes e",
                    "e",
                    " LEFT JOIN seasons sn ON sn.id = e.season_id \
                     LEFT JOIN series sr ON sr.id = sn.series_id",
                    "e.*",
                    &needle,
                    &gate,
                );
                total += self.search_total(&branch, &gate_binds).await?;
                for row in &self.search_page(&branch, &gate_binds, ceiling).await? {
                    candidates.push(SearchResult::Episode(row_to_episode(row)?));
                }
            }
            if want(SearchKind::Person) {
                let branch =
                    search_branch("person", "people p", "p", "", "p.id, p.name", &needle, "");
                total += self.search_total(&branch, &[]).await?;
                for row in &self.search_page(&branch, &[], ceiling).await? {
                    candidates.push(SearchResult::Person(Person {
                        id: PersonId(column(row, "id")?),
                        name: column(row, "name")?,
                        ..Person::default()
                    }));
                }
            }
        }
        let mut results = domain::discovery::search(&candidates, query, types, page);
        results.total = total;
        self.hydrate_search_artwork(&mut results.items).await?;
        Ok(results)
    }

    async fn rebuild(&self) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "rebuild");
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM search_index").execute(&mut *tx).await.map_err(backend)?;
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
    select: &'static str,
) -> Result<(), RepositoryError> {
    let rows = sqlx::query(select).fetch_all(&mut *conn).await.map_err(backend)?;
    let indexed = rows
        .iter()
        .map(|row| {
            Ok((column::<String>(row, "id")?, normalize_title(&column::<String>(row, "title")?)))
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;
    for chunk in indexed.chunks(REINDEX_CHUNK) {
        let values = vec!["(?, ?, ?)"; chunk.len()].join(", ");
        let mut query = sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO search_index (kind, id, title) VALUES {values}"
        )));
        for (id, title) in chunk {
            query = query.bind(kind).bind(id).bind(title);
        }
        query.execute(&mut *conn).await.map_err(backend)?;
    }
    Ok(())
}

fn search_artwork_owner(result: &SearchResult) -> (&'static str, &str) {
    match result {
        SearchResult::Movie(movie) => ("movie", movie.id.0.as_str()),
        SearchResult::Series(series) => ("series", series.id.0.as_str()),
        SearchResult::Episode(episode) => ("episode", episode.id.0.as_str()),
        SearchResult::Person(person) => ("person", person.id.0.as_str()),
    }
}

struct SearchBranch {
    count: String,
    page: String,
    rank_binds: usize,
    needle: String,
    match_query: String,
}

fn rank_where(needle: &str) -> &'static str {
    if needle.contains(' ') {
        " AND (s.title = ? OR s.title LIKE ? || '%')"
    } else {
        " AND (s.title = ? OR s.title LIKE ? || '%' OR s.title LIKE '% ' || ? || '%')"
    }
}

fn search_branch(
    kind: &str,
    from: &str,
    alias: &str,
    joins: &str,
    columns: &str,
    needle: &str,
    gate: &str,
) -> SearchBranch {
    let predicate = rank_where(needle);
    let body = format!(
        "FROM search_index s JOIN {from} ON {alias}.id = s.id{joins} \
         WHERE search_index MATCH ?{predicate}{gate}"
    );
    SearchBranch {
        count: format!("SELECT COUNT(*) AS n {body}"),
        page: format!(
            "SELECT {columns}, \
             CASE WHEN s.title = ? THEN 0 WHEN s.title LIKE ? || '%' THEN 1 ELSE 2 END AS tier \
             {body} ORDER BY tier, s.title, {alias}.id LIMIT ?"
        ),
        rank_binds: if needle.contains(' ') { 2 } else { 3 },
        needle: needle.to_owned(),
        match_query: fts_match(kind, needle),
    }
}

fn fts_match(kind: &str, needle: &str) -> String {
    let terms =
        needle.split_whitespace().map(|token| format!("{token}*")).collect::<Vec<_>>().join(" ");
    format!("kind:{kind} AND title:({terms})")
}

fn row_to_movie(row: &SqliteRow) -> Result<Movie, RepositoryError> {
    Ok(Movie {
        id: MovieId(column(row, "id")?),
        title: column(row, "title")?,
        sort_title: column(row, "sort_title")?,
        year: column::<Option<i64>>(row, "year")?.map(|y| y as u16),
        overview: column(row, "overview")?,
        runtime_minutes: column::<Option<i64>>(row, "runtime_minutes")?.map(|v| v as u32),
        content_rating: content_rating(row)?,
        manually_edited: column::<i64>(row, "manually_edited")? != 0,
        added_at: from_millis(column(row, "added_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
        artwork: Vec::new(),
    })
}

fn row_to_series(row: &SqliteRow) -> Result<Series, RepositoryError> {
    Ok(Series {
        id: SeriesId(column(row, "id")?),
        title: column(row, "title")?,
        sort_title: column(row, "sort_title")?,
        year: column::<Option<i64>>(row, "year")?.map(|y| y as u16),
        overview: column(row, "overview")?,
        content_rating: content_rating(row)?,
        manually_edited: column::<i64>(row, "manually_edited")? != 0,
        added_at: from_millis(column(row, "added_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
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
        added_at: from_millis(column(row, "added_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
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
        air_date: column::<Option<i64>>(row, "air_date")?.map(from_millis).transpose()?,
        manually_edited: column::<i64>(row, "manually_edited")? != 0,
        added_at: from_millis(column(row, "added_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
        artwork: Vec::new(),
    })
}

fn row_to_version(row: &SqliteRow) -> Result<Version, RepositoryError> {
    Ok(Version {
        id: VersionId(column(row, "id")?),
        title: title_from_parts(&column::<String>(row, "title_kind")?, column(row, "title_id")?)?,
        library: LibraryId(column(row, "library_id")?),
        quality: quality_from_str(&column::<String>(row, "quality")?)?,
        container: column(row, "container")?,
        path: column(row, "path")?,
        size_bytes: column::<i64>(row, "size_bytes")? as u64,
        duration_ms: column::<i64>(row, "duration_ms")? as u64,
        available: column(row, "available")?,
        added_at: from_millis(column(row, "added_at")?)?,
        updated_at: from_millis(column(row, "updated_at")?)?,
    })
}

fn row_to_video(row: &SqliteRow) -> Result<VideoTrack, RepositoryError> {
    Ok(VideoTrack {
        index: column::<i64>(row, "stream_index")? as u32,
        codec: column(row, "codec")?,
        width: column::<i64>(row, "width")? as u32,
        height: column::<i64>(row, "height")? as u32,
        bit_depth: column::<i64>(row, "bit_depth")? as u8,
        hdr: column::<Option<String>>(row, "hdr")?.map(|value| hdr_from_str(&value)).transpose()?,
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

fn row_to_subtitle_file(row: &SqliteRow) -> Result<SubtitleFile, RepositoryError> {
    Ok(SubtitleFile {
        id: SubtitleFileId(column::<String>(row, "id")?),
        version: VersionId(column::<String>(row, "version_id")?),
        language: column::<Option<String>>(row, "language")?.map(LanguageCode),
        format: subtitle_format_from_str(&column::<String>(row, "format")?)?,
        source: subtitle_source_from_str(&column::<String>(row, "source")?)?,
        path: column(row, "path")?,
        translated_from: column::<Option<String>>(row, "translated_from")?.map(SubtitleFileId),
        label: column::<Option<String>>(row, "label")?,
        pinned: column::<i64>(row, "pinned")? != 0,
    })
}

fn row_to_chapter(row: &SqliteRow) -> Result<Chapter, RepositoryError> {
    Ok(Chapter { title: column(row, "title")?, start_ms: column::<i64>(row, "start_ms")? as u64 })
}

async fn insert_version_row<'e, E>(executor: E, version: &Version) -> Result<(), RepositoryError>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO versions \
         (id, title_kind, title_id, library_id, quality, container, path, size_bytes, \
          duration_ms, available, added_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
             title_kind = excluded.title_kind, \
             title_id = excluded.title_id, \
             library_id = excluded.library_id, \
             quality = excluded.quality, \
             container = excluded.container, \
             path = excluded.path, \
             size_bytes = excluded.size_bytes, \
             duration_ms = excluded.duration_ms, \
             available = excluded.available, \
             updated_at = excluded.updated_at",
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
    .bind(version.available)
    .bind(to_millis(version.added_at))
    .bind(to_millis(version.updated_at))
    .execute(executor)
    .await
    .map_err(backend)?;
    Ok(())
}

async fn count_all(pool: &SqlitePool, query: &'static str) -> Result<u64, RepositoryError> {
    let row = sqlx::query(query).fetch_one(pool).await.map_err(backend)?;
    Ok(column::<i64>(&row, "n")? as u64)
}

fn recent_episodes_sql(gate: &str) -> String {
    format!("{EPISODE_CONTEXT_FROM} WHERE 1 = 1{gate} ORDER BY e.added_at DESC, e.id ASC LIMIT ?")
}

fn next_in_series_sql(gate: &str) -> String {
    format!(
        "{EPISODE_CONTEXT_FROM} \
         WHERE sr.id = ? AND (sn.number > ? OR (sn.number = ? AND e.number > ?)){gate} \
         ORDER BY sn.number ASC, e.number ASC, e.id ASC LIMIT 1"
    )
}

fn count_filtered_sql(from: &str, where_sql: &str) -> String {
    format!("SELECT COUNT(*) AS n FROM {from}{where_sql}")
}

fn list_page_sql(
    from: &str,
    alias: &str,
    where_sql: &str,
    sort: TitleSort,
    order: SortOrder,
) -> String {
    format!(
        "SELECT {alias}.* FROM {from}{where_sql} {} LIMIT ? OFFSET ?",
        order_by_sql(alias, sort, order)
    )
}

async fn count_filtered(
    pool: &SqlitePool,
    from: &str,
    where_sql: &str,
    binds: &[String],
) -> Result<u64, RepositoryError> {
    let sql = count_filtered_sql(from, where_sql);
    let mut query = sqlx::query(AssertSqlSafe(sql));
    for bind in binds {
        query = query.bind(bind);
    }
    let row = query.fetch_one(pool).await.map_err(backend)?;
    Ok(column::<i64>(&row, "n")? as u64)
}

fn order_by_sql(alias: &str, sort: TitleSort, order: SortOrder) -> String {
    let column = match sort {
        TitleSort::AddedAt => "added_at",
        TitleSort::Title => "sort_title",
        TitleSort::Year => "year",
    };
    let direction = match order {
        SortOrder::Asc => "ASC",
        SortOrder::Desc => "DESC",
    };
    format!("ORDER BY {alias}.{column} {direction}, {alias}.id ASC")
}

fn rating_clause(
    alias: &str,
    blocked: &[ContentRating],
    binds: &mut Vec<String>,
) -> Option<String> {
    if blocked.is_empty() {
        return None;
    }
    let placeholders = vec!["?"; blocked.len()].join(", ");
    for rating in blocked {
        binds.push(format!("{}::{}", rating.system.to_lowercase(), rating.code.to_lowercase()));
    }
    Some(format!(
        "({alias}.rating_system IS NULL OR \
         (LOWER({alias}.rating_system) || '::' || LOWER({alias}.rating_code)) NOT IN ({placeholders}))"
    ))
}

fn genre_clauses(
    kind: &str,
    alias: &str,
    genres: &[String],
    binds: &mut Vec<String>,
) -> Vec<String> {
    genres
        .iter()
        .map(|name| {
            binds.push(name.clone());
            format!(
                "EXISTS (SELECT 1 FROM title_genres tg \
                 JOIN genres g ON g.id = tg.genre_id \
                 WHERE tg.title_kind = '{kind}' AND tg.title_id = {alias}.id AND g.name = ?)"
            )
        })
        .collect()
}

fn library_clause(scope: &str, binds: &mut Vec<String>, libraries: &[LibraryId]) -> String {
    if libraries.is_empty() {
        return "0 = 1".to_owned();
    }
    let placeholders = vec!["?"; libraries.len()].join(", ");
    for library in libraries {
        binds.push(library.0.clone());
    }
    format!("EXISTS ({scope} AND v.library_id IN ({placeholders}))")
}

fn where_sql(clauses: Vec<String>) -> String {
    if clauses.is_empty() { String::new() } else { format!(" WHERE {}", clauses.join(" AND ")) }
}

fn movie_filter_where(filter: &TitleListFilter) -> (String, Vec<String>) {
    let mut binds = Vec::new();
    let mut clauses = Vec::new();
    clauses.extend(genre_clauses("movie", "m", &filter.genres, &mut binds));
    if let Some(clause) = rating_clause("m", &filter.blocked_ratings, &mut binds) {
        clauses.push(clause);
    }
    if let Some(libraries) = &filter.libraries {
        clauses.push(library_clause(
            "SELECT 1 FROM versions v WHERE v.title_kind = 'movie' AND v.title_id = m.id",
            &mut binds,
            libraries,
        ));
    }
    (where_sql(clauses), binds)
}

fn series_filter_where(filter: &TitleListFilter) -> (String, Vec<String>) {
    let mut binds = Vec::new();
    let mut clauses = Vec::new();
    clauses.extend(genre_clauses("series", "s", &filter.genres, &mut binds));
    if let Some(clause) = rating_clause("s", &filter.blocked_ratings, &mut binds) {
        clauses.push(clause);
    }
    if let Some(libraries) = &filter.libraries {
        clauses.push(library_clause(
            "SELECT 1 FROM seasons se \
             CROSS JOIN episodes e ON e.season_id = se.id \
             CROSS JOIN versions v ON v.title_id = e.id AND v.title_kind = 'episode' \
             WHERE se.series_id = s.id",
            &mut binds,
            libraries,
        ));
    }
    (where_sql(clauses), binds)
}

fn movie_gate(filter: &TitleListFilter, binds: &mut Vec<String>) -> String {
    let mut clauses = Vec::new();
    if let Some(clause) = rating_clause("m", &filter.blocked_ratings, binds) {
        clauses.push(clause);
    }
    if let Some(libraries) = &filter.libraries {
        clauses.push(library_clause(
            "SELECT 1 FROM versions v WHERE v.title_kind = 'movie' AND v.title_id = m.id",
            binds,
            libraries,
        ));
    }
    prefixed_and(clauses)
}

fn series_gate(filter: &TitleListFilter, binds: &mut Vec<String>) -> String {
    let mut clauses = Vec::new();
    if let Some(clause) = rating_clause("e", &filter.blocked_ratings, binds) {
        clauses.push(clause);
    }
    if let Some(libraries) = &filter.libraries {
        clauses.push(library_clause(
            "SELECT 1 FROM seasons se \
             CROSS JOIN episodes ep ON ep.season_id = se.id \
             CROSS JOIN versions v ON v.title_id = ep.id AND v.title_kind = 'episode' \
             WHERE se.series_id = e.id",
            binds,
            libraries,
        ));
    }
    prefixed_and(clauses)
}

fn episode_gate(filter: &TitleListFilter, binds: &mut Vec<String>) -> String {
    let mut clauses = Vec::new();
    if let Some(clause) = rating_clause("sr", &filter.blocked_ratings, binds) {
        clauses.push(clause);
    }
    if let Some(libraries) = &filter.libraries {
        clauses.push(library_clause(
            "SELECT 1 FROM versions v WHERE v.title_kind = 'episode' AND v.title_id = e.id",
            binds,
            libraries,
        ));
    }
    prefixed_and(clauses)
}

fn prefixed_and(clauses: Vec<String>) -> String {
    if clauses.is_empty() { String::new() } else { format!(" AND {}", clauses.join(" AND ")) }
}

fn playable_clause(
    scope: &str,
    binds: &mut Vec<String>,
    libraries: Option<&Vec<LibraryId>>,
) -> String {
    match libraries {
        Some(libraries) => library_clause(scope, binds, libraries),
        None => format!("EXISTS ({scope})"),
    }
}

fn random_movie_sql(
    collection: Option<&CollectionId>,
    filter: &TitleListFilter,
) -> (String, Vec<String>) {
    let mut binds = Vec::new();
    let mut clauses = Vec::new();
    let from = match collection {
        Some(id) => {
            binds.push(id.0.clone());
            clauses.push("cm.collection_id = ?".to_owned());
            "movies m JOIN collection_movies cm ON cm.movie_id = m.id"
        }
        None => "movies m",
    };
    clauses.extend(genre_clauses("movie", "m", &filter.genres, &mut binds));
    if let Some(clause) = rating_clause("m", &filter.blocked_ratings, &mut binds) {
        clauses.push(clause);
    }
    clauses.push(playable_clause(
        "SELECT 1 FROM versions v WHERE v.title_kind = 'movie' \
         AND v.title_id = m.id AND v.available = 1",
        &mut binds,
        filter.libraries.as_ref(),
    ));
    (
        format!("SELECT m.id AS id FROM {from}{} ORDER BY RANDOM() LIMIT 1", where_sql(clauses)),
        binds,
    )
}

fn random_episode_sql(
    series: Option<&SeriesId>,
    season: Option<&SeasonId>,
    filter: &TitleListFilter,
) -> (String, Vec<String>) {
    let mut binds = Vec::new();
    let mut clauses = Vec::new();
    if let Some(id) = season {
        binds.push(id.0.clone());
        clauses.push("se.id = ?".to_owned());
    }
    if let Some(id) = series {
        binds.push(id.0.clone());
        clauses.push("s.id = ?".to_owned());
    }
    clauses.extend(genre_clauses("series", "s", &filter.genres, &mut binds));
    if let Some(clause) = rating_clause("s", &filter.blocked_ratings, &mut binds) {
        clauses.push(clause);
    }
    clauses.push(playable_clause(
        "SELECT 1 FROM versions v WHERE v.title_kind = 'episode' \
         AND v.title_id = e.id AND v.available = 1",
        &mut binds,
        filter.libraries.as_ref(),
    ));
    (
        format!(
            "SELECT e.id AS id FROM episodes e \
             JOIN seasons se ON se.id = e.season_id \
             JOIN series s ON s.id = se.series_id{} ORDER BY RANDOM() LIMIT 1",
            where_sql(clauses)
        ),
        binds,
    )
}

fn random_sql(scope: &RandomScope, filter: &TitleListFilter) -> (String, Vec<String>, bool) {
    match scope {
        RandomScope::Movies => {
            let (sql, binds) = random_movie_sql(None, filter);
            (sql, binds, true)
        }
        RandomScope::Collection(id) => {
            let (sql, binds) = random_movie_sql(Some(id), filter);
            (sql, binds, true)
        }
        RandomScope::Episodes => {
            let (sql, binds) = random_episode_sql(None, None, filter);
            (sql, binds, false)
        }
        RandomScope::Series(id) => {
            let (sql, binds) = random_episode_sql(Some(id), None, filter);
            (sql, binds, false)
        }
        RandomScope::Season(id) => {
            let (sql, binds) = random_episode_sql(None, Some(id), filter);
            (sql, binds, false)
        }
    }
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
        let items = self.hydrate_movies(rows).await?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
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

    async fn movies_by_ids(&self, ids: &[MovieId]) -> Result<Vec<Movie>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "movies_by_ids");
        let mut found: HashMap<String, Movie> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!("SELECT * FROM movies WHERE id IN ({placeholders})");
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id.0.as_str());
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            let movie_ids = rows
                .iter()
                .map(|row| column::<String>(row, "id"))
                .collect::<Result<Vec<_>, _>>()?;
            let mut artwork = self.load_artwork_batch("movie", &movie_ids).await?;
            for row in &rows {
                let mut movie = row_to_movie(row)?;
                movie.artwork = artwork.remove(&movie.id.0).unwrap_or_default();
                found.insert(movie.id.0.clone(), movie);
            }
        }
        Ok(ids.iter().filter_map(|id| found.remove(&id.0)).collect::<Vec<_>>())
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
        let items = self.hydrate_series(rows).await?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
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

    async fn series_by_ids(&self, ids: &[SeriesId]) -> Result<Vec<Series>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "series_by_ids");
        let mut found: HashMap<String, Series> = HashMap::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!("SELECT * FROM series WHERE id IN ({placeholders})");
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for id in chunk {
                query = query.bind(id.0.as_str());
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            for series in self.hydrate_series(rows).await? {
                found.insert(series.id.0.clone(), series);
            }
        }
        Ok(ids.iter().filter_map(|id| found.remove(&id.0)).collect::<Vec<_>>())
    }

    async fn list_seasons(&self, series: &SeriesId) -> Result<Vec<Season>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_seasons");
        let rows = sqlx::query(LIST_SEASONS_SQL)
            .bind(series.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut seasons =
            rows.iter().map(row_to_season).collect::<Result<Vec<_>, RepositoryError>>()?;
        let ids: Vec<String> = seasons.iter().map(|season| season.id.0.clone()).collect();
        let mut artwork = self.load_artwork_batch("season", &ids).await?;
        for season in &mut seasons {
            season.artwork = artwork.remove(&season.id.0).unwrap_or_default();
        }
        Ok(seasons)
    }

    async fn list_episodes(&self, season: &SeasonId) -> Result<Vec<Episode>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_episodes");
        let rows = sqlx::query(LIST_EPISODES_SQL)
            .bind(season.0.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut episodes =
            rows.iter().map(row_to_episode).collect::<Result<Vec<_>, RepositoryError>>()?;
        let ids: Vec<String> = episodes.iter().map(|episode| episode.id.0.clone()).collect();
        let mut artwork = self.load_artwork_batch("episode", &ids).await?;
        for episode in &mut episodes {
            episode.artwork = artwork.remove(&episode.id.0).unwrap_or_default();
        }
        Ok(episodes)
    }

    async fn episode_ids_for_series(
        &self,
        series: &[SeriesId],
    ) -> Result<HashMap<SeriesId, Vec<EpisodeId>>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "episode_ids_for_series");
        let ids: Vec<&str> = series.iter().map(|id| id.0.as_str()).collect();
        Ok(self
            .grouped_episode_ids(EPISODES_BY_SERIES_SQL, &ids)
            .await?
            .into_iter()
            .map(|(parent, episodes)| (SeriesId(parent), episodes))
            .collect())
    }

    async fn episode_ids_for_seasons(
        &self,
        seasons: &[SeasonId],
    ) -> Result<HashMap<SeasonId, Vec<EpisodeId>>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "episode_ids_for_seasons");
        let ids: Vec<&str> = seasons.iter().map(|id| id.0.as_str()).collect();
        Ok(self
            .grouped_episode_ids(EPISODES_BY_SEASON_SQL, &ids)
            .await?
            .into_iter()
            .map(|(parent, episodes)| (SeasonId(parent), episodes))
            .collect())
    }

    async fn recent_episodes(
        &self,
        filter: &TitleListFilter,
        limit: u32,
    ) -> Result<Vec<EpisodeContext>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "recent_episodes");
        let mut binds = Vec::new();
        let gate = episode_gate(filter, &mut binds);
        let mut query = sqlx::query(AssertSqlSafe(recent_episodes_sql(&gate)));
        for bind in &binds {
            query = query.bind(bind);
        }
        let rows = query.bind(i64::from(limit)).fetch_all(&self.pool).await.map_err(backend)?;
        self.hydrate_contexts(rows).await
    }

    async fn visible_movies(
        &self,
        ids: &[MovieId],
        filter: &TitleListFilter,
    ) -> Result<Vec<Movie>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "visible_movies");
        let mut out = Vec::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let mut binds: Vec<String> = chunk.iter().map(|id| id.0.clone()).collect();
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let gate = movie_gate(filter, &mut binds);
            let sql = format!("SELECT m.* FROM movies m WHERE m.id IN ({placeholders}){gate}");
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for bind in &binds {
                query = query.bind(bind);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            out.extend(self.hydrate_movies(rows).await?);
        }
        Ok(out)
    }

    async fn visible_episodes(
        &self,
        ids: &[EpisodeId],
        filter: &TitleListFilter,
    ) -> Result<Vec<EpisodeContext>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "visible_episodes");
        let mut out = Vec::new();
        for chunk in ids.chunks(BIND_CHUNK) {
            let mut binds: Vec<String> = chunk.iter().map(|id| id.0.clone()).collect();
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let gate = episode_gate(filter, &mut binds);
            let sql = format!("{EPISODE_CONTEXT_FROM} WHERE e.id IN ({placeholders}){gate}");
            let mut query = sqlx::query(AssertSqlSafe(sql));
            for bind in &binds {
                query = query.bind(bind);
            }
            let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
            out.extend(self.hydrate_contexts(rows).await?);
        }
        Ok(out)
    }

    async fn next_episode_in_series(
        &self,
        series: &SeriesId,
        after_season: u16,
        after_number: u16,
        filter: &TitleListFilter,
    ) -> Result<Option<EpisodeContext>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "next_episode_in_series");
        let mut binds = Vec::new();
        let gate = episode_gate(filter, &mut binds);
        let mut query = sqlx::query(AssertSqlSafe(next_in_series_sql(&gate)))
            .bind(series.0.as_str())
            .bind(i64::from(after_season))
            .bind(i64::from(after_season))
            .bind(i64::from(after_number));
        for bind in &binds {
            query = query.bind(bind);
        }
        let rows = query.fetch_all(&self.pool).await.map_err(backend)?;
        Ok(self.hydrate_contexts(rows).await?.pop())
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
        let rows = sqlx::query(
            "SELECT * FROM collections ORDER BY name COLLATE NOCASE, id LIMIT ? OFFSET ?",
        )
        .bind(i64::from(page.limit))
        .bind(i64::from(page.offset))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let items = self.load_collections(&rows).await?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
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

    async fn collections_of_movie(
        &self,
        movie: &MovieId,
    ) -> Result<Vec<CollectionId>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "collections_of_movie");
        let rows = sqlx::query(
            "SELECT DISTINCT collection_id FROM collection_movies \
             WHERE movie_id = ? ORDER BY collection_id",
        )
        .bind(movie.0.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in &rows {
            ids.push(CollectionId(column::<String>(row, "collection_id")?));
        }
        Ok(ids)
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

    async fn series_versions(&self, series: &SeriesId) -> Result<Vec<Version>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "series_versions");
        let rows = sqlx::query(
            "SELECT v.* FROM versions v \
             JOIN episodes e ON e.id = v.title_id \
             JOIN seasons sn ON sn.id = e.season_id \
             WHERE v.title_kind = 'episode' AND sn.series_id = ? \
             ORDER BY sn.number, e.number, e.id, v.id",
        )
        .bind(series.0.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(row_to_version).collect()
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
        let rows = sqlx::query(LIST_VERSIONS_SQL)
            .bind(title_kind(title))
            .bind(title.id())
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let items = rows.iter().map(row_to_version).collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
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
        let items = rows.iter().map(row_to_version).collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
    }

    async fn list_all_versions(&self, page: PageRequest) -> Result<Page<Version>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_all_versions");
        let count_row = sqlx::query("SELECT COUNT(*) AS n FROM versions")
            .fetch_one(&self.pool)
            .await
            .map_err(backend)?;
        let total = column::<i64>(&count_row, "n")? as u64;
        let rows = sqlx::query("SELECT * FROM versions ORDER BY id LIMIT ? OFFSET ?")
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let items = rows.iter().map(row_to_version).collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
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
        let subtitle_files = self.load_subtitle_files(&vid).await?;
        let chapters = self.load_chapters(&vid).await?;
        let markers = self.load_markers(&vid).await?;
        let trickplay = self.load_trickplay(&vid).await?;
        Ok(Some(VersionDetail {
            version,
            video,
            audio,
            subtitles,
            subtitle_files,
            chapters,
            markers,
            trickplay,
        }))
    }

    async fn get_version(&self, id: &VersionId) -> Result<Option<Version>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_version");
        let row = sqlx::query("SELECT * FROM versions WHERE id = ?")
            .bind(id.0.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?;
        row.as_ref().map(row_to_version).transpose()
    }

    async fn delete_version(&self, id: &VersionId) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_version");
        sqlx::query("DELETE FROM versions WHERE id = ?")
            .bind(id.0.as_str())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn delete_movie(&self, id: &MovieId) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_movie");
        self.delete_empty_title(DELETE_MOVIE_SQL, "movie", id.0.as_str()).await
    }

    async fn delete_series(&self, id: &SeriesId) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_series");
        self.delete_empty_title(DELETE_SERIES_SQL, "series", id.0.as_str()).await
    }

    async fn delete_season(&self, id: &SeasonId) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_season");
        self.delete_empty_title(DELETE_SEASON_SQL, "season", id.0.as_str()).await
    }

    async fn delete_episode(&self, id: &EpisodeId) -> Result<bool, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "delete_episode");
        self.delete_empty_title(DELETE_EPISODE_SQL, "episode", id.0.as_str()).await
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
        for chunk in present_paths.chunks(BIND_CHUNK) {
            let mut query =
                sqlx::query(AssertSqlSafe(mark_present_sql(chunk.len()))).bind(library.0.as_str());
            for path in chunk {
                query = query.bind(path.as_str());
            }
            query.execute(&mut *tx).await.map_err(backend)?;
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
                    "INSERT INTO artwork_widths (artwork_id, ordinal, width, path) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind(art.id.0.as_str())
                .bind(width_ordinal as i64)
                .bind(i64::from(width.width))
                .bind(width.path.as_str())
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

    async fn all_artwork_ids(&self) -> Result<Vec<ArtworkId>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "all_artwork_ids");
        let rows = sqlx::query("SELECT artwork_id FROM artwork")
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in &rows {
            ids.push(ArtworkId(column::<String>(row, "artwork_id")?));
        }
        Ok(ids)
    }

    async fn all_version_ids(&self) -> Result<Vec<VersionId>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "all_version_ids");
        let rows =
            sqlx::query("SELECT id FROM versions").fetch_all(&self.pool).await.map_err(backend)?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in &rows {
            ids.push(VersionId(column::<String>(row, "id")?));
        }
        Ok(ids)
    }

    async fn live_artwork_ids(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "live_artwork_ids");
        self.live_ids("SELECT artwork_id AS id FROM artwork WHERE artwork_id IN", ids).await
    }

    async fn live_version_ids(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "live_version_ids");
        self.live_ids("SELECT id FROM versions WHERE id IN", ids).await
    }

    async fn live_artwork_paths(&self, ids: &[String]) -> Result<HashSet<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "live_artwork_paths");
        self.live_ids("SELECT path AS id FROM artwork_widths WHERE artwork_id IN", ids).await
    }

    async fn live_subtitle_paths(
        &self,
        ids: &[String],
    ) -> Result<HashSet<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "live_subtitle_paths");
        self.live_ids("SELECT path AS id FROM subtitle_files WHERE version_id IN", ids).await
    }

    async fn live_trickplay_paths(
        &self,
        ids: &[String],
    ) -> Result<HashSet<String>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "live_trickplay_paths");
        self.live_ids("SELECT path AS id FROM trickplay_sheets WHERE version_id IN", ids).await
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
        for table in ["video_tracks", "audio_tracks", "subtitle_tracks", "chapters"] {
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {table} WHERE version_id = ?")))
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
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {table} WHERE version_id = ?")))
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

    async fn set_subtitle_files(
        &self,
        version: &VersionId,
        files: &[SubtitleFile],
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "set_subtitle_files");
        let vid = version.0.as_str();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query("DELETE FROM subtitle_files WHERE version_id = ?")
            .bind(vid)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        for (ordinal, file) in files.iter().enumerate() {
            sqlx::query(
                "INSERT INTO subtitle_files \
                 (id, version_id, ordinal, language, format, source, path, translated_from, \
                  label, pinned) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(file.id.0.as_str())
            .bind(vid)
            .bind(ordinal as i64)
            .bind(file.language.as_ref().map(|l| l.0.as_str()))
            .bind(subtitle_format_to_str(file.format))
            .bind(subtitle_source_to_str(file.source))
            .bind(file.path.as_str())
            .bind(file.translated_from.as_ref().map(|id| id.0.as_str()))
            .bind(file.label.as_deref())
            .bind(file.pinned as i64)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn add_subtitle_file(
        &self,
        version: &VersionId,
        file: &SubtitleFile,
    ) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "add_subtitle_file");
        let vid = version.0.as_str();
        sqlx::query(
            "INSERT INTO subtitle_files \
             (id, version_id, ordinal, language, format, source, path, translated_from, \
              label, pinned) \
             SELECT ?, ?, COALESCE(MAX(ordinal), -1) + 1, ?, ?, ?, ?, ?, ?, ? \
             FROM subtitle_files WHERE version_id = ? \
             ON CONFLICT(id) DO UPDATE SET \
                 version_id = excluded.version_id, \
                 language = excluded.language, \
                 format = excluded.format, \
                 source = excluded.source, \
                 path = excluded.path, \
                 translated_from = excluded.translated_from, \
                 label = excluded.label, \
                 pinned = excluded.pinned",
        )
        .bind(file.id.0.as_str())
        .bind(vid)
        .bind(file.language.as_ref().map(|l| l.0.as_str()))
        .bind(subtitle_format_to_str(file.format))
        .bind(subtitle_source_to_str(file.source))
        .bind(file.path.as_str())
        .bind(file.translated_from.as_ref().map(|id| id.0.as_str()))
        .bind(file.label.as_deref())
        .bind(file.pinned as i64)
        .bind(vid)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn upsert_person(&self, person: Person) -> Result<(), RepositoryError> {
        let _op = DbOpGuard::new("catalog", "upsert_person");
        let also_known_as = if person.also_known_as.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&person.also_known_as).map_err(backend)?)
        };
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT INTO people \
                 (id, name, biography, birthday, deathday, place_of_birth, also_known_as, external_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
                 name = excluded.name, \
                 biography = COALESCE(excluded.biography, biography), \
                 birthday = COALESCE(excluded.birthday, birthday), \
                 deathday = COALESCE(excluded.deathday, deathday), \
                 place_of_birth = COALESCE(excluded.place_of_birth, place_of_birth), \
                 also_known_as = COALESCE(excluded.also_known_as, also_known_as), \
                 external_id = COALESCE(excluded.external_id, external_id)",
        )
        .bind(person.id.0.as_str())
        .bind(person.name.as_str())
        .bind(person.biography.as_deref())
        .bind(person.birthday.as_deref())
        .bind(person.deathday.as_deref())
        .bind(person.place_of_birth.as_deref())
        .bind(also_known_as.as_deref())
        .bind(person.external_id.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        sync_search_row(&mut tx, "person", person.id.0.as_str(), person.name.as_str()).await?;
        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn get_person(&self, id: &PersonId) -> Result<Option<Person>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "get_person");
        let row = sqlx::query(
            "SELECT id, name, biography, birthday, deathday, place_of_birth, also_known_as, \
                 external_id FROM people WHERE id = ?",
        )
        .bind(id.0.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let person_id: String = column(&row, "id")?;
        let also_known_as = column::<Option<String>>(&row, "also_known_as")?
            .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
            .unwrap_or_default();
        let artwork = self.load_artwork("person", &person_id).await?;
        Ok(Some(Person {
            id: PersonId(person_id),
            name: column(&row, "name")?,
            biography: column(&row, "biography")?,
            birthday: column(&row, "birthday")?,
            deathday: column(&row, "deathday")?,
            place_of_birth: column(&row, "place_of_birth")?,
            also_known_as,
            external_id: column(&row, "external_id")?,
            artwork,
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
            sqlx::query(AssertSqlSafe(format!(
                "DELETE FROM {table} WHERE title_kind = ? AND title_id = ?"
            )))
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
        let (genres, credits, studios, ratings, external_ids, extras) = tokio::try_join!(
            self.load_genres("movie", tid),
            self.load_credits("movie", tid),
            self.load_studios("movie", tid),
            self.load_ratings("movie", tid),
            self.load_external_ids("movie", tid),
            self.load_extras("movie", tid),
        )?;
        Ok(Some(MovieDetail { genres, credits, studios, ratings, external_ids, extras, movie }))
    }

    async fn series_detail(&self, id: &SeriesId) -> Result<Option<SeriesDetail>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "series_detail");
        let Some(series) = self.get_series(id).await? else {
            return Ok(None);
        };
        let tid = id.0.as_str();
        let (genres, credits, studios, ratings, external_ids, extras, counts) = tokio::try_join!(
            self.load_genres("series", tid),
            self.load_credits("series", tid),
            self.load_studios("series", tid),
            self.load_ratings("series", tid),
            self.load_external_ids("series", tid),
            self.load_extras("series", tid),
            self.series_episode_counts(tid),
        )?;
        Ok(Some(SeriesDetail {
            genres,
            credits,
            studios,
            ratings,
            external_ids,
            extras,
            series,
            episodes_total: counts.0,
            episodes_with_available_version: counts.1,
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

    async fn list_genres(&self, kind: Option<TitleKind>) -> Result<Vec<Genre>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_genres");
        let rows = match kind {
            Some(kind) => {
                sqlx::query(
                    "SELECT g.id, g.name FROM genres g \
                     JOIN title_genres tg ON tg.genre_id = g.id \
                     WHERE tg.title_kind = ? \
                     GROUP BY g.id, g.name ORDER BY g.name, g.id",
                )
                .bind(title_kind_to_str(kind))
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query("SELECT id, name FROM genres ORDER BY name, id")
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(backend)?;
        rows.iter()
            .map(|row| Ok(Genre { id: GenreId(column(row, "id")?), name: column(row, "name")? }))
            .collect()
    }

    async fn list_movies_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> Result<Page<Movie>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_movies_filtered");
        let (where_sql, binds) = movie_filter_where(filter);
        let total = count_filtered(&self.pool, "movies m", &where_sql, &binds).await?;
        let list_sql = list_page_sql("movies m", "m", &where_sql, filter.sort, filter.order);
        let mut list_query = sqlx::query(AssertSqlSafe(list_sql));
        for bind in &binds {
            list_query = list_query.bind(bind);
        }
        let rows = list_query
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let items = self.hydrate_movies(rows).await?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
    }

    async fn list_series_filtered(
        &self,
        filter: &TitleListFilter,
        page: PageRequest,
    ) -> Result<Page<Series>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "list_series_filtered");
        let (where_sql, binds) = series_filter_where(filter);
        let total = count_filtered(&self.pool, "series s", &where_sql, &binds).await?;
        let list_sql = list_page_sql("series s", "s", &where_sql, filter.sort, filter.order);
        let mut list_query = sqlx::query(AssertSqlSafe(list_sql));
        for bind in &binds {
            list_query = list_query.bind(bind);
        }
        let rows = list_query
            .bind(i64::from(page.limit))
            .bind(i64::from(page.offset))
            .fetch_all(&self.pool)
            .await
            .map_err(backend)?;
        let items = self.hydrate_series(rows).await?;
        Ok(Page { items, total, offset: page.offset, limit: page.limit })
    }

    async fn random_playable_title(
        &self,
        scope: &RandomScope,
        filter: &TitleListFilter,
    ) -> Result<Option<TitleId>, RepositoryError> {
        let _op = DbOpGuard::new("catalog", "random_playable_title");
        let (sql, binds, is_movie) = random_sql(scope, filter);
        let mut query = sqlx::query(AssertSqlSafe(sql));
        for bind in &binds {
            query = query.bind(bind);
        }
        let Some(row) = query.fetch_optional(&self.pool).await.map_err(backend)? else {
            return Ok(None);
        };
        let id: String = column(&row, "id")?;
        Ok(Some(if is_movie {
            TitleId::Movie(MovieId(id))
        } else {
            TitleId::Episode(EpisodeId(id))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn plan(pool: &SqlitePool, sql: &str, binds: &[&str]) -> String {
        let mut query = sqlx::query(AssertSqlSafe(format!("EXPLAIN QUERY PLAN {sql}")));
        for bind in binds {
            query = query.bind(*bind);
        }
        let rows = query.fetch_all(pool).await.unwrap();
        rows.iter()
            .map(|row| column::<String>(row, "detail").unwrap())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    fn viewer_filter(sort: TitleSort) -> TitleListFilter {
        TitleListFilter {
            genres: Vec::new(),
            libraries: Some(vec![LibraryId("lib-1".into())]),
            blocked_ratings: Vec::new(),
            sort,
            order: SortOrder::Asc,
        }
    }

    async fn collections_repo(
        dir: &std::path::Path,
        collections: u32,
        members: u32,
    ) -> SqliteCatalogRepo {
        let repo = SqliteCatalogRepo::connect(&dir.join("catalog.db")).await.unwrap();
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO collections (id, name, overview, added_at, updated_at) \
             WITH RECURSIVE c(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM c WHERE i < {}) \
             SELECT 'col' || i, 'Collection ' || i, NULL, 0, 0 FROM c",
            collections - 1
        )))
        .execute(&repo.pool)
        .await
        .unwrap();
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO collection_movies (collection_id, ordinal, movie_id) \
             WITH RECURSIVE c(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM c WHERE i < {}) \
             SELECT 'col' || (i / {members}), i % {members}, 'mv' || i FROM c",
            collections * members - 1
        )))
        .execute(&repo.pool)
        .await
        .unwrap();
        repo
    }

    #[tokio::test]
    async fn a_page_of_collections_keeps_each_ones_members_to_itself() {
        let dir = tempfile::tempdir().unwrap();
        let repo = collections_repo(dir.path(), 30, 20).await;

        let page = repo.list_collections(PageRequest { offset: 0, limit: 30 }).await.unwrap();

        assert_eq!(page.items.len(), 30);
        for collection in &page.items {
            let index: u32 = collection.id.0.trim_start_matches("col").parse().unwrap();
            let expected: Vec<MovieId> =
                (0..20).map(|ordinal| MovieId(format!("mv{}", index * 20 + ordinal))).collect();
            assert_eq!(
                collection.movies, expected,
                "batching must not cross members between collections, nor lose ordinal order"
            );
        }
    }

    #[tokio::test]
    async fn a_collection_with_no_members_still_appears_on_the_page() {
        let dir = tempfile::tempdir().unwrap();
        let repo = collections_repo(dir.path(), 3, 2).await;
        sqlx::query("INSERT INTO collections (id, name, overview, added_at, updated_at) VALUES ('empty', 'Zed Empty', NULL, 0, 0)")
            .execute(&repo.pool)
            .await
            .unwrap();

        let page = repo.list_collections(PageRequest { offset: 0, limit: 10 }).await.unwrap();

        let empty = page
            .items
            .iter()
            .find(|c| c.id.0 == "empty")
            .expect("a collection with no members is still a collection");
        assert!(empty.movies.is_empty());
        assert!(empty.artwork.is_empty());
        assert_eq!(page.total, 4);
    }

    #[tokio::test]
    async fn artwork_hydration_survives_more_ids_than_sqlite_allows_parameters() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        let rows = 40_000;
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO artwork (artwork_id, title_kind, title_id, ordinal, kind) \
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
             SELECT 'art-' || i, 'movie', 'm-' || i, 0, 'poster' FROM c"
        )))
        .execute(&repo.pool)
        .await
        .unwrap();
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO artwork_widths (artwork_id, ordinal, width, path) \
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
             SELECT 'art-' || i, 0, 480, '/art/art-' || i || '/480.jpg' FROM c"
        )))
        .execute(&repo.pool)
        .await
        .unwrap();

        let ids: Vec<String> = (1..rows).map(|i| format!("m-{i}")).collect();
        let loaded = repo.load_artwork_batch("movie", &ids).await.unwrap();

        assert_eq!(
            loaded.len(),
            ids.len(),
            "the home page binds one parameter per title and SQLite caps them at 32766, \
             so an unchunked hydration stops returning 200 rather than getting slow"
        );
        assert_eq!(
            loaded["m-1"][0].sizes(),
            vec![480],
            "the widths lookup binds one parameter per artwork row and needs the same guard"
        );
    }

    async fn searchable_repo(dir: &std::path::Path, titles: &[&str]) -> SqliteCatalogRepo {
        let repo = SqliteCatalogRepo::connect(&dir.join("catalog.db")).await.unwrap();
        for (n, title) in titles.iter().enumerate() {
            sqlx::query(
                "INSERT INTO movies (id, title, sort_title, added_at, updated_at) \
                 VALUES (?, ?, ?, 0, 0)",
            )
            .bind(format!("m-{n}"))
            .bind(*title)
            .bind(normalize_title(title))
            .execute(&repo.pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO search_index (kind, id, title) VALUES ('movie', ?, ?)")
                .bind(format!("m-{n}"))
                .bind(normalize_title(title))
                .execute(&repo.pool)
                .await
                .unwrap();
        }
        repo
    }

    fn unfiltered() -> TitleListFilter {
        TitleListFilter {
            genres: Vec::new(),
            libraries: None,
            blocked_ratings: Vec::new(),
            sort: TitleSort::Title,
            order: SortOrder::Asc,
        }
    }

    fn found(page: &Page<SearchResult>) -> Vec<String> {
        page.items
            .iter()
            .map(|item| {
                let SearchResult::Movie(movie) = item else { panic!("not a movie: {item:?}") };
                movie.title.clone()
            })
            .collect()
    }

    #[tokio::test]
    async fn search_reads_only_as_far_as_the_page_it_was_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        let rows = 5_000;
        for sql in [
            format!(
                "INSERT INTO movies (id, title, sort_title, added_at, updated_at) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
                 SELECT 'm-' || i, 'shared ' || i, 'shared ' || i, i, i FROM c"
            ),
            format!(
                "INSERT INTO search_index (kind, id, title) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
                 SELECT 'movie', 'm-' || i, 'shared ' || i FROM c"
            ),
        ] {
            sqlx::query(AssertSqlSafe(sql)).execute(&repo.pool).await.unwrap();
        }
        let branch = search_branch("movie", "movies m", "m", "", "m.*", "shared", "");

        let rows_read = repo.search_page(&branch, &[], 10).await.unwrap().len();

        assert_eq!(
            rows_read, 10,
            "every match used to come back as a domain object so Rust could sort them \
             and keep fifty; one common word matched the whole catalog"
        );
        let hits = repo
            .search("shared", &[], &unfiltered(), PageRequest { offset: 0, limit: 10 })
            .await
            .unwrap();
        assert_eq!(hits.items.len(), 10);
        assert_eq!(
            hits.total, rows as u64,
            "the count still reports every match, not just the page that was read"
        );
    }

    #[tokio::test]
    async fn search_ranking_survives_moving_into_sql() {
        let dir = tempfile::tempdir().unwrap();
        let repo = searchable_repo(
            dir.path(),
            &["Rematrix", "The Matrix", "Matrix Reloaded", "Matrix", "Watch Matrix Now"],
        )
        .await;

        let hits = repo
            .search("matrix", &[], &unfiltered(), PageRequest { offset: 0, limit: 10 })
            .await
            .unwrap();

        assert_eq!(
            found(&hits),
            vec!["Matrix", "Matrix Reloaded", "The Matrix", "Watch Matrix Now"],
            "exact first, then whole title prefix, then word prefix, each group \
             alphabetical; Rematrix is a substring only and never matched"
        );
        assert_eq!(hits.total, 4);
    }

    #[tokio::test]
    async fn a_one_row_page_returns_the_best_match_not_the_first_row() {
        let dir = tempfile::tempdir().unwrap();
        let mut titles: Vec<String> = (0..20).map(|n| format!("Aaa Matrix Bbb {n}")).collect();
        titles.push("Matrix".to_owned());
        let repo =
            searchable_repo(dir.path(), &titles.iter().map(String::as_str).collect::<Vec<_>>())
                .await;

        let hits = repo
            .search("matrix", &[], &unfiltered(), PageRequest { offset: 0, limit: 1 })
            .await
            .unwrap();

        assert_eq!(
            found(&hits),
            vec!["Matrix"],
            "SQL only hands back one row now, so it has to be the one the ranker \
             would have chosen rather than whichever was indexed first"
        );
        assert_eq!(hits.total, 21);
    }

    #[tokio::test]
    async fn a_small_page_is_not_filled_with_rows_the_ranker_throws_away() {
        let dir = tempfile::tempdir().unwrap();
        let mut titles: Vec<String> = (0..20).map(|n| format!("Chapter {n} The Matrix")).collect();
        titles.push("The Matrix".to_owned());
        let repo =
            searchable_repo(dir.path(), &titles.iter().map(String::as_str).collect::<Vec<_>>())
                .await;

        let hits = repo
            .search("the matrix", &[], &unfiltered(), PageRequest { offset: 0, limit: 1 })
            .await
            .unwrap();

        assert_eq!(
            found(&hits),
            vec!["The Matrix"],
            "the full text index matches every title carrying both words, but a \
             multi word query has to start the title, so SQL has to apply that \
             rule before the limit or the page comes back empty"
        );
        assert_eq!(hits.total, 1);
    }

    #[tokio::test]
    async fn a_search_for_a_kind_name_returns_only_the_titles_that_say_it() {
        let dir = tempfile::tempdir().unwrap();
        let repo = searchable_repo(dir.path(), &["Casablanca", "Movie Night"]).await;

        let hits = repo
            .search("movie", &[], &unfiltered(), PageRequest { offset: 0, limit: 10 })
            .await
            .unwrap();

        assert_eq!(found(&hits), vec!["Movie Night"]);
    }

    #[test]
    fn a_full_text_query_is_scoped_to_the_title_column() {
        assert_eq!(
            fts_match("episode", "ep"),
            "kind:episode AND title:(ep*)",
            "kind is indexed now so that a branch only walks its own rows; drop the \
             scope and a query like ep matches the kind column of every episode, \
             putting the whole catalog back in front of the row filter"
        );
        assert_eq!(fts_match("movie", "the matrix"), "kind:movie AND title:(the* matrix*)");
    }

    #[tokio::test]
    async fn a_partial_word_matches_the_word_it_starts() {
        let dir = tempfile::tempdir().unwrap();
        let repo = searchable_repo(
            dir.path(),
            &["The Matrix", "Children of the Gods", "Rematrix", "Inception"],
        )
        .await;
        let page = PageRequest { offset: 0, limit: 10 };

        let partial = repo.search("mat", &[], &unfiltered(), page).await.unwrap();
        let plural = repo.search("god", &[], &unfiltered(), page).await.unwrap();
        let infix = repo.search("atrix", &[], &unfiltered(), page).await.unwrap();

        assert_eq!(found(&partial), vec!["The Matrix"]);
        assert_eq!(found(&plural), vec!["Children of the Gods"]);
        assert_eq!(partial.total, 1);
        assert_eq!(plural.total, 1);
        assert!(
            infix.items.is_empty(),
            "the row filter has to stay a prefix test; matching anywhere in a word \
             would pull in Rematrix, which the full text index cannot return either"
        );
    }

    #[tokio::test]
    async fn a_multi_word_query_still_has_to_start_the_title() {
        let dir = tempfile::tempdir().unwrap();
        let repo = searchable_repo(dir.path(), &["Watch The Matrix Now", "The Matrix"]).await;

        let hits = repo
            .search("the matrix", &[], &unfiltered(), PageRequest { offset: 0, limit: 10 })
            .await
            .unwrap();

        assert_eq!(
            found(&hits),
            vec!["The Matrix"],
            "a word prefix match never spans a space, so a two word query \
             matches the start of the title or nothing"
        );
    }

    #[tokio::test]
    async fn a_typed_percent_sign_is_not_a_wildcard() {
        let dir = tempfile::tempdir().unwrap();
        let repo = searchable_repo(dir.path(), &["Apple Pie", "A E I O U"]).await;

        let hits = repo
            .search("a%e", &[], &unfiltered(), PageRequest { offset: 0, limit: 10 })
            .await
            .unwrap();

        assert_eq!(
            found(&hits),
            vec!["A E I O U"],
            "titles are matched with LIKE now, so a percent reaching the pattern \
             would match Apple Pie; normalising the query to letters and digits \
             is the only thing keeping it inert"
        );
    }

    #[tokio::test]
    async fn episode_ids_group_by_parent_past_the_bind_limit() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        let parents = 2_000;
        for sql in [
            format!(
                "INSERT INTO series (id, title, sort_title, added_at, updated_at) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {parents}) \
                 SELECT 'sr-' || i, 'Show ' || i, 'show ' || i, i, i FROM c"
            ),
            format!(
                "INSERT INTO seasons (id, series_id, number, added_at, updated_at) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {parents}) \
                 SELECT 'se-' || i, 'sr-' || i, 1, i, i FROM c"
            ),
            format!(
                "INSERT INTO episodes (id, season_id, number, title, added_at, updated_at) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {parents}) \
                 SELECT 'e-' || i, 'se-' || i, 2, 'Ep ' || i, i, i FROM c"
            ),
            format!(
                "INSERT INTO episodes (id, season_id, number, title, added_at, updated_at) \
                 WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {parents}) \
                 SELECT 'first-' || i, 'se-' || i, 1, 'Ep ' || i, i, i FROM c"
            ),
        ] {
            sqlx::query(AssertSqlSafe(sql)).execute(&repo.pool).await.unwrap();
        }
        let series: Vec<SeriesId> = (1..parents).map(|i| SeriesId(format!("sr-{i}"))).collect();
        let seasons: Vec<SeasonId> = (1..parents).map(|i| SeasonId(format!("se-{i}"))).collect();

        let by_series = repo.episode_ids_for_series(&series).await.unwrap();
        let by_season = repo.episode_ids_for_seasons(&seasons).await.unwrap();

        assert_eq!(
            by_series.len(),
            series.len(),
            "a rollup batch binds one parameter per target, so it has to chunk"
        );
        assert_eq!(by_season.len(), seasons.len());
        assert_eq!(
            by_series[&SeriesId("sr-7".into())],
            vec![EpisodeId("first-7".into()), EpisodeId("e-7".into())],
            "episodes come back in season then episode order, so a caller that \
             cares about the first unwatched one can trust the order"
        );
        assert_eq!(by_season[&SeasonId("se-7".into())].len(), 2);
    }

    #[tokio::test]
    async fn reconciling_more_paths_than_sqlite_allows_parameters_marks_exactly_those_present() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        let rows = 40_000;
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO versions \
             (id, title_kind, title_id, library_id, quality, container, path, \
              size_bytes, duration_ms, available, added_at, updated_at) \
             WITH RECURSIVE c(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM c WHERE i < {rows}) \
             SELECT 'v-' || i, 'movie', 'm-' || i, 'lib-1', 'hd', 'mkv', '/media/' || i || '.mkv', \
             0, 0, 0, 0, 0 FROM c"
        )))
        .execute(&repo.pool)
        .await
        .unwrap();

        let present: Vec<String> = (1..rows).map(|i| format!("/media/{i}.mkv")).collect();
        repo.reconcile_library_versions(&LibraryId("lib-1".into()), &present).await.unwrap();

        let available: i64 = column(
            &sqlx::query("SELECT COUNT(*) AS n FROM versions WHERE available = 1")
                .fetch_one(&repo.pool)
                .await
                .unwrap(),
            "n",
        )
        .unwrap();
        assert_eq!(
            available as usize,
            present.len(),
            "a scan binds one parameter per file it walked and SQLite caps them at 32766, \
             so reconciling in one statement fails a large library outright"
        );
        let missing: i64 = column(
            &sqlx::query("SELECT available AS n FROM versions WHERE path = ?")
                .bind(format!("/media/{rows}.mkv"))
                .fetch_one(&repo.pool)
                .await
                .unwrap(),
            "n",
        )
        .unwrap();
        assert_eq!(
            missing, 0,
            "the one path the scan did not walk must stay unavailable, \
             so each chunk may only add to the present set and never reset it"
        );
    }

    #[tokio::test]
    async fn hot_read_paths_are_served_by_an_index() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        let pool = &repo.pool;

        for (sort, index) in [
            (TitleSort::Title, "idx_movies_sort_title"),
            (TitleSort::AddedAt, "idx_movies_added_at"),
            (TitleSort::Year, "idx_movies_year"),
        ] {
            let filter = viewer_filter(sort);
            let (where_sql, binds) = movie_filter_where(&filter);
            let refs: Vec<&str> = binds.iter().map(String::as_str).collect();

            let counted = plan(pool, &count_filtered_sql("movies m", &where_sql), &refs).await;
            assert!(
                counted.contains("USING INDEX idx_versions_title"),
                "the entitlement subquery must seek, not scan versions: {counted}"
            );

            let listed = plan(
                pool,
                &list_page_sql("movies m", "m", &where_sql, sort, filter.order),
                &[refs.as_slice(), &["50", "0"]].concat(),
            )
            .await;
            assert!(
                listed.contains(index) && !listed.contains("TEMP B-TREE"),
                "sorting by {sort:?} must walk {index}, not sort the whole table: {listed}"
            );
        }

        let filter = viewer_filter(TitleSort::Title);
        let mut gate_binds = Vec::new();
        let gate = episode_gate(&filter, &mut gate_binds);
        let mut recent_args: Vec<&str> = gate_binds.iter().map(String::as_str).collect();
        recent_args.push("2000");
        let windowed = plan(pool, &recent_episodes_sql(&gate), &recent_args).await;
        assert!(
            windowed.contains("idx_episodes_recent") && !windowed.contains("TEMP B-TREE"),
            "the home rail reads a window of the newest episodes, so ordering the whole \
             table would put the full scan straight back: {windowed}"
        );

        let mut next_args = vec!["s1", "1", "1", "1"];
        next_args.extend(gate_binds.iter().map(String::as_str));
        let successor = plan(pool, &next_in_series_sql(&gate), &next_args).await;
        assert!(
            successor.contains("idx_seasons_series") || successor.contains("idx_episodes_season"),
            "up next runs once per watched series, so it must seek: {successor}"
        );

        let (where_sql, binds) = series_filter_where(&filter);
        let refs: Vec<&str> = binds.iter().map(String::as_str).collect();
        let counted = plan(pool, &count_filtered_sql("series s", &where_sql), &refs).await;
        for index in ["idx_seasons_series", "idx_episodes_season", "idx_versions_title"] {
            assert!(
                counted.contains(index),
                "the series entitlement subquery must start at seasons and seek down; \
                 without {index} it drives off a range scan of every episode version: {counted}"
            );
        }

        for (sql, index, args) in [
            (LIST_SEASONS_SQL.to_string(), "idx_seasons_series", vec!["s1"]),
            (LIST_EPISODES_SQL.to_string(), "idx_episodes_season", vec!["se1"]),
            (LIST_VERSIONS_SQL.to_string(), "idx_versions_title", vec!["movie", "m1", "50", "0"]),
            (
                mark_present_sql(2),
                "idx_versions_library_path",
                vec!["lib-1", "/media/a.mkv", "/media/b.mkv"],
            ),
        ] {
            let text = plan(pool, &sql, &args).await;
            assert!(
                text.contains(&format!("USING INDEX {index}")),
                "{sql} must seek {index}: {text}"
            );
        }
    }

    #[test]
    fn enum_round_trips_and_rejects_unknown() {
        for quality in [Quality::Sd, Quality::Hd, Quality::Fhd, Quality::Uhd] {
            assert_eq!(quality_from_str(quality_to_str(quality)).unwrap(), quality);
        }
        for hdr in [HdrFormat::Hdr10, HdrFormat::Hdr10Plus, HdrFormat::DolbyVision, HdrFormat::Hlg]
        {
            assert_eq!(hdr_from_str(hdr_to_str(hdr)).unwrap(), hdr);
        }
        for format in [
            SubtitleFormat::Srt,
            SubtitleFormat::Ass,
            SubtitleFormat::Vtt,
            SubtitleFormat::Pgs,
            SubtitleFormat::VobSub,
        ] {
            assert_eq!(subtitle_format_from_str(subtitle_format_to_str(format)).unwrap(), format);
        }
        for source in [
            SubtitleSource::OpenSubtitles,
            SubtitleSource::External,
            SubtitleSource::Generated,
            SubtitleSource::MachineTranslated,
            SubtitleSource::Combined,
        ] {
            assert_eq!(subtitle_source_from_str(subtitle_source_to_str(source)).unwrap(), source);
        }
        assert!(quality_from_str("nope").is_err());
        assert!(hdr_from_str("nope").is_err());
        assert!(subtitle_format_from_str("nope").is_err());
        assert!(subtitle_source_from_str("nope").is_err());
    }

    // Every method here reaches the pool through `?`, and an untested `?` is an
    // error arm nothing proves propagates. Closing the pool is the cheapest way to
    // make all of them fail at once; a method missing from this list has an error
    // path no test has ever taken.
    #[tokio::test]
    async fn surfaces_backend_error_after_close() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        repo.pool.close().await;

        let page = PageRequest { offset: 0, limit: 10 };
        let movie_id = MovieId("m1".into());
        let series_id = SeriesId("s1".into());
        let season_id = SeasonId("se1".into());
        let episode_id = EpisodeId("e1".into());
        let version_id = VersionId("v1".into());
        let collection_id = CollectionId("c1".into());
        let person_id = PersonId("p1".into());
        let library_id = LibraryId("lib1".into());
        let title = TitleId::Movie(movie_id.clone());
        let filter = unfiltered();
        let ids = ["x".to_owned()];

        assert!(repo.list_movies(page).await.is_err());
        assert!(repo.get_movie(&movie_id).await.is_err());
        assert!(repo.movies_by_ids(std::slice::from_ref(&movie_id)).await.is_err());
        assert!(repo.list_series(page).await.is_err());
        assert!(repo.get_series(&series_id).await.is_err());
        assert!(repo.series_by_ids(std::slice::from_ref(&series_id)).await.is_err());
        assert!(repo.list_seasons(&series_id).await.is_err());
        assert!(repo.list_episodes(&season_id).await.is_err());
        assert!(repo.episode_ids_for_series(std::slice::from_ref(&series_id)).await.is_err());
        assert!(repo.episode_ids_for_seasons(std::slice::from_ref(&season_id)).await.is_err());
        assert!(repo.recent_episodes(&filter, 5).await.is_err());
        assert!(repo.visible_movies(std::slice::from_ref(&movie_id), &filter).await.is_err());
        assert!(repo.visible_episodes(std::slice::from_ref(&episode_id), &filter).await.is_err());
        assert!(repo.next_episode_in_series(&series_id, 1, 1, &filter).await.is_err());
        assert!(repo.get_episode(&episode_id).await.is_err());
        assert!(repo.list_collections(page).await.is_err());
        assert!(repo.get_collection(&collection_id).await.is_err());
        assert!(repo.upsert_collection(contracts::fixture::saga_collection()).await.is_err());
        assert!(repo.delete_collection(&collection_id).await.is_err());
        assert!(repo.collections_of_movie(&movie_id).await.is_err());
        assert!(repo.get_season(&season_id).await.is_err());
        assert!(repo.series_versions(&series_id).await.is_err());
        assert!(repo.list_versions(&title, page).await.is_err());
        assert!(repo.list_library_versions(&library_id, page).await.is_err());
        assert!(repo.list_all_versions(page).await.is_err());
        assert!(repo.version_detail(&version_id).await.is_err());
        assert!(repo.get_version(&version_id).await.is_err());
        assert!(repo.delete_version(&version_id).await.is_err());
        assert!(repo.delete_movie(&movie_id).await.is_err());
        assert!(repo.delete_series(&series_id).await.is_err());
        assert!(repo.delete_season(&season_id).await.is_err());
        assert!(repo.delete_episode(&episode_id).await.is_err());
        assert!(repo.upsert_movie(contracts::fixture::movie("m1")).await.is_err());
        assert!(repo.upsert_series(contracts::fixture::series("s1")).await.is_err());
        assert!(repo.upsert_season(contracts::fixture::season("se1", "s1")).await.is_err());
        assert!(repo.upsert_episode(contracts::fixture::episode("e1", "se1")).await.is_err());
        assert!(
            repo.upsert_version(contracts::fixture::version(
                "v1",
                title.clone(),
                "lib1",
                Quality::Hd
            ))
            .await
            .is_err()
        );
        assert!(repo.reconcile_library_versions(&library_id, &ids).await.is_err());
        let owner = ArtworkOwner::Movie(movie_id.clone());
        assert!(repo.set_artwork(&owner, &contracts::fixture::artwork_set("m1")).await.is_err());
        assert!(repo.list_artwork(&owner).await.is_err());
        assert!(repo.all_artwork_ids().await.is_err());
        assert!(repo.all_version_ids().await.is_err());
        assert!(repo.live_artwork_ids(&ids).await.is_err());
        assert!(repo.live_version_ids(&ids).await.is_err());
        assert!(repo.live_artwork_paths(&ids).await.is_err());
        assert!(repo.live_subtitle_paths(&ids).await.is_err());
        assert!(repo.live_trickplay_paths(&ids).await.is_err());
        assert!(repo.set_version_tracks(&version_id, &[], &[], &[], &[]).await.is_err());
        assert!(repo.set_trickplay(&version_id, &[]).await.is_err());
        assert!(repo.set_subtitle_files(&version_id, &[]).await.is_err());
        let subtitle = SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: version_id.clone(),
            language: None,
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/subs/sf1.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        };
        assert!(repo.add_subtitle_file(&version_id, &subtitle).await.is_err());
        assert!(repo.upsert_person(contracts::fixture::people()[0].clone()).await.is_err());
        assert!(repo.get_person(&person_id).await.is_err());
        assert!(
            repo.set_title_enrichment(
                &TitleRef::Movie(movie_id.clone()),
                &contracts::fixture::movie_enrichment()
            )
            .await
            .is_err()
        );
        assert!(repo.movie_detail(&movie_id).await.is_err());
        assert!(repo.series_detail(&series_id).await.is_err());
        assert!(repo.filmography(&person_id).await.is_err());
        assert!(repo.list_genres(None).await.is_err());
        assert!(repo.list_movies_filtered(&filter, page).await.is_err());
        assert!(repo.list_series_filtered(&filter, page).await.is_err());
        assert!(repo.random_playable_title(&RandomScope::Movies, &filter).await.is_err());
        assert!(repo.search("q", &[], &filter, page).await.is_err());
    }

    #[tokio::test]
    async fn rejects_corrupt_title_kind() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        sqlx::query(
            "INSERT INTO versions \
             (id, title_kind, title_id, library_id, quality, container, path, size_bytes, \
              duration_ms, added_at, updated_at) \
             VALUES ('vx', 'bogus', 'm1', 'lib1', 'sd', 'mkv', '/x.mkv', 1, 1, 0, 0)",
        )
        .execute(&repo.pool)
        .await
        .unwrap();
        assert!(repo.version_detail(&VersionId("vx".into())).await.is_err());
    }

    #[tokio::test]
    async fn rejects_corrupt_credit_role() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
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
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
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
        let repo = SqliteCatalogRepo::connect(&dir.path().join("catalog.db")).await.unwrap();
        repo.insert_movie(Movie {
            id: MovieId("m1".into()),
            title: "Alpha".into(),
            sort_title: "alpha".into(),
            year: None,
            overview: None,
            runtime_minutes: None,
            content_rating: None,
            manually_edited: false,
            added_at: from_millis(0).unwrap(),
            updated_at: from_millis(0).unwrap(),
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
