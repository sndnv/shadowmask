use std::path::{Path, PathBuf};

use domain::error::FetchError;
use domain::media::{FetchSpec, FetchedMedia, MediaFetcher};
use domain::process::{OutputStream, ProcessSpawner};

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
    let mut args = vec![
        "--no-playlist".to_owned(),
        "--newline".to_owned(),
        "--verbose".to_owned(),
        "--merge-output-format".to_owned(),
        "mkv".to_owned(),
        "-o".to_owned(),
        format!("{}/{}.%(ext)s", request.dest_dir, request.filename_stem),
    ];
    if let Some(height) = max_height {
        args.push("-f".to_owned());
        args.push(format!("bestvideo[height<={height}]+bestaudio/best[height<={height}]/best"));
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

async fn first_media_file(dir: &str) -> Result<FetchedMedia, FetchError> {
    let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| FetchError::Io(e.to_string()))?;
    while let Some(entry) = entries.next_entry().await.map_err(|e| FetchError::Io(e.to_string()))? {
        let is_file = entry.file_type().await.map_err(|e| FetchError::Io(e.to_string()))?.is_file();
        if is_file {
            let size_bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            return Ok(FetchedMedia {
                path: entry.path().to_string_lossy().into_owned(),
                size_bytes,
            });
        }
    }
    Err(FetchError::NoOutput(format!("no media produced in [{dir}]")))
}

impl<P: ProcessSpawner> MediaFetcher for YtDlpFetcher<P> {
    async fn fetch(&self, request: &FetchSpec) -> Result<FetchedMedia, FetchError> {
        tokio::fs::create_dir_all(&request.dest_dir)
            .await
            .map_err(|e| FetchError::Io(e.to_string()))?;
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
            return Err(FetchError::Download(format!(
                "yt-dlp failed for [{}]: {}",
                request.url,
                output.failure_detail(DIAGNOSTIC_LINES)
            )));
        }
        first_media_file(&request.dest_dir).await
    }
}

#[cfg(test)]
mod tests {
    use domain::process::CommandOutput;

    use super::*;

    enum Mode {
        WriteThenOk,
        OkButEmpty,
        Fail,
        Err,
        Version,
    }

    struct MockSpawner {
        mode: Mode,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            match self.mode {
                Mode::Err => Err(std::io::Error::other("spawn failed")),
                Mode::Fail => Ok(false),
                Mode::OkButEmpty | Mode::Version => Ok(true),
                Mode::WriteThenOk => {
                    let template = args
                        .iter()
                        .skip_while(|a| a.as_str() != "-o")
                        .nth(1)
                        .expect("output template arg");
                    let path = template.replace("%(ext)s", "mkv");
                    tokio::fs::write(&path, b"video").await?;
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
            Ok(CommandOutput { success, ..CommandOutput::default() })
        }
    }

    fn request(dest_dir: String) -> FetchSpec {
        FetchSpec {
            url: "https://example.com/watch?v=abc".to_owned(),
            dest_dir,
            filename_stem: "Movie (2020)".to_owned(),
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
        assert_eq!(args.last().unwrap(), "https://x/v");
    }

    #[test]
    fn build_args_without_plugin_dir_omits_flag() {
        let req = FetchSpec {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
        };
        let args = build_args(&req, None, None, None);
        assert!(!args.contains(&"--plugin-dirs".to_owned()));
    }

    #[test]
    fn build_args_adds_cookies_when_set() {
        let req = FetchSpec {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
        };
        let args = build_args(&req, None, None, Some(Path::new("/secrets/cookies.txt")));
        let cookies_index = args.iter().position(|a| a == "--cookies").expect("--cookies present");
        assert_eq!(args[cookies_index + 1], "/secrets/cookies.txt");
        assert_eq!(args.last().unwrap(), "u");
    }

    #[test]
    fn build_args_caps_resolution_when_max_height_set() {
        let req = FetchSpec {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
        };
        let args = build_args(&req, None, Some(1080), None);
        let format_index = args.iter().position(|a| a == "-f").expect("-f present");
        assert_eq!(
            args[format_index + 1],
            "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best"
        );
    }

    #[tokio::test]
    async fn returns_the_downloaded_file_on_success() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("job1").to_string_lossy().into_owned();
        let out = fetcher(Mode::WriteThenOk).fetch(&request(dest)).await.unwrap();
        assert!(out.path.ends_with("Movie (2020).mkv"));
        assert!(Path::new(&out.path).exists());
        assert_eq!(out.size_bytes, b"video".len() as u64);
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
    async fn first_media_file_on_missing_dir_is_io() {
        let err = first_media_file("/no/such/shadowmask/dir").await.unwrap_err();
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

    #[tokio::test]
    async fn first_media_file_skips_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(dir.path().join("sub")).await.unwrap();
        tokio::fs::write(dir.path().join("movie.mkv"), b"v").await.unwrap();
        let out = first_media_file(&dir.path().to_string_lossy()).await.unwrap();
        assert!(out.path.ends_with("movie.mkv"));
    }
}
