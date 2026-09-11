use domain::library::{LibraryId, LibraryKind, ParsedMedia};
use domain::metadata::ExternalId;
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;
use crate::library::parse_filename;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchJobPayload {
    pub library: LibraryId,
    pub source_url: String,
    pub kind: LibraryKind,
    pub title: String,
    pub year: Option<u16>,
    pub external_id: Option<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
}

impl FetchJobPayload {
    pub fn encode(&self) -> String {
        encode_payload(&Wire::from(self))
    }

    pub fn decode(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<Wire>(raw).map(Self::from)
    }

    pub fn resolved_external_id(&self) -> Option<ExternalId> {
        let value = self.external_id.as_deref()?.trim();
        if value.is_empty() {
            return None;
        }
        if value.starts_with("tt") {
            return Some(ExternalId { source: "imdb".to_owned(), value: value.to_owned() });
        }
        let endpoint = match self.kind {
            LibraryKind::Tv => "tv",
            LibraryKind::Movie => "movie",
        };
        let value =
            if value.contains('/') { value.to_owned() } else { format!("{endpoint}/{value}") };
        Some(ExternalId { source: "tmdb".to_owned(), value })
    }

    pub fn filename_stem(&self) -> String {
        let title = safe_segment(&self.title);
        let tag = self
            .resolved_external_id()
            .as_ref()
            .and_then(filename_tag)
            .map(|tag| format!(" [{tag}]"))
            .unwrap_or_default();
        match self.kind {
            LibraryKind::Tv => {
                let s = self.season.unwrap_or(1);
                let e = self.episode.unwrap_or(1);
                format!("{title} - S{s:02}E{e:02}{tag}")
            }
            LibraryKind::Movie => match self.year {
                Some(year) => format!("{title} ({year}){tag}"),
                None => format!("{title}{tag}"),
            },
        }
    }

    pub fn parsed(&self, path: &str) -> ParsedMedia {
        let episodic = matches!(self.kind, LibraryKind::Tv);
        ParsedMedia {
            title: self.title.trim().to_owned(),
            year: self.year,
            season: episodic.then(|| self.season.unwrap_or(1)),
            episode: episodic.then(|| self.episode.unwrap_or(1)),
            quality: parse_filename(path).quality,
            external_id: self.resolved_external_id(),
        }
    }

    pub fn destination_dir(&self, root: &str, fallback_name: &str) -> String {
        let title = match safe_segment(&self.title) {
            name if name.is_empty() => safe_segment(fallback_name),
            name => name,
        };
        match self.kind {
            LibraryKind::Tv => {
                let s = self.season.unwrap_or(1);
                format!("{root}/{title}/Season {s:02}")
            }
            LibraryKind::Movie => match self.year {
                Some(year) => format!("{root}/{title} ({year})"),
                None => format!("{root}/{title}"),
            },
        }
    }
}

pub fn filename_tag(id: &ExternalId) -> Option<String> {
    let value = id.value.rsplit('/').next().unwrap_or(&id.value);
    match id.source.as_str() {
        "tmdb" => Some(format!("tmdbid-{value}")),
        "imdb" => Some(format!("imdbid-{value}")),
        _ => None,
    }
}

