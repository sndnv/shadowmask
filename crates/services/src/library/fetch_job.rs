use domain::library::{LibraryId, LibraryKind};
use domain::metadata::ExternalId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchJobPayload {
    pub library: LibraryId,
    pub source_url: String,
    pub kind: LibraryKind,
    pub title: String,
    pub external_id: Option<String>,
    pub season: Option<u16>,
    pub episode: Option<u16>,
}

impl FetchJobPayload {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&Wire::from(self))
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
            return Some(ExternalId {
                source: "imdb".to_owned(),
                value: value.to_owned(),
            });
        }
        let endpoint = match self.kind {
            LibraryKind::Tv => "tv",
            LibraryKind::Movie => "movie",
        };
        let value = if value.contains('/') {
            value.to_owned()
        } else {
            format!("{endpoint}/{value}")
        };
        Some(ExternalId {
            source: "tmdb".to_owned(),
            value,
        })
    }
}

pub fn fetch_filename_stem(
    kind: LibraryKind,
    title: &str,
    year: Option<u16>,
    season: Option<u16>,
    episode: Option<u16>,
) -> String {
    match kind {
        LibraryKind::Tv => {
            let s = season.unwrap_or(1);
            let e = episode.unwrap_or(1);
            format!("{title} - S{s:02}E{e:02}")
        }
        LibraryKind::Movie => match year {
            Some(year) => format!("{title} ({year})"),
            None => title.to_owned(),
        },
    }
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
            external_id: wire.external_id,
            season: wire.season,
            episode: wire.episode,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::parse_filename;

    fn round_trip(payload: FetchJobPayload) {
        let encoded = payload.encode().unwrap();
        assert_eq!(FetchJobPayload::decode(&encoded).unwrap(), payload);
    }

    #[test]
    fn round_trips_movie_and_tv() {
        round_trip(FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKind::Movie,
            title: "The Matrix".into(),
            external_id: Some("tt0133093".into()),
            season: None,
            episode: None,
        });
        round_trip(FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=xyz".into(),
            kind: LibraryKind::Tv,
            title: "Great Show".into(),
            external_id: None,
            season: Some(1),
            episode: Some(2),
        });
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(FetchJobPayload::decode("not json").is_err());
    }

    #[test]
    fn resolved_external_id_detects_imdb_tmdb_and_blank() {
        let mut payload = FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=abc".into(),
            kind: LibraryKind::Movie,
            title: "The Matrix".into(),
            external_id: Some("tt0133093".into()),
            season: None,
            episode: None,
        };
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId {
                source: "imdb".into(),
                value: "tt0133093".into(),
            })
        );
        payload.external_id = Some("603".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId {
                source: "tmdb".into(),
                value: "movie/603".into(),
            })
        );
        payload.kind = LibraryKind::Tv;
        payload.external_id = Some("1399".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId {
                source: "tmdb".into(),
                value: "tv/1399".into(),
            })
        );
        payload.external_id = Some("tv/1399".into());
        assert_eq!(
            payload.resolved_external_id(),
            Some(domain::metadata::ExternalId {
                source: "tmdb".into(),
                value: "tv/1399".into(),
            })
        );
        payload.external_id = Some("  ".into());
        assert_eq!(payload.resolved_external_id(), None);
        payload.external_id = None;
        assert_eq!(payload.resolved_external_id(), None);
    }

    #[test]
    fn movie_stem_round_trips_through_parser() {
        let stem = fetch_filename_stem(LibraryKind::Movie, "The Matrix", Some(1999), None, None);
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "The Matrix");
        assert_eq!(parsed.year, Some(1999));
        assert!(!parsed.is_episodic());
    }

    #[test]
    fn movie_without_year_stem_round_trips_through_parser() {
        let stem = fetch_filename_stem(LibraryKind::Movie, "Some Movie", None, None, None);
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "Some Movie");
        assert_eq!(parsed.year, None);
    }

    #[test]
    fn tv_stem_round_trips_through_parser() {
        let stem = fetch_filename_stem(LibraryKind::Tv, "Great Show", None, Some(1), Some(2));
        let parsed = parse_filename(&format!("{stem}.mkv"));
        assert_eq!(parsed.title, "Great Show");
        assert_eq!(parsed.season, Some(1));
        assert_eq!(parsed.episode, Some(2));
    }
}
