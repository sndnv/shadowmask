use std::collections::{HashMap, HashSet};
use std::future::Future;

use domain::error::WalkError;
use domain::library::{
    DiscoveredFile, Library, ScanReport, SkipReason, SkippedFile, SourceWalker, WalkedEntry,
};
use domain::media::MediaProbe;
use futures::StreamExt;

use super::external_subtitles::sidecar_owner;

pub const DEFAULT_PROBE_CONCURRENCY: usize = 8;

const PROGRESS_EVERY: usize = 200;

pub struct Scanner<W, P> {
    walker: W,
    probe: P,
    extensions: Vec<String>,
    probe_concurrency: usize,
}

impl<W: SourceWalker, P: MediaProbe> Scanner<W, P> {
    pub fn new(walker: W, probe: P) -> Self {
        Self {
            walker,
            probe,
            extensions: default_extensions(),
            probe_concurrency: DEFAULT_PROBE_CONCURRENCY,
        }
    }

    pub fn with_probe_concurrency(mut self, probes: usize) -> Self {
        self.probe_concurrency = probes.max(1);
        self
    }

    #[cfg(test)]
    pub fn with_extensions(walker: W, probe: P, extensions: Vec<String>) -> Self {
        Self { walker, probe, extensions, probe_concurrency: DEFAULT_PROBE_CONCURRENCY }
    }

    pub async fn scan<F, Fut>(
        &self,
        library: &Library,
        on_progress: F,
    ) -> Result<ScanReport, WalkError>
    where
        F: Fn(u32, u32) -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        let mut excluded: Vec<SkippedFile> = Vec::new();
        let mut subtitles: HashMap<String, Vec<String>> = HashMap::new();
        for root in &library.roots {
            for entry in self.walker.walk(root).await? {
                if is_hidden(&entry.path) || !seen.insert(entry.path.clone()) {
                    continue;
                }
                if is_media_candidate(&entry.path, &self.extensions) {
                    if let Some(directory) = excluded_directory(&entry.path, root) {
                        excluded.push(SkippedFile {
                            path: entry.path,
                            reason: SkipReason::ExcludedDirectory(directory),
                        });
                        continue;
                    }
                    candidates.push(entry);
                } else if is_subtitle_file(&entry.path) {
                    subtitles.entry(dir_of(&entry.path).to_owned()).or_default().push(entry.path);
                }
            }
        }

        let total_candidates = candidates.len();
        let mut sidecars = claim_sidecars(&candidates, subtitles);
        let mut discovered = Vec::new();
        let mut skipped = excluded;
        let mut probed = futures::stream::iter(candidates)
            .map(|entry| async move { (self.probe.probe(&entry.path).await, entry) })
            .buffered(self.probe_concurrency);
        let mut done = 0;
        while let Some((result, entry)) = probed.next().await {
            match result {
                Ok(probe) => discovered.push(DiscoveredFile {
                    library: library.id.clone(),
                    subtitle_siblings: sidecars.remove(&entry.path).unwrap_or_default(),
                    path: entry.path,
                    size_bytes: entry.size_bytes,
                    probe,
                }),
                Err(err) => skipped.push(SkippedFile {
                    path: entry.path,
                    reason: SkipReason::ProbeFailed(err.to_string()),
                }),
            }
            done += 1;
            if done % PROGRESS_EVERY == 0 && done < total_candidates {
                on_progress(done as u32, total_candidates as u32).await;
            }
        }

        Ok(ScanReport { discovered, skipped, total_candidates })
    }
}

pub(crate) fn default_extensions() -> Vec<String> {
    ["mkv", "mp4", "avi", "mov", "m4v", "wmv", "flv", "webm", "mpg", "mpeg", "ts", "m2ts"]
        .iter()
        .map(|ext| (*ext).to_owned())
        .collect()
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

const EXTRA_DIRS: [&str; 14] = [
    "extras",
    "featurettes",
    "behind the scenes",
    "deleted scenes",
    "interviews",
    "scenes",
    "shorts",
    "trailers",
    "sample",
    "samples",
    "other",
    "bonus",
    "video_ts",
    "audio_ts",
];

pub(crate) fn excluded_directory(path: &str, root: &str) -> Option<String> {
    let relative = relative_to(path, root)?;
    let mut parts: Vec<&str> = relative.split(['/', '\\']).collect();
    parts.pop();
    parts
        .iter()
        .find(|part| EXTRA_DIRS.contains(&part.trim().to_ascii_lowercase().as_str()))
        .map(|part| (*part).to_owned())
}

fn relative_to<'a>(path: &'a str, root: &str) -> Option<&'a str> {
    let root = root.trim_end_matches(['/', '\\']);
    if root.is_empty() {
        return Some(path);
    }
    let rest = path.strip_prefix(root)?;
    rest.starts_with(['/', '\\']).then_some(rest)
}

