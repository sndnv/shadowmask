use domain::library::{LibraryId, LibraryKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchJobPayload {
    pub library: LibraryId,
    pub source_url: String,
    pub kind: LibraryKind,
    pub title: String,
    pub imdb_id: Option<String>,
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
    imdb_id: Option<String>,
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
            imdb_id: payload.imdb_id.clone(),
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
            imdb_id: wire.imdb_id,
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
            imdb_id: Some("tt0133093".into()),
            season: None,
            episode: None,
        });
        round_trip(FetchJobPayload {
            library: LibraryId("lib1".into()),
            source_url: "https://example.com/watch?v=xyz".into(),
            kind: LibraryKind::Tv,
            title: "Great Show".into(),
            imdb_id: None,
            season: Some(1),
            episode: Some(2),
        });
    }

    #[test]
    fn decode_rejects_malformed_json() {
        assert!(FetchJobPayload::decode("not json").is_err());
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
