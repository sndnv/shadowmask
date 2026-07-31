use std::collections::{HashMap, HashSet};

use domain::error::WalkError;
use domain::library::{DiscoveredFile, Library, ScanReport, SkipReason, SkippedFile, SourceWalker};
use domain::media::MediaProbe;

pub struct Scanner<W, P> {
    walker: W,
    probe: P,
    extensions: Vec<String>,
}

impl<W: SourceWalker, P: MediaProbe> Scanner<W, P> {
    pub fn new(walker: W, probe: P) -> Self {
        Self {
            walker,
            probe,
            extensions: default_extensions(),
        }
    }

    pub fn with_extensions(walker: W, probe: P, extensions: Vec<String>) -> Self {
        Self {
            walker,
            probe,
            extensions,
        }
    }

    pub async fn scan(&self, library: &Library) -> Result<ScanReport, WalkError> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        let mut subtitles: HashMap<String, Vec<String>> = HashMap::new();
        for root in &library.roots {
            for entry in self.walker.walk(root).await? {
                if is_hidden(&entry.path) || !seen.insert(entry.path.clone()) {
                    continue;
                }
                if is_media_candidate(&entry.path, &self.extensions) {
                    candidates.push(entry);
                } else if is_subtitle_file(&entry.path) {
                    subtitles
                        .entry(dir_of(&entry.path).to_owned())
                        .or_default()
                        .push(entry.path);
                }
            }
        }

        let total_candidates = candidates.len();
        let mut discovered = Vec::new();
        let mut skipped = Vec::new();
        for entry in candidates {
            match self.probe.probe(&entry.path).await {
                Ok(probe) => discovered.push(DiscoveredFile {
                    library: library.id.clone(),
                    subtitle_siblings: subtitles
                        .get(dir_of(&entry.path))
                        .cloned()
                        .unwrap_or_default(),
                    path: entry.path,
                    size_bytes: entry.size_bytes,
                    probe,
                }),
                Err(err) => skipped.push(SkippedFile {
                    path: entry.path,
                    reason: SkipReason::ProbeFailed(err.to_string()),
                }),
            }
        }

        Ok(ScanReport {
            discovered,
            skipped,
            total_candidates,
        })
    }
}

fn default_extensions() -> Vec<String> {
    [
        "mkv", "mp4", "avi", "mov", "m4v", "wmv", "flv", "webm", "mpg", "mpeg", "ts", "m2ts",
    ]
    .iter()
    .map(|ext| (*ext).to_owned())
    .collect()
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn is_media_candidate(path: &str, extensions: &[String]) -> bool {
    match basename(path).rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => extensions
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(ext)),
        _ => false,
    }
}

fn is_subtitle_file(path: &str) -> bool {
    match basename(path).rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ["srt", "vtt", "ass", "ssa", "sub"]
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(ext)),
        _ => false,
    }
}

fn dir_of(path: &str) -> &str {
    match path.rsplit_once(['/', '\\']) {
        Some((dir, _)) => dir,
        None => "",
    }
}

