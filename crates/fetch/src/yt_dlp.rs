use std::collections::HashSet;
use std::path::{Path, PathBuf};

use domain::error::FetchError;
use domain::media::{FetchOptions, FetchSpec, FetchedMedia, MediaFetcher, SiteDetails};
use domain::process::{OutputStream, ProcessSpawner};
use serde::Deserialize;

const REPORT_TEMPLATE: &str =
    "after_move:%(.{filepath,thumbnail,thumbnails,upload_date,extractor_key})j";
const REPORT_SUFFIX: &str = "fetch.json";
const MAX_THUMBNAILS: usize = 3;
const MIN_THUMBNAIL_WIDTH: u64 = 640;
const LAST_MODIFIED_EXTRACTORS: [&str; 2] = ["Generic", "HTML5MediaEmbed"];

#[derive(Deserialize)]
struct Report {
    filepath: Option<String>,
    thumbnail: Option<serde_json::Value>,
    thumbnails: Option<serde_json::Value>,
    upload_date: Option<serde_json::Value>,
    extractor_key: Option<serde_json::Value>,
}

fn text(value: Option<serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(text) => Some(text),
        _ => None,
    }
}

fn names_a_jpeg(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|url| {
        let path = url.path().to_ascii_lowercase();
        path.ends_with(".jpg") || path.ends_with(".jpeg")
    })
}

fn wide_jpeg(mut entry: serde_json::Value) -> Option<String> {
    let width = entry.get("width")?.as_u64()?;
    let height = entry.get("height")?.as_u64()?;
    let url = text(entry.get_mut("url").map(serde_json::Value::take))?;
    let wide =
        width >= MIN_THUMBNAIL_WIDTH && width.saturating_mul(10) >= height.saturating_mul(16);
    (wide && names_a_jpeg(&url)).then_some(url)
}

fn site_thumbnails(
    listed: Option<serde_json::Value>,
    preferred: Option<serde_json::Value>,
) -> Vec<String> {
    let entries = match listed {
        Some(serde_json::Value::Array(entries)) => entries,
        _ => Vec::new(),
    };
    let mut seen = HashSet::new();
    let jpegs: Vec<String> = entries
        .into_iter()
        .rev()
        .filter_map(wide_jpeg)
        .filter(|url| seen.insert(url.clone()))
        .take(MAX_THUMBNAILS)
        .collect();
    if jpegs.is_empty() { text(preferred).into_iter().collect() } else { jpegs }
}

fn site_upload_date(
    upload_date: Option<serde_json::Value>,
    extractor_key: Option<serde_json::Value>,
) -> Option<String> {
    match text(extractor_key) {
        Some(key) if LAST_MODIFIED_EXTRACTORS.contains(&key.as_str()) => None,
        _ => text(upload_date),
    }
}

#[derive(Debug, Clone)]
pub struct YtDlpFetcher<P> {
    spawner: P,
    binary: String,
    plugin_dir: Option<PathBuf>,
    max_height: Option<u32>,
    cookies_file: Option<PathBuf>,
}

impl<P> YtDlpFetcher<P> {
    pub fn new(
        spawner: P,
        binary: impl Into<String>,
        plugin_dir: Option<PathBuf>,
        max_height: Option<u32>,
        cookies_file: Option<PathBuf>,
    ) -> Self {
        Self { spawner, binary: binary.into(), plugin_dir, max_height, cookies_file }
    }
}

pub async fn yt_dlp_version<P: ProcessSpawner>(
    spawner: &P,
    binary: &str,
) -> Result<String, FetchError> {
    let output = spawner
        .run_captured(binary, &["--version".to_owned()])
        .await
        .map_err(|e| FetchError::Spawn(e.to_string()))?;
    if !output.success {
        return Err(FetchError::Spawn(output.failure_detail(DIAGNOSTIC_LINES)));
    }
    match output.stdout.trim() {
        "" => Err(FetchError::NoOutput("no version reported".to_owned())),
        version => Ok(version.to_owned()),
    }
}

