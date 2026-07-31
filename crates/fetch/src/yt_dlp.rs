use std::path::{Path, PathBuf};

use domain::error::FetchError;
use domain::media::{FetchRequest, FetchedMedia, MediaFetcher};
use media::transcode::{CommandOutput, OutputStream, ProcessSpawner};

#[derive(Debug, Clone)]
pub struct YtDlpFetcher<P> {
    spawner: P,
    binary: String,
    plugin_dir: Option<PathBuf>,
    max_height: Option<u32>,
}

impl<P> YtDlpFetcher<P> {
    pub fn new(
        spawner: P,
        binary: impl Into<String>,
        plugin_dir: Option<PathBuf>,
        max_height: Option<u32>,
    ) -> Self {
        Self {
            spawner,
            binary: binary.into(),
            plugin_dir,
            max_height,
        }
    }
}

fn build_args(
    request: &FetchRequest,
    plugin_dir: Option<&Path>,
    max_height: Option<u32>,
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
        args.push(format!(
            "bestvideo[height<={height}]+bestaudio/best[height<={height}]/best"
        ));
    }
    if let Some(dir) = plugin_dir {
        args.push("--plugin-dirs".to_owned());
        args.push(dir.to_string_lossy().into_owned());
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

fn error_detail(output: &CommandOutput) -> String {
    let text = if output.stderr.trim().is_empty() {
        output.stdout.trim_end()
    } else {
        output.stderr.trim_end()
    };
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return match output.status.trim() {
            "" => "no diagnostic output captured".to_owned(),
            status => format!("no diagnostic output captured ({status})"),
        };
    }
    let start = lines.len().saturating_sub(15);
    lines[start..].join("\n")
}

async fn first_media_file(dir: &str) -> Result<FetchedMedia, FetchError> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| FetchError::Io(e.to_string()))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| FetchError::Io(e.to_string()))?
    {
        let is_file = entry
            .file_type()
            .await
            .map_err(|e| FetchError::Io(e.to_string()))?
            .is_file();
        if is_file {
            let size_bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            return Ok(FetchedMedia {
                path: entry.path().to_string_lossy().into_owned(),
                size_bytes,
            });
        }
    }
    Err(FetchError::NoOutput(format!(
        "no media produced in [{dir}]"
    )))
}

