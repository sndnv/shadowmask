use std::collections::BTreeMap;

use domain::catalog::{EpisodeId, MovieId, TitleId};
use domain::library::{DiscoveredFile, DuplicateCandidate, DuplicateCandidateId, MatchedGroup};

pub fn find_duplicates(groups: &[MatchedGroup]) -> Vec<DuplicateCandidate> {
    let mut candidates = Vec::new();
    for group in groups {
        let title = synthetic_title_id(group);
        let mut buckets: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for file in &group.files {
            buckets
                .entry(resolution(file))
                .or_default()
                .push(file.path.clone());
        }
        for (res, mut paths) in buckets {
            if paths.len() < 2 {
                continue;
            }
            paths.sort();
            candidates.push(DuplicateCandidate {
                id: DuplicateCandidateId(format!("dup:{}:{res}", title.id())),
                title: title.clone(),
                paths,
            });
        }
    }
    candidates.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    candidates
}

fn resolution(file: &DiscoveredFile) -> u32 {
    file.probe.video.iter().map(|v| v.height).max().unwrap_or(0)
}

fn synthetic_title_id(group: &MatchedGroup) -> TitleId {
    let key = &group.key;
    let year = key.year.map(|y| format!(":{y}")).unwrap_or_default();
    let episode = match (key.season, key.episode) {
        (Some(s), Some(e)) => format!(":s{s:02}e{e:02}"),
        _ => String::new(),
    };
    let id = format!("scan:{}{year}{episode}", key.title_slug);
    if group.parsed.is_episodic() {
        TitleId::Episode(EpisodeId(id))
    } else {
        TitleId::Movie(MovieId(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::library::{LibraryId, MatchKey, ParsedMedia};
    use domain::media::{ProbeResult, VideoTrack};

    fn video_track(height: u32) -> VideoTrack {
        VideoTrack {
            index: 0,
            codec: "h264".to_owned(),
            width: height * 16 / 9,
            height,
            bit_depth: 8,
            hdr: None,
            frame_rate: 24.0,
            bitrate: None,
        }
    }

    fn file(path: &str, height: Option<u32>) -> DiscoveredFile {
        DiscoveredFile {
            library: LibraryId("lib".to_owned()),
            path: path.to_owned(),
            size_bytes: 1,
            probe: ProbeResult {
                duration_ms: 0,
                video: height.map(|h| vec![video_track(h)]).unwrap_or_default(),
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        }
    }

    fn group(
        slug: &str,
        year: Option<u16>,
        season: Option<u16>,
        episode: Option<u16>,
        files: Vec<DiscoveredFile>,
    ) -> MatchedGroup {
        MatchedGroup {
            key: MatchKey {
                title_slug: slug.to_owned(),
                year,
                season,
                episode,
            },
            parsed: ParsedMedia {
                title: slug.to_owned(),
                year,
                season,
                episode,
                quality: None,
            },
            confidence: 0.9,
            files,
        }
    }

    #[test]
    fn two_files_same_resolution_are_a_duplicate() {
        let groups = [group(
            "the-matrix",
            Some(1999),
            None,
            None,
            vec![
                file("/m/matrix-b.mkv", Some(1080)),
                file("/m/matrix-a.mkv", Some(1080)),
            ],
        )];
        let dupes = find_duplicates(&groups);
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].id.0, "dup:scan:the-matrix:1999:1080");
        assert_eq!(
            dupes[0].title,
            TitleId::Movie(MovieId("scan:the-matrix:1999".to_owned()))
        );
        assert_eq!(dupes[0].paths, vec!["/m/matrix-a.mkv", "/m/matrix-b.mkv"]);
    }

    #[test]
    fn different_resolutions_are_legitimate_versions() {
        let groups = [group(
            "the-matrix",
            Some(1999),
            None,
            None,
            vec![
                file("/m/matrix.1080p.mkv", Some(1080)),
                file("/m/matrix.2160p.mkv", Some(2160)),
            ],
        )];
        assert!(find_duplicates(&groups).is_empty());
    }

    #[test]
    fn single_file_is_not_a_duplicate() {
        let groups = [group(
            "solo",
            Some(2020),
            None,
            None,
            vec![file("/m/solo.mkv", Some(1080))],
        )];
        assert!(find_duplicates(&groups).is_empty());
    }

    #[test]
    fn episodic_group_yields_episode_title() {
        let groups = [group(
            "show",
            None,
            Some(1),
            Some(2),
            vec![
                file("/tv/show.s01e02.mkv", Some(720)),
                file("/tv/show.s01e02.repack.mkv", Some(720)),
            ],
        )];
        let dupes = find_duplicates(&groups);
        assert_eq!(dupes.len(), 1);
        assert_eq!(
            dupes[0].title,
            TitleId::Episode(EpisodeId("scan:show:s01e02".to_owned()))
        );
    }

    #[test]
    fn files_without_video_bucket_together() {
        let groups = [group(
            "audio-only",
            None,
            None,
            None,
            vec![file("/m/a.mkv", None), file("/m/b.mkv", None)],
        )];
        let dupes = find_duplicates(&groups);
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0].id.0, "dup:scan:audio-only:0");
    }

    #[test]
    fn multiple_resolution_buckets_each_flagged_and_sorted() {
        let groups = [group(
            "movie",
            Some(2001),
            None,
            None,
            vec![
                file("/m/1080-a.mkv", Some(1080)),
                file("/m/720-a.mkv", Some(720)),
                file("/m/1080-b.mkv", Some(1080)),
                file("/m/720-b.mkv", Some(720)),
            ],
        )];
        let dupes = find_duplicates(&groups);
        assert_eq!(dupes.len(), 2);
        assert_eq!(dupes[0].id.0, "dup:scan:movie:2001:1080");
        assert_eq!(dupes[1].id.0, "dup:scan:movie:2001:720");
    }
}