fn build_args(
    request: &FetchSpec,
    plugin_dir: Option<&Path>,
    max_height: Option<u32>,
    cookies_file: Option<&Path>,
) -> Vec<String> {
    let dir = escape_template(&request.dest_dir);
    let stem = escape_template(&request.filename_stem);
    let mut args = vec![
        "--no-playlist".to_owned(),
        "--newline".to_owned(),
        "--verbose".to_owned(),
        "--merge-output-format".to_owned(),
        "mkv".to_owned(),
        "-o".to_owned(),
        format!("{dir}/{stem}.%(ext)s"),
        "--print-to-file".to_owned(),
        REPORT_TEMPLATE.to_owned(),
        format!("{dir}/{stem}.{REPORT_SUFFIX}"),
    ];
    let options = &request.options;
    if let Some(selector) = format_selector(options, max_height) {
        args.push("-f".to_owned());
        args.push(selector);
    }
    if options.prefer_h264 {
        args.extend(["-S", "vcodec:h264"].map(str::to_owned));
    }
    if options.site_subtitles {
        args.extend(
            ["--embed-subs", "--sub-langs", "all,-live_chat", "--ignore-errors"].map(str::to_owned),
        );
    }
    if options.chapters {
        args.push("--embed-chapters".to_owned());
    }
    if let Some(dir) = plugin_dir {
        args.push("--plugin-dirs".to_owned());
        args.push(dir.to_string_lossy().into_owned());
    }
    if let Some(cookies) = cookies_file {
        args.push("--cookies".to_owned());
        args.push(cookies.to_string_lossy().into_owned());
    }
    args.push(request.url.clone());
    args
}

fn format_selector(options: &FetchOptions, max_height: Option<u32>) -> Option<String> {
    if let Some(format) = &options.format {
        return Some(format.clone());
    }
    let note = options
        .skip_note
        .as_deref()
        .map(|note| format!("[format_note!~=?\"(?i){note}\"]"))
        .unwrap_or_default();
    match max_height {
        Some(h) => Some(format!(
            "bestvideo[height<={h}]{note}+bestaudio{note}/best[height<={h}]{note}/best{note}"
        )),
        None if note.is_empty() => None,
        None => Some(format!("bestvideo*{note}+bestaudio{note}/best{note}")),
    }
}

fn names_an_unsupported_site(line: &str) -> bool {
    line.starts_with("ERROR:") && line.contains("Unsupported URL: ")
}

fn refused_format(stderr: &str) -> Option<String> {
    stderr.lines().find_map(|line| {
        let reason = line.strip_prefix("SyntaxError: ")?;
        ["Invalid format specification", "Invalid filter specification"]
            .iter()
            .any(|refusal| reason.starts_with(refusal))
            .then(|| reason.to_owned())
    })
}

fn is_progress_line(line: &str) -> bool {
    line.trim_start().starts_with("[download]")
}

fn log_line(url: &str, stream: OutputStream, line: &str) {
    if is_progress_line(line) {
        tracing::info!("yt-dlp [{url}] {}", line.trim_end());
    } else {
        let _ = stream;
        tracing::debug!("yt-dlp [{url}] {}", line.trim_end());
    }
}

const DIAGNOSTIC_LINES: usize = 15;

fn escape_template(text: &str) -> String {
    text.replace('%', "%%")
}

fn report_path(request: &FetchSpec) -> PathBuf {
    Path::new(&request.dest_dir).join(format!("{}.{REPORT_SUFFIX}", request.filename_stem))
}

async fn clear_report(report: &Path) -> Result<(), FetchError> {
    match tokio::fs::remove_file(report).await {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(FetchError::Io(e.to_string())),
        _ => Ok(()),
    }
}

fn reported_file(raw: &str) -> Result<(String, SiteDetails), FetchError> {
    let lines: Vec<&str> = raw.lines().filter(|line| !line.trim().is_empty()).collect();
    let [line] = lines.as_slice() else {
        return Err(FetchError::NoOutput(format!(
            "expected one downloaded file, yt-dlp reported [{}]",
            lines.len()
        )));
    };
    let report: Report = serde_json::from_str(line)
        .map_err(|e| FetchError::NoOutput(format!("unreadable fetch report: {e}")))?;
    let site = SiteDetails {
        thumbnails: site_thumbnails(report.thumbnails, report.thumbnail),
        upload_date: site_upload_date(report.upload_date, report.extractor_key),
    };
    let path = report
        .filepath
        .ok_or_else(|| FetchError::NoOutput("the fetch report names no file".to_owned()))?;
    Ok((path, site))
}

fn read_error(e: &std::io::Error, what: String) -> FetchError {
    match e.kind() {
        std::io::ErrorKind::NotFound => FetchError::NoOutput(format!("{what}: {e}")),
        _ => FetchError::Io(format!("{what}: {e}")),
    }
}

async fn reported_media(report: &Path) -> Result<FetchedMedia, FetchError> {
    let raw = tokio::fs::read_to_string(report).await.map_err(|e| {
        read_error(&e, format!("cannot read the fetch report [{}]", report.display()))
    })?;
    clear_report(report).await?;
    let (path, site) = reported_file(&raw)?;
    let size_bytes = tokio::fs::metadata(&path)
        .await
        .map_err(|e| read_error(&e, format!("cannot read the reported file [{path}]")))?
        .len();
    Ok(FetchedMedia { path, size_bytes, site })
}