impl<P: ProcessSpawner> MediaFetcher for YtDlpFetcher<P> {
    async fn fetch(&self, request: &FetchRequest) -> Result<FetchedMedia, FetchError> {
        tokio::fs::create_dir_all(&request.dest_dir)
            .await
            .map_err(|e| FetchError::Io(e.to_string()))?;
        let args = build_args(request, self.plugin_dir.as_deref(), self.max_height);
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
                error_detail(&output)
            )));
        }
        first_media_file(&request.dest_dir).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum Mode {
        WriteThenOk,
        OkButEmpty,
        Fail,
        Err,
    }

    struct MockSpawner {
        mode: Mode,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            match self.mode {
                Mode::Err => Err(std::io::Error::other("spawn failed")),
                Mode::Fail => Ok(false),
                Mode::OkButEmpty => Ok(true),
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

        async fn run_streaming(
            &self,
            program: &str,
            args: &[String],
            on_line: &mut (dyn FnMut(OutputStream, &str) + Send),
        ) -> std::io::Result<CommandOutput> {
            let success = self.run(program, args).await?;
            on_line(
                OutputStream::Stderr,
                "[download]  50.0% of 1.00MiB at 1.00MiB/s",
            );
            on_line(OutputStream::Stderr, "[debug] some verbose detail");
            Ok(CommandOutput {
                success,
                ..CommandOutput::default()
            })
        }
    }

    fn request(dest_dir: String) -> FetchRequest {
        FetchRequest {
            url: "https://example.com/watch?v=abc".to_owned(),
            dest_dir,
            filename_stem: "Movie (2020)".to_owned(),
        }
    }

    fn fetcher(mode: Mode) -> YtDlpFetcher<MockSpawner> {
        YtDlpFetcher::new(MockSpawner { mode }, "yt-dlp", None, None)
    }

    #[test]
    fn build_args_uses_output_template_and_plugin_dir() {
        let req = FetchRequest {
            url: "https://x/v".to_owned(),
            dest_dir: "/d/job1".to_owned(),
            filename_stem: "Show - S01E02".to_owned(),
        };
        let args = build_args(&req, Some(Path::new("/plugins")), None);
        assert!(args.contains(&"--no-playlist".to_owned()));
        assert!(args.contains(&"--newline".to_owned()));
        assert!(args.contains(&"--verbose".to_owned()));
        assert!(!args.contains(&"--no-progress".to_owned()));
        assert!(args.contains(&"--merge-output-format".to_owned()));
        assert!(args.contains(&"mkv".to_owned()));
        assert!(args.contains(&"--plugin-dirs".to_owned()));
        assert!(args.contains(&"/plugins".to_owned()));
        assert!(!args.contains(&"-f".to_owned()));
        assert!(args.iter().any(|a| a == "/d/job1/Show - S01E02.%(ext)s"));
        assert_eq!(args.last().unwrap(), "https://x/v");
    }

    #[test]
    fn build_args_without_plugin_dir_omits_flag() {
        let req = FetchRequest {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
        };
        let args = build_args(&req, None, None);
        assert!(!args.contains(&"--plugin-dirs".to_owned()));
    }

    #[test]
    fn build_args_caps_resolution_when_max_height_set() {
        let req = FetchRequest {
            url: "u".to_owned(),
            dest_dir: "/d".to_owned(),
            filename_stem: "x".to_owned(),
        };
        let args = build_args(&req, None, Some(1080));
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
        let out = fetcher(Mode::WriteThenOk)
            .fetch(&request(dest))
            .await
            .unwrap();
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
    fn error_detail_tails_stderr_to_last_lines() {
        let stderr = (0..20)
            .map(|i| format!("L{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let detail = error_detail(&CommandOutput {
            success: false,
            stdout: String::new(),
            stderr,
            status: "exit code 1".to_owned(),
        });
        assert!(detail.contains("L19"));
        assert!(detail.contains("L5"));
        assert!(!detail.contains("L4"));
    }

    #[test]
    fn error_detail_falls_back_to_stdout_when_stderr_empty() {
        let detail = error_detail(&CommandOutput {
            success: false,
            stdout: "only stdout diagnostic".to_owned(),
            stderr: String::new(),
            status: "exit code 1".to_owned(),
        });
        assert!(detail.contains("only stdout diagnostic"));
    }

    #[test]
    fn error_detail_without_output_reports_status() {
        let detail = error_detail(&CommandOutput {
            success: false,
            status: "terminated by signal 4".to_owned(),
            ..CommandOutput::default()
        });
        assert_eq!(
            detail,
            "no diagnostic output captured (terminated by signal 4)"
        );
    }

    #[test]
    fn error_detail_without_output_or_status_is_labelled() {
        let detail = error_detail(&CommandOutput::default());
        assert_eq!(detail, "no diagnostic output captured");
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
        log_line(
            "https://x/v",
            OutputStream::Stderr,
            "[download]  50.0% of 1.00MiB",
        );
        log_line(
            "https://x/v",
            OutputStream::Stderr,
            "[debug] verbose detail",
        );
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
        let err = fetcher(Mode::OkButEmpty)
            .fetch(&request(dest))
            .await
            .unwrap_err();
        assert!(matches!(err, FetchError::NoOutput(_)));
    }

    #[tokio::test]
    async fn create_dir_failure_is_io() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        tokio::fs::write(&blocker, b"x").await.unwrap();
        let dest = blocker.join("nested").to_string_lossy().into_owned();
        let err = fetcher(Mode::WriteThenOk)
            .fetch(&request(dest))
            .await
            .unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn first_media_file_on_missing_dir_is_io() {
        let err = first_media_file("/no/such/shadowmask/dir")
            .await
            .unwrap_err();
        assert!(matches!(err, FetchError::Io(_)));
    }

    #[tokio::test]
    async fn first_media_file_skips_subdirectories() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(dir.path().join("sub")).await.unwrap();
        tokio::fs::write(dir.path().join("movie.mkv"), b"v")
            .await
            .unwrap();
        let out = first_media_file(&dir.path().to_string_lossy())
            .await
            .unwrap();
        assert!(out.path.ends_with("movie.mkv"));
    }
}