fn safe_segment(name: &str) -> String {
    sanitize_filename::sanitize_with_options(
        name,
        sanitize_filename::Options { windows: true, truncate: true, replacement: "" },
    )
    .trim()
    .to_owned()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum KindWire {
    Movie,
    Tv,
}

impl From<LibraryKind> for KindWire {
    fn from(kind: LibraryKind) -> Self {
        match kind {
            LibraryKind::Movie => KindWire::Movie,
            LibraryKind::Tv => KindWire::Tv,
        }
    }
}

impl From<KindWire> for LibraryKind {
    fn from(kind: KindWire) -> Self {
        match kind {
            KindWire::Movie => LibraryKind::Movie,
            KindWire::Tv => LibraryKind::Tv,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    library: String,
    source_url: String,
    kind: KindWire,
    title: String,
    #[serde(default)]
    year: Option<u16>,
    external_id: Option<String>,
    season: Option<u16>,
    episode: Option<u16>,
}

impl From<&FetchJobPayload> for Wire {
    fn from(payload: &FetchJobPayload) -> Self {
        Wire {
            library: payload.library.0.clone(),
            source_url: payload.source_url.clone(),
            kind: payload.kind.into(),
            title: payload.title.clone(),
            year: payload.year,
            external_id: payload.external_id.clone(),
            season: payload.season,
            episode: payload.episode,
        }
    }
}

impl From<Wire> for FetchJobPayload {
    fn from(wire: Wire) -> Self {
        FetchJobPayload {
            library: LibraryId(wire.library),
            source_url: wire.source_url,
            kind: wire.kind.into(),
            title: wire.title,
            year: wire.year,
            external_id: wire.external_id,
            season: wire.season,
            episode: wire.episode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::confidence;

    fn round_trip(payload: FetchJobPayload) {
        let encoded = payload.encode();
        assert_eq!(FetchJobPayload::decode(&encoded).unwrap(), payload);
    }

    fn movie(title: &str, year: Option<u16>, external_id: Option<&str>) -> FetchJobPayload {
        FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKind::Movie,
            title: title.into(),
            year,
            external_id: external_id.map(str::to_owned),
            season: None,
            episode: None,
        }
    }

    fn series(
        title: &str,
        season: Option<u16>,
        episode: Option<u16>,
        external_id: Option<&str>,
    ) -> FetchJobPayload {
        FetchJobPayload {
            kind: LibraryKind::Tv,
            title: title.into(),
            season,
            episode,
            external_id: external_id.map(str::to_owned),
            ..movie(title, None, None)
        }
    }

    #[test]
    fn round_trips_movie_and_tv() {
        round_trip(movie("The Matrix", Some(1999), Some("tt0133093")));
        round_trip(series("Great Show", Some(1), Some(2), None));
    }

    #[test]
    fn a_payload_written_before_the_year_existed_still_decodes() {
        let legacy = r#"{"library":"lib1","source_url":"https://example.com/a","kind":"movie","title":"The Matrix","external_id":null,"season":null,"episode":null}"#;
        assert_eq!(FetchJobPayload::decode(legacy).unwrap().year, None);
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(FetchJobPayload::decode("not json").is_err());
    }

    #[test]
    fn a_typed_title_is_kept_whole_however_it_reads_as_a_release_name() {
        // parse_filename cuts a scanned name at the first junk token, so "4K"
        // inside a real title truncated this to "THIS IS". A fetch already
        // knows the title, so it must not be re-derived from the filename.
        let payload = movie("THIS IS 4K ANIME YOUR NAME 2160P 60FPS", None, None);
        let parsed = payload.parsed("/ext/whatever.mkv");
        assert_eq!(parsed.title, "THIS IS 4K ANIME YOUR NAME 2160P 60FPS");
        assert_eq!(parse_filename("/ext/THIS IS 4K ANIME.mkv").title, "THIS IS");
    }

    #[test]
    fn a_title_that_ends_in_a_bare_year_keeps_it() {
        let parsed = movie("Blade Runner 2049", None, None).parsed("/ext/x.mkv");
        assert_eq!(parsed.title, "Blade Runner 2049");
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn a_movie_carries_its_year_and_no_episode_numbering() {
        let parsed = movie("The Matrix", Some(1999), Some("tt0133093")).parsed("/ext/x.mkv");
        assert_eq!(parsed.year, Some(1999));
        assert_eq!(parsed.season, None);
        assert_eq!(parsed.episode, None);
        assert_eq!(parsed.external_id.unwrap().source, "imdb");
    }

    #[test]
    fn an_episode_defaults_to_the_same_numbering_the_filename_uses() {
        // filename_stem falls back to S01E01, so parsed has to agree or the
        // catalog entry names a different episode than the file on disk.
        let payload = series("Great Show", None, None, None);
        let parsed = payload.parsed("/ext/x.mkv");
        assert_eq!((parsed.season, parsed.episode), (Some(1), Some(1)));
        assert!(payload.filename_stem().contains("S01E01"));

        let numbered = series("Great Show", Some(2), Some(5), None).parsed("/ext/x.mkv");
        assert_eq!((numbered.season, numbered.episode), (Some(2), Some(5)));
    }

    #[test]
    fn quality_still_comes_from_the_file_since_the_payload_cannot_know_it() {
        let payload = movie("Anything", None, None);
        assert_eq!(payload.parsed("/ext/x.mkv").quality, None);
        assert_eq!(
            payload.parsed("/ext/x 2160p.mkv").quality,
            parse_filename("/ext/x 2160p.mkv").quality
        );
    }

    #[test]
    fn resolved_external_id_detects_imdb_tmdb_and_blank() {
        let mut payload = movie("The Matrix", None, Some("tt0133093"));
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId { source: "imdb".into(), value: "tt0133093".into() })
        );
        payload.external_id = Some("603".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "movie/603".into() })
        );
        payload.kind = LibraryKind::Tv;
        payload.external_id = Some("1399".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
        payload.external_id = Some("tv/1399".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
        payload.external_id = Some("  ".into());
        assert_eq!(payload.resolved_external_id(), None);
        payload.external_id = None;
        assert_eq!(payload.resolved_external_id(), None);
    }

    #[test]
    fn movie_stem_round_trips_through_parser() {
        let payload = movie("The Matrix", Some(1999), None);
        assert_eq!(payload.filename_stem(), "The Matrix (1999)");
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, Some(1999));
        assert!(!parsed.is_episodic());
    }

    #[test]
    fn movie_without_year_stem_round_trips_through_parser() {
        let payload = movie("Some Movie", None, None);
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "Some Movie");
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn tv_stem_round_trips_through_parser() {
        let payload = series("Great Show", Some(1), Some(2), None);
        assert_eq!(payload.filename_stem(), "Great Show - S01E02");
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "Great Show");
        assert_eq!(parsed.season, Some(1));
        assert_eq!(parsed.episode, Some(2));
    }

    #[test]
    fn a_tagged_movie_survives_a_wipe_without_its_year() {
        let payload = movie("The Matrix", None, Some("603"));
        assert_eq!(payload.filename_stem(), "The Matrix [tmdbid-603]");
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, None);
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "movie/603".into() })
        );
        assert_eq!(confidence(&parsed), 0.9);
    }

    #[test]
    fn an_imdb_id_is_tagged_and_read_back_as_imdb() {
        let payload = movie("The Matrix", Some(1999), Some("tt0133093"));
        assert_eq!(payload.filename_stem(), "The Matrix (1999) [imdbid-tt0133093]");
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, Some(1999));
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "imdb".into(), value: "tt0133093".into() })
        );
    }

    #[test]
    fn a_tagged_episode_reads_back_as_a_tv_id() {
        let payload = series("Great Show", Some(1), Some(2), Some("1399"));
        assert_eq!(payload.filename_stem(), "Great Show - S01E02 [tmdbid-1399]");
        let parsed = parse_filename(&format!("{}.mkv", payload.filename_stem()));
        assert_eq!(parsed.title, "Great Show");
        assert_eq!(
            parsed.external_id,
            Some(ExternalId { source: "tmdb".into(), value: "tv/1399".into() })
        );
    }

    #[test]
    fn a_provider_we_do_not_tag_for_leaves_the_name_alone() {
        assert_eq!(filename_tag(&ExternalId { source: "tvdb".into(), value: "603".into() }), None);
    }

    #[test]
    fn a_movie_lands_in_a_title_directory() {
        assert_eq!(
            movie("The Matrix", Some(1999), None).destination_dir("/ext", "job-1"),
            "/ext/The Matrix (1999)"
        );
        assert_eq!(
            movie("Some Movie", None, None).destination_dir("/ext", "job-1"),
            "/ext/Some Movie"
        );
    }

    #[test]
    fn an_episode_lands_under_a_padded_season_directory() {
        assert_eq!(
            series("Great Show", Some(1), Some(2), None).destination_dir("/ext", "job-1"),
            "/ext/Great Show/Season 01"
        );
        assert_eq!(
            series("Great Show", None, None, None).destination_dir("/ext", "job-1"),
            "/ext/Great Show/Season 01"
        );
        assert_eq!(
            series("Great Show", Some(12), Some(3), None).destination_dir("/ext", "job-1"),
            "/ext/Great Show/Season 12"
        );
    }

    #[test]
    fn a_title_cannot_escape_the_library_root() {
        assert_eq!(movie("../../etc", None, None).destination_dir("/ext", "job-1"), "/ext/....etc");
        assert_eq!(
            movie("Face/Off", Some(1997), None).destination_dir("/ext", "job-1"),
            "/ext/FaceOff (1997)"
        );
        assert_eq!(movie("Alien: Resurrection", None, None).filename_stem(), "Alien Resurrection");
    }

    #[test]
    fn a_title_that_sanitises_to_nothing_falls_back_to_the_job_id() {
        assert_eq!(movie("///", None, None).destination_dir("/ext", "job-1"), "/ext/job-1");
        assert_eq!(
            series("///", Some(2), Some(1), None).destination_dir("/ext", "job-1"),
            "/ext/job-1/Season 02"
        );
    }
}