impl<P: ProcessSpawner> MediaFetcher for YtDlpFetcher<P> {
    async fn fetch(&self, request: &FetchSpec) -> Result<FetchedMedia, FetchError> {
        tokio::fs::create_dir_all(&request.dest_dir)
            .await
            .map_err(|e| FetchError::Io(e.to_string()))?;
        let report = report_path(request);
        clear_report(&report).await?;
        let args = build_args(
            request,
            self.plugin_dir.as_deref(),
            self.max_height,
            self.cookies_file.as_deref(),
        );
        let mut on_line = |stream: OutputStream, line: &str| log_line(&request.url, stream, line);
        let output = self
            .spawner
            .run_streaming(&self.binary, &args, &mut on_line)
            .await
            .map_err(|e| FetchError::Spawn(e.to_string()))?;
        if !output.success {
            if output.stderr.lines().any(names_an_unsupported_site) {
                return Err(FetchError::Unsupported);
            }
            if let Some(reason) = refused_format(&output.stderr) {
                return Err(FetchError::InvalidFormat(reason));
            }
            return Err(FetchError::Download(format!(
                "yt-dlp failed for [{}]: {}",
                request.url,
                output.failure_detail(DIAGNOSTIC_LINES)
            )));
        }
        reported_media(&report).await
    }
}

#[cfg(test)]
mod tests {
    use domain::process::CommandOutput;

    use super::*;

    enum Mode {
        WriteThenOk,
        ReportWith(serde_json::Value),
        WriteWithoutReport,
        ReportTwice,
        ReportWithoutPath,
        ReportNotJson,
        ReportOfMissingFile,
        ReportThroughAFile,
        ReportIsADirectory,
        OkButEmpty,
        Fail,
        FailWith(&'static str),
        Err,
        Version,
    }

    struct MockSpawner {
        mode: Mode,
    }

