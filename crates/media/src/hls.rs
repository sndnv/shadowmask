use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use domain::error::StreamError;
use domain::session::{DeliveryMode, SessionId, StreamClaims, StreamSource};

pub(crate) const VARIANT: &str = "v0";
const MEDIA_PLAYLIST: &str = "index.m3u8";
const SUBTITLE_GROUP: &str = "subs";
const SUBTITLE_VARIANT: &str = "subs";

pub struct SubtitleRendition {
    pub name: String,
    pub language: String,
}

pub struct StreamEntry {
    pub mode: DeliveryMode,
    pub output_dir: PathBuf,
    pub direct_path: Option<PathBuf>,
    pub bandwidth: u64,
    pub subtitle: Option<SubtitleRendition>,
}

#[derive(Default)]
pub struct HlsStreamSource {
    sessions: RwLock<HashMap<SessionId, StreamEntry>>,
}

impl HlsStreamSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, session: SessionId, entry: StreamEntry) {
        self.sessions.write().unwrap().insert(session, entry);
    }

    pub fn remove(&self, session: &SessionId) {
        self.sessions.write().unwrap().remove(session);
    }
}

impl StreamSource for HlsStreamSource {
    fn master_playlist(&self, claims: &StreamClaims) -> Result<String, StreamError> {
        let sessions = self.sessions.read().unwrap();
        let entry = sessions.get(&claims.session).ok_or(StreamError::NotLive)?;
        if entry.mode == DeliveryMode::Direct {
            return Err(StreamError::Invalid);
        }
        let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:3\n");
        let mut stream_inf = format!("#EXT-X-STREAM-INF:BANDWIDTH={}", entry.bandwidth);
        if let Some(sub) = &entry.subtitle {
            out.push_str(&format!(
                "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"{SUBTITLE_GROUP}\",NAME=\"{}\",LANGUAGE=\"{}\",AUTOSELECT=YES,DEFAULT=YES,URI=\"{SUBTITLE_VARIANT}/{MEDIA_PLAYLIST}\"\n",
                sub.name, sub.language,
            ));
            stream_inf.push_str(&format!(",SUBTITLES=\"{SUBTITLE_GROUP}\""));
        }
        out.push_str(&stream_inf);
        out.push('\n');
        out.push_str(&format!("{VARIANT}/{MEDIA_PLAYLIST}\n"));
        Ok(out)
    }

    fn media_path(
        &self,
        claims: &StreamClaims,
        variant: &str,
        file: &str,
    ) -> Result<PathBuf, StreamError> {
        let sessions = self.sessions.read().unwrap();
        let entry = sessions.get(&claims.session).ok_or(StreamError::NotLive)?;
        if !is_safe_segment(variant) || !is_safe_segment(file) {
            return Err(StreamError::Invalid);
        }
        Ok(entry.output_dir.join(variant).join(file))
    }

    fn direct_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError> {
        let sessions = self.sessions.read().unwrap();
        let entry = sessions.get(&claims.session).ok_or(StreamError::NotLive)?;
        entry.direct_path.clone().ok_or(StreamError::Invalid)
    }
}

