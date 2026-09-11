use jiff::Timestamp;

use crate::catalog::TitleId;
use crate::common::Quality;
use crate::library::LibraryId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionId(pub String);

#[derive(Debug, Clone)]
pub struct Version {
    pub id: VersionId,
    pub title: TitleId,
    pub library: LibraryId,
    pub quality: Quality,
    pub container: String,
    pub path: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub available: bool,
    pub added_at: Timestamp,
    pub updated_at: Timestamp,
}

pub fn best_available(versions: &[Version]) -> Option<&Version> {
    versions.iter().filter(|version| version.available).max_by(|a, b| {
        a.quality.cmp(&b.quality).then(a.size_bytes.cmp(&b.size_bytes)).then(b.id.0.cmp(&a.id.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, quality: Quality, size: u64, available: bool) -> Version {
        Version {
            id: VersionId(id.to_owned()),
            title: TitleId::Movie(crate::catalog::MovieId("m1".to_owned())),
            library: LibraryId("lib".to_owned()),
            quality,
            container: "mkv".to_owned(),
            path: format!("/media/{id}.mkv"),
            size_bytes: size,
            duration_ms: 1,
            available,
            added_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn nothing_available_means_no_pick() {
        assert!(best_available(&[]).is_none());
        let offline = [version("v1", Quality::Uhd, 9, false)];
        assert!(best_available(&offline).is_none());
    }

    #[test]
    fn quality_outranks_size() {
        let versions =
            [version("v1", Quality::Hd, 900, true), version("v2", Quality::Uhd, 100, true)];
        assert_eq!(best_available(&versions).unwrap().id.0, "v2");
    }

    #[test]
    fn an_unavailable_best_is_skipped_for_the_next_one() {
        let versions =
            [version("v1", Quality::Uhd, 900, false), version("v2", Quality::Hd, 100, true)];
        assert_eq!(best_available(&versions).unwrap().id.0, "v2");
    }

    #[test]
    fn size_breaks_a_quality_tie() {
        let versions =
            [version("v1", Quality::Hd, 100, true), version("v2", Quality::Hd, 900, true)];
        assert_eq!(best_available(&versions).unwrap().id.0, "v2");
    }

    #[test]
    fn the_lowest_id_breaks_a_full_tie() {
        let versions =
            [version("v2", Quality::Hd, 100, true), version("v1", Quality::Hd, 100, true)];
        assert_eq!(best_available(&versions).unwrap().id.0, "v1");
    }
}