    fn arg_after<'a>(args: &'a [String], flag: &str, offset: usize) -> &'a str {
        args.iter().skip_while(|a| a.as_str() != flag).nth(offset).expect("flag argument")
    }

    fn unescape(template: &str) -> String {
        template.replace("%%", "%")
    }

    impl MockSpawner {
        async fn write_outputs(&self, args: &[String]) -> std::io::Result<()> {
            let media = unescape(&arg_after(args, "-o", 1).replace("%(ext)s", "mkv"));
            if !matches!(self.mode, Mode::ReportOfMissingFile) {
                tokio::fs::write(&media, b"video").await?;
            }
            if matches!(self.mode, Mode::WriteWithoutReport) {
                return Ok(());
            }
            let report = unescape(arg_after(args, "--print-to-file", 2));
            if matches!(self.mode, Mode::ReportIsADirectory) {
                return tokio::fs::create_dir(&report).await;
            }
            let line = match &self.mode {
                Mode::ReportWithoutPath => "{}".to_owned(),
                Mode::ReportNotJson => "NA".to_owned(),
                Mode::ReportOfMissingFile => {
                    serde_json::json!({ "filepath": format!("{media}.gone") }).to_string()
                }
                Mode::ReportThroughAFile => {
                    serde_json::json!({ "filepath": format!("{media}/inner.mkv") }).to_string()
                }
                Mode::ReportWith(extra) => {
                    let mut report = serde_json::json!({ "filepath": media });
                    report.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
                    report.to_string()
                }
                _ => serde_json::json!({ "filepath": media }).to_string(),
            };
            let lines = match self.mode {
                Mode::ReportTwice => format!("{line}\n{line}\n"),
                _ => format!("{line}\n"),
            };
            let existing = tokio::fs::read_to_string(&report).await.unwrap_or_default();
            tokio::fs::write(&report, existing + &lines).await
        }
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            match self.mode {
                Mode::Err => Err(std::io::Error::other("spawn failed")),
                Mode::Fail | Mode::FailWith(_) => Ok(false),
                Mode::OkButEmpty | Mode::Version => Ok(true),
                Mode::WriteThenOk
                | Mode::ReportWith(_)
                | Mode::WriteWithoutReport
                | Mode::ReportTwice
                | Mode::ReportWithoutPath
                | Mode::ReportNotJson
                | Mode::ReportOfMissingFile
                | Mode::ReportThroughAFile
                | Mode::ReportIsADirectory => {
                    self.write_outputs(args).await?;
                    Ok(true)
                }
            }
        }

        async fn run_captured(
            &self,
            program: &str,
            args: &[String],
        ) -> std::io::Result<CommandOutput> {
            let success = self.run(program, args).await?;
            Ok(CommandOutput {
                success,
                stdout: match self.mode {
                    Mode::Version => "2026.08.19\n".to_owned(),
                    _ => String::new(),
                },
                stderr: "ERROR: no such option: --version".to_owned(),
                status: "exit code 2".to_owned(),
            })
        }

        async fn run_streaming(
            &self,
            program: &str,
            args: &[String],
            on_line: &mut (dyn FnMut(OutputStream, &str) + Send),
        ) -> std::io::Result<CommandOutput> {
            let success = self.run(program, args).await?;
            on_line(OutputStream::Stderr, "[download]  50.0% of 1.00MiB at 1.00MiB/s");
            on_line(OutputStream::Stderr, "[debug] some verbose detail");
            let stderr = match self.mode {
                Mode::FailWith(stderr) => stderr.to_owned(),
                _ => String::new(),
            };
            Ok(CommandOutput { success, stderr, ..CommandOutput::default() })
        }
    }

    fn request(dest_dir: String) -> FetchSpec {
        FetchSpec {
            url: "https://example.com/watch?v=abc".to_owned(),
            dest_dir,
            filename_stem: "Movie (2020)".to_owned(),
            options: FetchOptions::default(),
        }
    }

    fn spec(options: FetchOptions) -> FetchSpec {
        FetchSpec {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
            options,
        }
    }

    fn fetcher(mode: Mode) -> YtDlpFetcher<MockSpawner> {
        YtDlpFetcher::new(MockSpawner { mode }, "yt-dlp", None, None, None)
    }

    #[test]
    fn build_args_uses_output_template_and_plugin_dir() {
        let req = FetchSpec {
            url: "https://x/v".to_owned(),
            dest_dir: "/d/job1".to_owned(),
            filename_stem: "Show - S01E02".to_owned(),
            options: FetchOptions::default(),
        };
        let args = build_args(&req, Some(Path::new("/plugins")), None, None);
        assert!(args.contains(&"--no-playlist".to_owned()));
        assert!(args.contains(&"--newline".to_owned()));
        assert!(args.contains(&"--verbose".to_owned()));
        assert!(!args.contains(&"--no-progress".to_owned()));
        assert!(args.contains(&"--merge-output-format".to_owned()));
        assert!(args.contains(&"mkv".to_owned()));
        assert!(args.contains(&"--plugin-dirs".to_owned()));
        assert!(args.contains(&"/plugins".to_owned()));
        assert!(!args.contains(&"--cookies".to_owned()));
        assert!(!args.contains(&"-f".to_owned()));
        assert!(args.iter().any(|a| a == "/d/job1/Show - S01E02.%(ext)s"));
        let report = args.iter().position(|a| a == "--print-to-file").expect("report flag");
        assert_eq!(
            args[report + 1],
            "after_move:%(.{filepath,thumbnail,thumbnails,upload_date,extractor_key})j"
        );
        assert_eq!(args[report + 2], "/d/job1/Show - S01E02.fetch.json");
        assert_eq!(args.last().unwrap(), "https://x/v");
    }

    #[test]
    fn a_percent_in_the_directory_or_stem_reaches_yt_dlp_escaped() {
        let req = FetchSpec {
            url: "u".to_owned(),
            dest_dir: "/d/50%% Off/Season 01".to_owned(),
            filename_stem: "50%% Off - S01E02 %(id)s".to_owned(),
            options: FetchOptions::default(),
        };
        let args = build_args(&req, None, None, None);
        assert_eq!(
            arg_after(&args, "-o", 1),
            "/d/50%%%% Off/Season 01/50%%%% Off - S01E02 %%(id)s.%(ext)s"
        );
        assert_eq!(
            arg_after(&args, "--print-to-file", 2),
            "/d/50%%%% Off/Season 01/50%%%% Off - S01E02 %%(id)s.fetch.json"
        );
    }

    #[test]
    fn build_args_without_plugin_dir_omits_flag() {
        let args = build_args(&spec(FetchOptions::default()), None, None, None);
        assert!(!args.contains(&"--plugin-dirs".to_owned()));
    }

    #[test]
    fn build_args_adds_cookies_when_set() {
        let req = spec(FetchOptions::default());
        let args = build_args(&req, None, None, Some(Path::new("/secrets/cookies.txt")));
        let cookies_index = args.iter().position(|a| a == "--cookies").expect("--cookies present");
        assert_eq!(args[cookies_index + 1], "/secrets/cookies.txt");
        assert_eq!(args.last().unwrap(), "u");
    }

    #[test]
    fn build_args_caps_resolution_when_max_height_set() {
        let args = build_args(&spec(FetchOptions::default()), None, Some(1080), None);
        let format_index = args.iter().position(|a| a == "-f").expect("-f present");
        assert_eq!(
            args[format_index + 1],
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best"
        );
    }

    const OPTION_FLAGS: [&str; 7] = [
        "-S",
        "--embed-subs",
        "--sub-langs",
        "--ignore-errors",
        "--embed-chapters",
        "--write-subs",
        "--write-auto-subs",
    ];

    fn premium() -> FetchOptions {
        FetchOptions { skip_note: Some("Premium".into()), ..FetchOptions::default() }
    }

    #[test]
    fn with_every_option_off_the_arguments_are_unchanged() {
        let base = [
            "--no-playlist",
            "--newline",
            "--verbose",
            "--merge-output-format",
            "mkv",
            "-o",
            "/d/x.%(ext)s",
            "--print-to-file",
            REPORT_TEMPLATE,
            "/d/x.fetch.json",
        ];
        let capped = ["-f", "bestvideo[height<=720]+bestaudio/best[height<=720]/best"];
        assert_eq!(
            build_args(&spec(FetchOptions::default()), None, None, None),
            [&base[..], &["u"][..]].concat()
        );
        assert_eq!(
            build_args(&spec(FetchOptions::default()), None, Some(720), None),
            [&base[..], &capped[..], &["u"][..]].concat()
        );
    }

    #[test]
    fn each_tick_box_adds_only_its_own_flags() {
        let h264 = build_args(
            &spec(FetchOptions { prefer_h264: true, ..FetchOptions::default() }),
            None,
            None,
            None,
        );
        assert_eq!(arg_after(&h264, "-S", 1), "vcodec:h264");
        let subtitles = build_args(
            &spec(FetchOptions { site_subtitles: true, ..FetchOptions::default() }),
            None,
            None,
            None,
        );
        assert!(subtitles.contains(&"--embed-subs".to_owned()));
        assert!(subtitles.contains(&"--ignore-errors".to_owned()));
        assert_eq!(arg_after(&subtitles, "--sub-langs", 1), "all,-live_chat");
        let chapters = build_args(
            &spec(FetchOptions { chapters: true, ..FetchOptions::default() }),
            None,
            None,
            None,
        );
        assert!(chapters.contains(&"--embed-chapters".to_owned()));
        for (args, own) in [
            (&h264, &["-S"][..]),
            (&subtitles, &["--embed-subs", "--sub-langs", "--ignore-errors"][..]),
            (&chapters, &["--embed-chapters"][..]),
        ] {
            let flags: Vec<&str> =
                args.iter().map(String::as_str).filter(|a| OPTION_FLAGS.contains(a)).collect();
            assert_eq!(flags, own, "{args:?}");
            assert!(!args.contains(&"-f".to_owned()));
            assert_eq!(args.last().unwrap(), "u");
        }
    }

    #[test]
    fn a_noted_stream_is_skipped_by_every_choice_with_or_without_the_cap() {
        let filter = "[format_note!~=?\"(?i)Premium\"]";
        let uncapped = build_args(&spec(premium()), None, None, None);
        assert_eq!(
            arg_after(&uncapped, "-f", 1),
            format!("bestvideo*{filter}+bestaudio{filter}/best{filter}")
        );
        let capped = build_args(&spec(premium()), None, Some(720), None);
        let cap = "[height<=720]";
        assert_eq!(
            arg_after(&capped, "-f", 1),
            format!("bestvideo{cap}{filter}+bestaudio{filter}/best{cap}{filter}/best{filter}")
        );
    }

    #[test]
    fn a_typed_format_replaces_the_cap_and_keeps_the_h264_preference() {
        let options = FetchOptions {
            format: Some("bv*[height<=480]+ba/b".into()),
            prefer_h264: true,
            ..FetchOptions::default()
        };
        let args = build_args(&spec(options), None, Some(1080), None);
        assert_eq!(arg_after(&args, "-f", 1), "bv*[height<=480]+ba/b");
        assert_eq!(args.iter().filter(|a| *a == "-f").count(), 1);
        assert_eq!(arg_after(&args, "-S", 1), "vcodec:h264");
    }

    #[tokio::test]
    async fn an_unsupported_site_fails_as_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let stderr = "WARNING: [generic] Falling back on generic information extractor\n\
                      ERROR: Unsupported URL: https://example.com/watch?v=abc\n";
        let err = fetcher(Mode::FailWith(stderr)).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Unsupported), "{err:?}");
    }

    #[tokio::test]
    async fn a_format_yt_dlp_cannot_parse_fails_as_invalid_and_names_the_reason() {
        for (stderr, reason) in [
            (
                "Traceback (most recent call last):\n  File \"YoutubeDL.py\", line 2445\n\
                 SyntaxError: Invalid format specification: Expected a selector\n\tbv*+\n\t   ^\n",
                "Invalid format specification: Expected a selector",
            ),
            (
                "Traceback (most recent call last):\n\
                 SyntaxError: Invalid filter specification 'height<>720'\n",
                "Invalid filter specification 'height<>720'",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let dest = dir.path().join("job1").to_string_lossy().into_owned();
            let err = fetcher(Mode::FailWith(stderr)).fetch(&request(dest)).await.unwrap_err();
            assert!(matches!(&err, FetchError::InvalidFormat(r) if r == reason), "{err:?}");
        }
    }

    #[tokio::test]
    async fn any_other_failure_stays_a_download_failure() {
        for stderr in [
            "ERROR: [generic] Unable to download webpage: HTTP Error 403: Forbidden\n",
            "WARNING: Unsupported URL: https://example.com/a\nERROR: fragment 1 not found\n",
            "ERROR: [youtube] abc: Requested format is not available. Use --list-formats\n",
            "SyntaxError: invalid syntax\n",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let dest = dir.path().join("job1").to_string_lossy().into_owned();
            let err = fetcher(Mode::FailWith(stderr)).fetch(&request(dest)).await.unwrap_err();
            let last = stderr.lines().last().unwrap();
            assert!(
                matches!(&err, FetchError::Download(detail) if detail.contains(last)),
                "{err:?}"
            );
        }
    }

    #[tokio::test]
    async fn returns_the_downloaded_file_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let out = fetcher(Mode::WriteThenOk).fetch(&request(dest)).await.unwrap();
        assert!(out.path.ends_with("Movie (2020).mkv"));
        assert!(Path::new(&out.path).exists());
        assert_eq!(out.size_bytes, b"video".len() as u64);
        assert_eq!(out.site, SiteDetails::default());
    }

    async fn site_from(extra: serde_json::Value) -> SiteDetails {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        fetcher(Mode::ReportWith(extra)).fetch(&request(dest)).await.unwrap().site
    }

    fn sized(url: &str, width: u64, height: u64) -> serde_json::Value {
        serde_json::json!({ "url": url, "width": width, "height": height })
    }

    fn bare(url: &str) -> serde_json::Value {
        serde_json::json!({ "url": url })
    }

    #[tokio::test]
    async fn the_site_details_come_back_with_the_wide_jpeg_thumbnails_best_first() {
        let site = site_from(serde_json::json!({
            "thumbnails": [
                sized("https://i.ytimg.com/vi/abc/hqdefault.jpg?sqp=-oaymwE&rs=AOn4", 336, 188),
                bare("https://i.ytimg.com/vi_webp/abc/hqdefault.webp"),
                bare("https://i.ytimg.com/vi/abc/sddefault.jpg"),
                bare("https://i.ytimg.com/vi_webp/abc/sddefault.webp"),
                sized("https://i.ytimg.com/vi/abc/hq720.jpg", 1280, 720),
                bare("https://i.ytimg.com/vi_webp/abc/hq720.webp"),
                sized("https://i.ytimg.com/vi/abc/maxresdefault.jpg", 1280, 720),
                sized("https://i.ytimg.com/vi_webp/abc/maxresdefault.webp", 1920, 1080),
            ],
            "thumbnail": "https://i.ytimg.com/vi_webp/abc/maxresdefault.webp",
            "description": "not asked for, and not kept",
            "extractor_key": "Youtube",
            "upload_date": "20260801",
        }))
        .await;
        assert_eq!(
            site,
            SiteDetails {
                thumbnails: vec![
                    "https://i.ytimg.com/vi/abc/maxresdefault.jpg".to_owned(),
                    "https://i.ytimg.com/vi/abc/hq720.jpg".to_owned(),
                ],
                upload_date: Some("20260801".to_owned()),
            }
        );
    }

    #[tokio::test]
    async fn at_most_three_distinct_thumbnails_are_kept() {
        let site = site_from(serde_json::json!({
            "thumbnails": [
                sized("https://cdn.example/a.jpg", 1280, 720),
                sized("https://cdn.example/b.jpg", 1280, 720),
                sized("https://cdn.example/c.jpg", 1280, 720),
                sized("https://cdn.example/d.jpg", 1280, 720),
                sized("https://cdn.example/d.jpg", 1280, 720),
            ],
        }))
        .await;
        assert_eq!(
            site.thumbnails,
            ["https://cdn.example/d.jpg", "https://cdn.example/c.jpg", "https://cdn.example/b.jpg"]
        );
    }

    #[tokio::test]
    async fn a_jpeg_path_counts_with_a_query_or_in_capitals() {
        let site = site_from(serde_json::json!({
            "thumbnails": [
                sized("https://i.ytimg.com/vi/abc/hq720.jpg?sqp=-oaymwE&rs=AOn4", 1280, 720),
                sized("https://cdn.example/covers/A.JPEG", 1920, 1080),
            ],
        }))
        .await;
        assert_eq!(
            site.thumbnails,
            [
                "https://cdn.example/covers/A.JPEG",
                "https://i.ytimg.com/vi/abc/hq720.jpg?sqp=-oaymwE&rs=AOn4",
            ]
        );
    }

    #[tokio::test]
    async fn a_listed_jpeg_without_a_size_or_too_small_or_not_wide_is_skipped() {
        let site = site_from(serde_json::json!({
            "thumbnails": [
                bare("https://i.ytimg.com/vi/abc/default.jpg"),
                sized("https://i.ytimg.com/vi/abc/mqdefault.jpg", 320, 180),
                sized("https://i.ytimg.com/vi/abc/sddefault.jpg", 640, 480),
                serde_json::json!({ "url": "https://cdn.example/s.jpg", "width": "1280", "height": 720 }),
            ],
            "thumbnail": "https://i.ytimg.com/vi_webp/abc/maxresdefault.webp",
        }))
        .await;
        assert_eq!(
            site.thumbnails,
            ["https://i.ytimg.com/vi_webp/abc/maxresdefault.webp"],
            "with no wide listed JPEG the preferred thumbnail is the only candidate"
        );
    }

    #[tokio::test]
    async fn a_list_without_a_jpeg_path_falls_back_to_the_preferred_thumbnail() {
        let site = site_from(serde_json::json!({
            "thumbnails": [
                sized("https://cdn.example/still.webp", 1920, 1080),
                bare("https://s1.dmcdn.net/v/X/x1080"),
            ],
            "thumbnail": "https://s1.dmcdn.net/v/X/x1080",
        }))
        .await;
        assert_eq!(site.thumbnails, ["https://s1.dmcdn.net/v/X/x1080"]);
    }

    #[tokio::test]
    async fn a_huge_reported_size_does_not_overflow() {
        let site = site_from(serde_json::json!({
            "thumbnails": [sized("https://cdn.example/x.jpg", u64::MAX, u64::MAX)],
        }))
        .await;
        assert_eq!(site.thumbnails, ["https://cdn.example/x.jpg"]);
    }

    #[tokio::test]
    async fn the_upload_date_comes_back_on_its_own() {
        let site = site_from(serde_json::json!({ "upload_date": "20260801" })).await;
        assert_eq!(
            site,
            SiteDetails { upload_date: Some("20260801".to_owned()), ..SiteDetails::default() }
        );
    }

    #[tokio::test]
    async fn a_direct_link_or_a_plain_video_page_reports_no_upload_date() {
        for extractor_key in ["Generic", "HTML5MediaEmbed"] {
            let site = site_from(serde_json::json!({
                "extractor_key": extractor_key,
                "upload_date": "20100512",
            }))
            .await;
            assert_eq!(
                site.upload_date, None,
                "for [{extractor_key}] yt-dlp's date is the Last-Modified header"
            );
        }
    }

    #[tokio::test]
    async fn a_site_detail_that_is_null_or_not_text_is_left_out() {
        let odd_entries = site_from(serde_json::json!({
            "thumbnails": [{ "url": 5 }, { "id": "0" }, "not an entry", sized("relative.jpg", 1280, 720)],
            "thumbnail": null,
            "extractor_key": 7,
            "upload_date": 20260801,
        }))
        .await;
        let not_a_list = site_from(serde_json::json!({ "thumbnails": "https://cdn/a.jpg" })).await;
        assert_eq!(
            odd_entries,
            SiteDetails::default(),
            "an odd optional field must not fail the fetch"
        );
        assert_eq!(not_a_list, SiteDetails::default());
    }

    fn report_in(dest: &str) -> PathBuf {
        Path::new(dest).join("Movie (2020).fetch.json")
    }

    #[tokio::test]
    async fn the_reported_file_wins_over_whatever_else_is_in_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("Season 01");
        tokio::fs::create_dir(&dest).await.unwrap();
        tokio::fs::write(dest.join("Movie (2019).mkv"), b"older episode").await.unwrap();
        tokio::fs::write(dest.join("Movie (2020).f137.mp4.part"), b"partial").await.unwrap();
        let dest = dest.to_string_lossy().into_owned();
        let out = fetcher(Mode::WriteThenOk).fetch(&request(dest.clone())).await.unwrap();
        assert_eq!(out.path, Path::new(&dest).join("Movie (2020).mkv").to_string_lossy());
        assert_eq!(out.size_bytes, b"video".len() as u64);
    }

    #[tokio::test]
    async fn a_title_with_a_double_percent_comes_back_with_its_real_path() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("50%% Off").to_string_lossy().into_owned();
        let spec = FetchSpec { filename_stem: "50%% Off - S01E02".to_owned(), ..request(dest) };
        let out = fetcher(Mode::WriteThenOk).fetch(&spec).await.unwrap();
        assert!(out.path.ends_with("50%% Off/50%% Off - S01E02.mkv"));
        assert!(Path::new(&out.path).exists());
    }

    #[tokio::test]
    async fn the_report_is_removed_once_read() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        fetcher(Mode::WriteThenOk).fetch(&request(dest.clone())).await.unwrap();
        assert!(!report_in(&dest).exists());
    }

    #[tokio::test]
    async fn a_report_left_by_an_earlier_run_is_cleared_first() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1");
        tokio::fs::create_dir(&dest).await.unwrap();
        let dest = dest.to_string_lossy().into_owned();
        tokio::fs::write(report_in(&dest), "{\"filepath\": \"/stale.mkv\"}\n").await.unwrap();
        let out = fetcher(Mode::WriteThenOk).fetch(&request(dest)).await.unwrap();
        assert!(out.path.ends_with("Movie (2020).mkv"));
    }

    #[tokio::test]
    async fn a_report_that_cannot_be_cleared_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1");
        tokio::fs::create_dir(&dest).await.unwrap();
        let dest = dest.to_string_lossy().into_owned();
        tokio::fs::create_dir(report_in(&dest)).await.unwrap();
        let err = fetcher(Mode::WriteThenOk).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn a_missing_report_is_no_output_even_with_media_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::WriteWithoutReport).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn more_than_one_reported_file_is_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportTwice).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(&err, FetchError::NoOutput(detail) if detail.contains("reported [2]")));
    }

    #[tokio::test]
    async fn a_report_without_a_path_is_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportWithoutPath).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn a_report_that_is_not_json_is_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportNotJson).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn a_reported_file_that_is_not_there_is_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportOfMissingFile).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn a_reported_file_that_cannot_be_read_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportThroughAFile).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn a_report_that_cannot_be_read_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::ReportIsADirectory).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn download_failure_is_download_error() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::Fail).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Download(_)));
    }

    #[test]
    fn is_progress_line_detects_download_lines() {
        assert!(is_progress_line("[download]  12.3% of 300.00MiB"));
        assert!(is_progress_line("  [download] Destination: file.mkv"));
        assert!(!is_progress_line("[debug] Command-line config"));
        assert!(!is_progress_line("[youtube] Extracting URL"));
    }

    #[test]
    fn log_line_routes_progress_and_diagnostics() {
        log_line("https://x/v", OutputStream::Stderr, "[download]  50.0% of 1.00MiB");
        log_line("https://x/v", OutputStream::Stderr, "[debug] verbose detail");
    }

    #[tokio::test]
    async fn spawn_error_is_spawn_error() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::Err).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Spawn(_)));
    }

    #[tokio::test]
    async fn empty_output_is_no_output() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let err = fetcher(Mode::OkButEmpty).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn create_dir_failure_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        tokio::fs::write(&blocker, b"x").await.unwrap();
        let dest = blocker.join("nested").to_string_lossy().into_owned();
        let err = fetcher(Mode::WriteThenOk).fetch(&request(dest)).await.unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn version_reports_what_the_binary_printed() {
        let spawner = MockSpawner { mode: Mode::Version };
        assert_eq!(yt_dlp_version(&spawner, "yt-dlp").await.unwrap(), "2026.08.19");
    }

    #[tokio::test]
    async fn a_binary_that_will_not_start_is_a_spawn_error() {
        let spawner = MockSpawner { mode: Mode::Err };
        assert!(matches!(
            yt_dlp_version(&spawner, "yt-dlp").await.unwrap_err(),
            FetchError::Spawn(_)
        ));
    }

    #[tokio::test]
    async fn a_binary_that_rejects_the_flag_reports_why() {
        let spawner = MockSpawner { mode: Mode::Fail };
        // The startup line is the only warning an operator gets, so it has to
        // carry the reason rather than just say the version is unknown.
        let err = yt_dlp_version(&spawner, "yt-dlp").await.unwrap_err();
        assert!(matches!(&err, FetchError::Spawn(detail) if detail.contains("no such option")));
    }

    #[tokio::test]
    async fn a_silent_binary_is_no_output() {
        let spawner = MockSpawner { mode: Mode::OkButEmpty };
        assert!(matches!(
            yt_dlp_version(&spawner, "yt-dlp").await.unwrap_err(),
            FetchError::NoOutput(_)
        ));
    }
}