fn is_media_candidate(path: &str, extensions: &[String]) -> bool {
    match basename(path).rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => {
            extensions.iter().any(|allowed| allowed.eq_ignore_ascii_case(ext))
        }
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

fn claim_sidecars(
    candidates: &[WalkedEntry],
    subtitles: HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
    let mut videos_by_dir: HashMap<&str, Vec<&str>> = HashMap::new();
    for entry in candidates {
        videos_by_dir.entry(dir_of(&entry.path)).or_default().push(&entry.path);
    }
    let mut claimed: HashMap<String, Vec<String>> = HashMap::new();
    for (dir, found) in subtitles {
        let Some(videos) = videos_by_dir.get_mut(dir.as_str()) else {
            continue;
        };
        videos.sort_unstable();
        for sidecar in found {
            if let Some(owner) = sidecar_owner(&sidecar, videos) {
                claimed.entry(owner.to_owned()).or_default().push(sidecar);
            }
        }
    }
    claimed
}

fn dir_of(path: &str) -> &str {
    match path.rsplit_once(['/', '\\']) {
        Some((dir, _)) => dir,
        None => "",
    }
}

pub(crate) fn is_hidden(path: &str) -> bool {
    basename(path).starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::error::ProbeError;
    use domain::library::{LibraryId, LibraryKind, LibraryOrigin, WalkedEntry, WatcherStrategy};
    use domain::media::ProbeResult;
    use jiff::Timestamp;
    use mocks::{MockMediaProbe, MockSourceWalker};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn is_extra(path: &str, root: &str) -> bool {
        excluded_directory(path, root).is_some()
    }

    fn entry(path: &str, size: u64) -> WalkedEntry {
        WalkedEntry { path: path.to_owned(), size_bytes: size }
    }

    // An extension we ingest but cannot negotiate is a title that appears in the library and then
    // fails at play time, which is worse than never having scanned it.
    #[test]
    fn every_extension_we_ingest_can_be_negotiated() {
        for extension in default_extensions() {
            assert!(
                domain::profile::Container::parse(&extension).is_some(),
                "the scanner ingests [{extension}] but Container::parse rejects it"
            );
        }
    }

    fn recording(seen: Arc<Mutex<Vec<(u32, u32)>>>) -> impl Fn(u32, u32) -> std::future::Ready<()> {
        move |done, total| {
            seen.lock().unwrap().push((done, total));
            std::future::ready(())
        }
    }

    fn quiet() -> impl Fn(u32, u32) -> std::future::Ready<()> {
        recording(Arc::default())
    }

    #[derive(Default)]
    struct WatermarkProbe {
        in_flight: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    }

    impl MediaProbe for WatermarkProbe {
        fn probe(
            &self,
            _path: &str,
        ) -> impl std::future::Future<Output = Result<ProbeResult, ProbeError>> + Send {
            let in_flight = Arc::clone(&self.in_flight);
            let peak = Arc::clone(&self.peak);
            async move {
                let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                tokio::task::yield_now().await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
                Ok(ProbeResult {
                    duration_ms: 1,
                    video: Vec::new(),
                    audio: Vec::new(),
                    subtitles: Vec::new(),
                    chapters: Vec::new(),
                })
            }
        }
    }

    fn many(count: usize) -> MockSourceWalker {
        MockSourceWalker::new()
            .with_entries("/m", (0..count).map(|n| entry(&format!("/m/movie{n}.mkv"), 1)).collect())
    }

    #[tokio::test]
    async fn probes_run_concurrently_up_to_the_configured_ceiling() {
        let probe = WatermarkProbe::default();
        let peak = Arc::clone(&probe.peak);
        let scanner = Scanner::new(many(20), probe).with_probe_concurrency(4);

        let report = scanner.scan(&library(&["/m"]), quiet()).await.unwrap();

        assert_eq!(report.discovered.len(), 20);
        assert_eq!(
            peak.load(Ordering::SeqCst),
            4,
            "a cold probe is a subprocess the scanner waits on, so a serial loop \
             leaves every core but one idle for the length of the scan"
        );
        assert_eq!(
            report.discovered[0].path, "/m/movie0.mkv",
            "concurrency must not reorder the report, so the walk order survives"
        );
    }

    #[tokio::test]
    async fn one_probe_at_a_time_is_still_allowed() {
        let probe = WatermarkProbe::default();
        let peak = Arc::clone(&probe.peak);
        let scanner = Scanner::new(many(5), probe).with_probe_concurrency(0);

        scanner.scan(&library(&["/m"]), quiet()).await.unwrap();

        assert_eq!(
            peak.load(Ordering::SeqCst),
            1,
            "a zero ceiling would stall the stream forever, so it floors at one"
        );
    }

    #[tokio::test]
    async fn a_long_scan_reports_progress_before_it_finishes() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let scanner = Scanner::new(many(1_000), MockMediaProbe::new());

        scanner.scan(&library(&["/m"]), recording(Arc::clone(&seen))).await.unwrap();

        assert_eq!(
            *seen.lock().unwrap(),
            vec![(200, 1_000), (400, 1_000), (600, 1_000), (800, 1_000)],
            "the admin page showed nothing but zero until a scan finished; the last \
             tick is left to the caller, which writes the idle state at one"
        );
    }

    #[tokio::test]
    async fn a_short_scan_reports_nothing_before_it_finishes() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let scanner = Scanner::new(many(3), MockMediaProbe::new());

        scanner.scan(&library(&["/m"]), recording(Arc::clone(&seen))).await.unwrap();

        assert!(
            seen.lock().unwrap().is_empty(),
            "a scan that finishes in a moment must not write a progress row per file"
        );
    }

    fn library(roots: &[&str]) -> Library {
        Library {
            id: LibraryId("lib".into()),
            name: "Lib".into(),
            origin: LibraryOrigin::Local,
            kind: LibraryKind::Movie,
            sort_articles: Vec::new(),
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

    #[test]
    fn extras_are_recognised_by_directory_below_the_root() {
        assert!(is_extra("/tv/Tallowbrook/Extras/Bonus Disc1.mkv", "/tv"));
        assert!(is_extra("/tv/Harbor.Lights/Other/Retrospective.mkv", "/tv"));
        assert!(is_extra("/tv/Show/Show.Season.01/Featurettes/clip.mkv", "/tv"));
        assert!(is_extra("/m/Doc/VIDEO_TS/VTS_01_1.vob", "/m"));
        assert!(!is_extra("/tv/Tallowbrook/Tallowbrook.Season.01/1x01.mkv", "/tv"));
    }

    #[test]
    fn specials_are_not_extras() {
        assert!(!is_extra("/tv/Mr. Quill/Mr.Quill.Specials/S00E01.mkv", "/tv"));
        assert!(!is_extra("/tv/Show/Specials/S00E01.mkv", "/tv"));
    }

    #[test]
    fn a_library_rooted_at_the_filesystem_root_still_finds_its_extras() {
        assert!(is_extra("/Extras/clip.mkv", "/"));
        assert!(!is_extra("/Show/Show.Season.01/1x01.mkv", "/"));
    }

    #[test]
    fn the_root_itself_is_never_read_as_an_extras_folder() {
        assert!(!is_extra("/media/Other/Show/Show.Season.01/1x01.mkv", "/media/Other"));
        assert!(!is_extra("/media/Other/Show/Show.Season.01/1x01.mkv", "/media/Other/"));
    }

    #[test]
    fn a_path_outside_the_root_is_not_excluded_by_it() {
        assert!(!is_extra("/media/Other/Show/1x01.mkv", "/media/Movies"));
        assert!(!is_extra("/media/Other/Show/Extras/clip.mkv", "/media/Movies"));
        assert!(!is_extra("/media/movies-4k/Shorts/Kestrel.mkv", "/media/movies"));
    }

    #[tokio::test]
    async fn scan_leaves_extras_out_of_the_library() {
        let walker = MockSourceWalker::new().with_entries(
            "/tv",
            vec![
                entry("/tv/Tallowbrook/Tallowbrook.Season.01/1x01.mkv", 10),
                entry("/tv/Tallowbrook/Extras/Bonus Disc1.mkv", 20),
            ],
        );
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let report = scanner.scan(&library(&["/tv"]), quiet()).await.unwrap();
        assert_eq!(report.total_candidates, 1);
        assert_eq!(report.discovered.len(), 1);
        assert_eq!(report.discovered[0].path, "/tv/Tallowbrook/Tallowbrook.Season.01/1x01.mkv");
    }

    #[tokio::test]
    async fn an_excluded_file_is_reported_as_seen_rather_than_forgotten() {
        let walker = MockSourceWalker::new().with_entries(
            "/tv",
            vec![
                entry("/tv/Tallowbrook/Tallowbrook.Season.01/1x01.mkv", 10),
                entry("/tv/Tallowbrook/Extras/Bonus Disc1.mkv", 20),
            ],
        );
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let report = scanner.scan(&library(&["/tv"]), quiet()).await.unwrap();

        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "/tv/Tallowbrook/Extras/Bonus Disc1.mkv");
        assert_eq!(report.skipped[0].reason, SkipReason::ExcludedDirectory("Extras".to_owned()));
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

        let report = scanner.scan(&library(&["/m"]), quiet()).await.unwrap();
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

        let report = scanner.scan(&library(&["/m"]), quiet()).await.unwrap();
        let movie = report.discovered.iter().find(|d| d.path == "/m/a/movie.mkv").unwrap();
        assert_eq!(movie.subtitle_siblings.len(), 2);
        assert!(movie.subtitle_siblings.contains(&"/m/a/movie.en.srt".to_owned()));
        assert!(movie.subtitle_siblings.contains(&"/m/a/movie.fr.srt".to_owned()));
        let other = report.discovered.iter().find(|d| d.path == "/m/b/other.mp4").unwrap();
        assert!(other.subtitle_siblings.is_empty());
    }

    #[tokio::test]
    async fn a_sidecar_goes_only_to_the_video_whose_name_it_extends_furthest() {
        let walker = MockSourceWalker::new().with_entries(
            "/m",
            vec![
                entry("/m/a/Neon.Harbor.2017.mkv", 10),
                entry("/m/a/Neon.Harbor.2017.1080p.mkv", 10),
                entry("/m/a/Neon.Harbor.2017.en.srt", 1),
                entry("/m/a/Neon.Harbor.2017.1080p.en.srt", 1),
                entry("/m/b/lantern.mp4", 10),
                entry("/m/b/lantern.mkv", 10),
                entry("/m/b/lantern.en.srt", 1),
                entry("/m/c/stray.en.srt", 1),
            ],
        );
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let report = scanner.scan(&library(&["/m"]), quiet()).await.unwrap();
        let siblings = |path: &str| {
            report.discovered.iter().find(|d| d.path == path).unwrap().subtitle_siblings.clone()
        };

        assert_eq!(siblings("/m/a/Neon.Harbor.2017.mkv"), ["/m/a/Neon.Harbor.2017.en.srt"]);
        assert_eq!(
            siblings("/m/a/Neon.Harbor.2017.1080p.mkv"),
            ["/m/a/Neon.Harbor.2017.1080p.en.srt"]
        );
        assert_eq!(siblings("/m/b/lantern.mkv"), ["/m/b/lantern.en.srt"]);
        assert!(siblings("/m/b/lantern.mp4").is_empty());
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

        let report = scanner.scan(&library(&["/a", "/b"]), quiet()).await.unwrap();
        assert_eq!(report.discovered.len(), 1);
    }

    #[tokio::test]
    async fn scan_propagates_walk_failure() {
        let walker = MockSourceWalker::new().with_failing("/missing");
        let scanner = Scanner::new(walker, MockMediaProbe::new());

        let err = scanner.scan(&library(&["/missing"]), quiet()).await.unwrap_err();
        assert!(matches!(err, WalkError::RootNotFound(_)));
    }

    #[tokio::test]
    async fn scan_of_empty_roots_is_empty() {
        let scanner = Scanner::new(MockSourceWalker::new(), MockMediaProbe::new());
        let report = scanner.scan(&library(&[]), quiet()).await.unwrap();
        assert!(report.discovered.is_empty());
        assert_eq!(report.total_candidates, 0);
    }

    #[tokio::test]
    async fn with_extensions_overrides_the_defaults() {
        let walker = MockSourceWalker::new()
            .with_entries("/m", vec![entry("/m/disc.iso", 1), entry("/m/movie.mkv", 1)]);
        let scanner =
            Scanner::with_extensions(walker, MockMediaProbe::new(), vec!["iso".to_owned()]);

        let report = scanner.scan(&library(&["/m"]), quiet()).await.unwrap();
        assert_eq!(report.discovered.len(), 1);
        assert_eq!(report.discovered[0].path, "/m/disc.iso");
    }
}