fn is_safe_segment(segment: &str) -> bool {
    !segment.is_empty() && segment != ".." && !segment.contains('/') && !segment.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;

    use domain::catalog::VersionId;
    use domain::user::UserId;

    fn claims(session: &str) -> StreamClaims {
        StreamClaims {
            session: SessionId(session.to_owned()),
            user: UserId("u1".to_owned()),
            version: VersionId("ver-1".to_owned()),
            expires_at: Timestamp::now(),
        }
    }

    fn transcode_entry() -> StreamEntry {
        StreamEntry {
            mode: DeliveryMode::Transcode,
            output_dir: PathBuf::from("/cache/s1"),
            direct_path: None,
            bandwidth: 4_000_000,
            subtitle: None,
        }
    }

    #[test]
    fn master_playlist_lists_single_variant() {
        let source = HlsStreamSource::new();
        source.register(SessionId("s1".to_owned()), transcode_entry());
        let master = source.master_playlist(&claims("s1")).unwrap();
        assert!(master.starts_with("#EXTM3U"));
        assert!(master.contains("#EXT-X-STREAM-INF:BANDWIDTH=4000000"));
        assert!(master.contains("v0/index.m3u8"));
        assert!(!master.contains("SUBTITLES"));
    }

    #[test]
    fn master_playlist_includes_subtitle_rendition() {
        let source = HlsStreamSource::default();
        source.register(
            SessionId("s1".to_owned()),
            StreamEntry {
                subtitle: Some(SubtitleRendition {
                    name: "English".to_owned(),
                    language: "en".to_owned(),
                }),
                ..transcode_entry()
            },
        );
        let master = source.master_playlist(&claims("s1")).unwrap();
        assert!(master.contains(
            "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"English\",LANGUAGE=\"en\""
        ));
        assert!(master.contains("URI=\"subs/index.m3u8\""));
        assert!(master.contains("#EXT-X-STREAM-INF:BANDWIDTH=4000000,SUBTITLES=\"subs\""));
    }

    #[test]
    fn master_playlist_direct_is_invalid() {
        let source = HlsStreamSource::new();
        source.register(
            SessionId("s1".to_owned()),
            StreamEntry {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/media/movie.mkv")),
                ..transcode_entry()
            },
        );
        assert!(matches!(
            source.master_playlist(&claims("s1")),
            Err(StreamError::Invalid)
        ));
    }

    #[test]
    fn master_playlist_unknown_session_not_live() {
        let source = HlsStreamSource::new();
        assert!(matches!(
            source.master_playlist(&claims("nope")),
            Err(StreamError::NotLive)
        ));
    }

    #[test]
    fn media_path_maps_variant_and_file() {
        let source = HlsStreamSource::new();
        source.register(SessionId("s1".to_owned()), transcode_entry());
        let path = source
            .media_path(&claims("s1"), "v0", "seg_00001.ts")
            .unwrap();
        assert_eq!(path, PathBuf::from("/cache/s1/v0/seg_00001.ts"));
    }

    #[test]
    fn media_path_rejects_unsafe_segments() {
        let source = HlsStreamSource::new();
        source.register(SessionId("s1".to_owned()), transcode_entry());
        for (variant, file) in [("..", "index.m3u8"), ("v0", ""), ("v0", "a\\b")] {
            assert!(matches!(
                source.media_path(&claims("s1"), variant, file),
                Err(StreamError::Invalid)
            ));
        }
    }

    #[test]
    fn media_path_unknown_session_not_live() {
        let source = HlsStreamSource::new();
        assert!(matches!(
            source.media_path(&claims("nope"), "v0", "index.m3u8"),
            Err(StreamError::NotLive)
        ));
    }

    #[test]
    fn direct_file_returns_path() {
        let source = HlsStreamSource::new();
        source.register(
            SessionId("s1".to_owned()),
            StreamEntry {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/media/movie.mkv")),
                ..transcode_entry()
            },
        );
        assert_eq!(
            source.direct_file(&claims("s1")).unwrap(),
            PathBuf::from("/media/movie.mkv")
        );
    }

    #[test]
    fn direct_file_without_path_is_invalid() {
        let source = HlsStreamSource::new();
        source.register(SessionId("s1".to_owned()), transcode_entry());
        assert!(matches!(
            source.direct_file(&claims("s1")),
            Err(StreamError::Invalid)
        ));
    }

    #[test]
    fn direct_file_unknown_session_not_live() {
        let source = HlsStreamSource::new();
        assert!(matches!(
            source.direct_file(&claims("nope")),
            Err(StreamError::NotLive)
        ));
    }

    #[test]
    fn remove_deregisters_session() {
        let source = HlsStreamSource::new();
        source.register(SessionId("s1".to_owned()), transcode_entry());
        source.remove(&SessionId("s1".to_owned()));
        assert!(matches!(
            source.master_playlist(&claims("s1")),
            Err(StreamError::NotLive)
        ));
    }
}