fn is_hidden(path: &str) -> bool {
    basename(path).starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockMediaProbe, MockSourceWalker};
    use domain::library::{LibraryId, LibraryKind, LibraryOrigin, WalkedEntry, WatcherStrategy};
    use jiff::Timestamp;

    fn entry(path: &str, size: u64) -> WalkedEntry {
        WalkedEntry {
            path: path.to_owned(),
            size_bytes: size,
        }
    }

    fn library(roots: &[&str]) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            roots: roots.iter().map(|r| (*r).into()).collect(),
            watcher: WatcherStrategy::Manual,
            scan_schedule: None,
            metadata_sources: Vec::new(),
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn candidate_filter_matches_media_extensions_case_insensitively() {
        let exts = default_extensions();
        assert!(is_media_candidate("/m/movie.mkv", &exts));
        assert!(is_media_candidate("/m/Movie.MP4", &exts));
        assert!(!is_media_candidate("/m/notes.txt", &exts));
        assert!(!is_media_candidate("/m/README", &exts));
        assert!(!is_media_candidate("/m/.mkv", &exts));
    }

    #[test]
    fn hidden_detects_dotfiles_by_basename() {
        assert!(is_hidden("/m/.secret.mkv"));
        assert!(!is_hidden("/m/movie.mkv"));
    }

    #[tokio::test]
    async fn scan_discovers_media_and_skips_probe_failures() {
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![
                entry("/m/good.mkv", 10),
                entry("/m/bad.mp4", 20),
                entry("/m/notes.txt", 5),
                entry("/m/.hidden.mkv", 5),
            ],
        );
        let probe = MockMediaProbe::new().failing_on("/m/bad.mp4");
        let scanner = Scanner::new(walker, probe);

        let report = scanner.scan(&library(&["/m"])).await.unwrap();
        assert_eq!(report.total_candidates, 2);
        assert_eq!(report.discovered.len(), 1);
        assert_eq!(report.discovered[0].path, "/m/good.mkv");
        assert_eq!(report.discovered[0].size_bytes, 10);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "/m/bad.mp4");
    }

    #[tokio::test]
    async fn scan_attaches_subtitle_siblings_by_directory() {
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![
                entry("/m/a/movie.mkv", 10),
                entry("/m/a/movie.en.srt", 1),
                entry("/m/a/movie.fr.srt", 1),
                entry("/m/b/other.mp4", 10),
                entry("/m/a/notes.txt", 1),
            ],
        );
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let report = scanner.scan(&library(&["/m"])).await.unwrap();
        let movie = report
            .discovered
            .iter()
            .find(|d| d.path == "/m/a/movie.mkv")
            .unwrap();
        assert_eq!(movie.subtitle_siblings.len(), 2);
        assert!(
            movie
                .subtitle_siblings
                .contains(&"/m/a/movie.en.srt".to_owned())
        );
        assert!(
            movie
                .subtitle_siblings
                .contains(&"/m/a/movie.fr.srt".to_owned())
        );
        let other = report
            .discovered
            .iter()
            .find(|d| d.path == "/m/b/other.mp4")
            .unwrap();
        assert!(other.subtitle_siblings.is_empty());
    }

    #[test]
    fn subtitle_and_dir_helpers() {
        assert!(is_subtitle_file("/m/a.SRT"));
        assert!(is_subtitle_file("/m/a.vtt"));
        assert!(!is_subtitle_file("/m/a.mkv"));
        assert!(!is_subtitle_file("/m/.srt"));
        assert_eq!(dir_of("/m/a/movie.mkv"), "/m/a");
        assert_eq!(dir_of("movie.mkv"), "");
    }

    #[tokio::test]
    async fn scan_dedupes_across_overlapping_roots() {
        let walker = MockSourceWalker::new()
            .with_entries("/a", vec![entry("/shared/movie.mkv", 1)])
            .with_entries("/b", vec![entry("/shared/movie.mkv", 1)]);
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let report = scanner.scan(&library(&["/a", "/b"])).await.unwrap();
        assert_eq!(report.discovered.len(), 1);
    }

    #[tokio::test]
    async fn scan_propagates_walk_failure() {
        let walker = MockSourceWalker::new().with_failing("/missing");
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let err = scanner.scan(&library(&["/missing"])).await.unwrap_err();
        assert!(matches!(err, WalkError::RootNotFound(_)));
    }

    #[tokio::test]
    async fn scan_of_empty_roots_is_empty() {
        let scanner = Scanner::new(MockSourceWalker::new(), MockMediaProbe::new());
        let report = scanner.scan(&library(&[])).await.unwrap();
        assert!(report.discovered.is_empty());
        assert_eq!(report.total_candidates, 0);
    }

    #[tokio::test]
    async fn with_extensions_overrides_the_defaults() {
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![entry("/m/disc.iso", 1), entry("/m/movie.mkv", 1)],
        );
        let scanner =
            Scanner::with_extensions(walker, MockMediaProbe::new(), vec!["iso".to_owned()]);

        let report = scanner.scan(&library(&["/m"])).await.unwrap();
        assert_eq!(report.discovered.len(), 1);
        assert_eq!(report.discovered[0].path, "/m/disc.iso");
    }
}
