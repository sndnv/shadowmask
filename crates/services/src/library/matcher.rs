use std::collections::HashMap;

use domain::library::{
    DiscoveredFile, MatchKey, MatchReport, MatchedGroup, ParsedMedia, UnmatchedFile,
    UnmatchedFileId,
};
use jiff::Timestamp;

use crate::library::normalize_title;
use crate::library::parse::{confidence, parse_filename};

const DEFAULT_THRESHOLD: f32 = 0.5;

pub struct Matcher {
    threshold: f32,
}

impl Matcher {
    pub fn new() -> Self {
        Self { threshold: DEFAULT_THRESHOLD }
    }

    #[cfg(test)]
    pub fn with_threshold(threshold: f32) -> Self {
        Self { threshold }
    }

    pub fn match_files(&self, discovered: &[DiscoveredFile]) -> MatchReport {
        let mut groups: HashMap<MatchKey, (ParsedMedia, f32, Vec<DiscoveredFile>)> = HashMap::new();
        let mut unmatched = Vec::new();
        let now = Timestamp::now();

        for file in discovered {
            let parsed = parse_filename(&file.path);
            let score = confidence(&parsed);
            if parsed.title.is_empty() || score < self.threshold {
                unmatched.push(UnmatchedFile {
                    id: UnmatchedFileId(format!("unmatched:{}", file.path)),
                    library: file.library.clone(),
                    path: file.path.clone(),
                    candidates: Vec::new(),
                    created_at: now,
                    updated_at: now,
                });
                continue;
            }
            let key = MatchKey {
                title_slug: normalize_title(&parsed.title),
                year: parsed.year,
                season: parsed.season,
                episode: parsed.episode,
            };
            let entry = groups.entry(key).or_insert_with(|| (parsed.clone(), score, Vec::new()));
            entry.1 = entry.1.min(score);
            entry.2.push(file.clone());
        }

        let mut matched: Vec<MatchedGroup> = groups
            .into_iter()
            .map(|(key, (parsed, confidence, mut files))| {
                files.sort_by(|a, b| a.path.cmp(&b.path));
                MatchedGroup { key, parsed, confidence, files }
            })
            .collect();
        matched.sort_by(|a, b| a.key.cmp(&b.key));
        unmatched.sort_by(|a, b| a.path.cmp(&b.path));

        MatchReport { matched, unmatched }
    }
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::library::LibraryId;
    use domain::media::ProbeResult;

    fn file(path: &str) -> DiscoveredFile {
        DiscoveredFile {
            library: LibraryId("lib".into()),
            path: path.to_owned(),
            size_bytes: 1,
            subtitle_siblings: Vec::new(),
            probe: ProbeResult {
                duration_ms: 0,
                video: Vec::new(),
                audio: Vec::new(),
                subtitles: Vec::new(),
                chapters: Vec::new(),
            },
        }
    }

    #[test]
    fn groups_multiple_qualities_of_one_movie() {
        let report = Matcher::new().match_files(&[
            file("/m/The Matrix (1999) 1080p.mkv"),
            file("/m/The Matrix (1999) 2160p.mkv"),
        ]);
        assert_eq!(report.matched.len(), 1);
        assert_eq!(report.matched[0].files.len(), 2);
        assert_eq!(report.matched[0].key.title_slug, "the matrix");
        assert_eq!(report.matched[0].key.year, Some(1999));
        assert!(report.unmatched.is_empty());
    }

    #[test]
    fn separate_episodes_are_separate_groups() {
        let report =
            Matcher::new().match_files(&[file("/tv/Show.S01E01.mkv"), file("/tv/Show.S01E02.mkv")]);
        assert_eq!(report.matched.len(), 2);
        assert_eq!(report.matched[0].key.episode, Some(1));
        assert_eq!(report.matched[1].key.episode, Some(2));
    }

    #[test]
    fn low_confidence_file_goes_to_unmatched_queue() {
        let report = Matcher::new().match_files(&[file("/m/recording.mkv")]);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched.len(), 1);
        assert_eq!(report.unmatched[0].id.0, "unmatched:/m/recording.mkv");
        assert_eq!(report.unmatched[0].library, LibraryId("lib".into()));
        assert!(report.unmatched[0].candidates.is_empty());
    }

    #[test]
    fn empty_title_file_goes_to_unmatched_queue() {
        let report = Matcher::new().match_files(&[file("/m/2019.mkv")]);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched.len(), 1);
    }

    #[test]
    fn mixed_input_partitions_deterministically() {
        let report = Matcher::new().match_files(&[
            file("/m/recording.mkv"),
            file("/m/The Matrix (1999).mkv"),
            file("/m/Sinister 2 (2015).mkv"),
        ]);
        assert_eq!(report.matched.len(), 2);
        assert_eq!(report.matched[0].key.title_slug, "sinister 2");
        assert_eq!(report.matched[1].key.title_slug, "the matrix");
        assert_eq!(report.unmatched.len(), 1);
        assert_eq!(report.unmatched[0].path, "/m/recording.mkv");
    }

    #[test]
    fn empty_input_yields_empty_report() {
        let report = Matcher::default().match_files(&[]);
        assert!(report.matched.is_empty());
        assert!(report.unmatched.is_empty());
    }

    #[test]
    fn threshold_override_can_admit_bare_titles() {
        let report = Matcher::with_threshold(0.3).match_files(&[file("/m/recording.mkv")]);
        assert_eq!(report.matched.len(), 1);
        assert_eq!(report.matched[0].key.title_slug, "recording");
        assert!(report.unmatched.is_empty());
    }

    #[test]
    fn unmatched_queue_is_sorted_by_path() {
        let report = Matcher::new().match_files(&[file("/m/zebra.mkv"), file("/m/apple.mkv")]);
        assert!(report.matched.is_empty());
        let paths: Vec<&str> = report.unmatched.iter().map(|u| u.path.as_str()).collect();
        assert_eq!(paths, ["/m/apple.mkv", "/m/zebra.mkv"]);
    }
}
