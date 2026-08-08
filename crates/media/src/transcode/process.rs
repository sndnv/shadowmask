use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Stdio};

use domain::process::{CommandOutput, OutputStream, ProcessSpawner};
use tokio::process::Command;

fn describe_status(status: &ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {code}"),
        None => format!("terminated by signal {}", status.signal().unwrap_or(0)),
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TokioProcessSpawner;

impl ProcessSpawner for TokioProcessSpawner {
    async fn run(&self, program: &str, args: &[String]) -> io::Result<bool> {
        let status = Command::new(program)
            .args(args)
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
        Ok(status.success())
    }

    async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
        let output = Command::new(program)
            .args(args)
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            status: describe_status(&output.status),
        })
    }

    async fn run_streaming(
        &self,
        program: &str,
        args: &[String],
        on_line: &mut (dyn FnMut(OutputStream, &str) + Send),
    ) -> io::Result<CommandOutput> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let mut child = Command::new(program)
            .args(args)
            .kill_on_drop(true)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");

        let (tx, mut rx) = tokio::sync::mpsc::channel::<(OutputStream, String)>(256);
        let tx_err = tx.clone();
        let out_task = tokio::spawn(async move {
            let mut acc = String::new();
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                acc.push_str(&line);
                acc.push('\n');
                let _ = tx.send((OutputStream::Stdout, line)).await;
            }
            acc
        });
        let err_task = tokio::spawn(async move {
            let mut acc = String::new();
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                acc.push_str(&line);
                acc.push('\n');
                let _ = tx_err.send((OutputStream::Stderr, line)).await;
            }
            acc
        });

        while let Some((stream, line)) = rx.recv().await {
            on_line(stream, &line);
        }

        let stdout_acc = out_task.await.map_err(io::Error::other)?;
        let stderr_acc = err_task.await.map_err(io::Error::other)?;
        let status = child.wait().await?;
        Ok(CommandOutput {
            success: status.success(),
            stdout: stdout_acc,
            stderr: stderr_acc,
            status: describe_status(&status),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_reports_success_for_a_quick_command() {
        let ok = TokioProcessSpawner
            .run("ffmpeg", &["-version".to_owned()])
            .await
            .expect("ffmpeg should run; is it installed and on PATH?");
        assert!(ok);
    }

    #[tokio::test]
    async fn run_of_missing_binary_is_err() {
        let result = TokioProcessSpawner
            .run("shadowmask-no-such-binary-xyz", &[])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_reports_failure_for_bad_args() {
        let ok = TokioProcessSpawner
            .run("ffmpeg", &["-shadowmask-not-a-flag".to_owned()])
            .await
            .expect("ffmpeg should spawn even with a bad flag");
        assert!(!ok);
    }

    #[tokio::test]
    async fn run_captured_returns_stdout_on_success() {
        let out = TokioProcessSpawner
            .run_captured("ffmpeg", &["-version".to_owned()])
            .await
            .expect("ffmpeg should run; is it installed and on PATH?");
        assert!(out.success);
        assert!(out.stdout.contains("ffmpeg") || out.stderr.contains("ffmpeg"));
        assert_eq!(out.status, "exit code 0");
    }

    #[tokio::test]
    async fn run_captured_reports_failure_with_diagnostics() {
        let out = TokioProcessSpawner
            .run_captured("ffmpeg", &["-shadowmask-not-a-flag".to_owned()])
            .await
            .expect("ffmpeg should spawn even with a bad flag");
        assert!(!out.success);
        assert!(!out.stderr.is_empty());
        assert!(out.status.starts_with("exit code"));
    }

    #[tokio::test]
    async fn run_captured_reports_signal_termination() {
        let out = TokioProcessSpawner
            .run_captured("sh", &["-c".to_owned(), "kill -s KILL $$".to_owned()])
            .await
            .expect("sh should spawn");
        assert!(!out.success);
        assert_eq!(out.status, "terminated by signal 9");
    }

    #[tokio::test]
    async fn run_captured_of_missing_binary_is_err() {
        let result = TokioProcessSpawner
            .run_captured("shadowmask-no-such-binary-xyz", &[])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn run_streaming_delivers_lines_live_and_accumulates() {
        let mut seen: Vec<(OutputStream, String)> = Vec::new();
        let out = {
            let mut on_line = |stream: OutputStream, line: &str| {
                seen.push((stream, line.to_owned()));
            };
            TokioProcessSpawner
                .run_streaming(
                    "sh",
                    &[
                        "-c".to_owned(),
                        "echo out1; echo err1 1>&2; echo out2".to_owned(),
                    ],
                    &mut on_line,
                )
                .await
                .expect("sh should spawn")
        };
        assert!(out.success);
        assert_eq!(out.status, "exit code 0");
        assert!(out.stdout.contains("out1") && out.stdout.contains("out2"));
        assert!(out.stderr.contains("err1"));
        assert!(seen.contains(&(OutputStream::Stdout, "out1".to_owned())));
        assert!(seen.contains(&(OutputStream::Stderr, "err1".to_owned())));
    }

    #[tokio::test]
    async fn run_streaming_of_missing_binary_is_err() {
        let mut on_line = |_: OutputStream, _: &str| {};
        let result = TokioProcessSpawner
            .run_streaming("shadowmask-no-such-binary-xyz", &[], &mut on_line)
            .await;
        assert!(result.is_err());
    }

    struct RunOnlySpawner;

    impl ProcessSpawner for RunOnlySpawner {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn default_run_captured_wraps_run_with_empty_output() {
        let out = RunOnlySpawner.run_captured("x", &[]).await.unwrap();
        assert!(out.success);
        assert_eq!(out.stdout, "");
        assert_eq!(out.stderr, "");
        assert_eq!(out.status, "");
    }

    struct CapturedOnlySpawner;

    impl ProcessSpawner for CapturedOnlySpawner {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            Ok(true)
        }

        async fn run_captured(
            &self,
            _program: &str,
            _args: &[String],
        ) -> io::Result<CommandOutput> {
            Ok(CommandOutput {
                success: true,
                stdout: "a\nb".to_owned(),
                stderr: "c".to_owned(),
                status: "exit code 0".to_owned(),
            })
        }
    }

    #[tokio::test]
    async fn default_run_streaming_replays_captured_lines() {
        assert!(CapturedOnlySpawner.run("x", &[]).await.unwrap());
        let mut seen: Vec<(OutputStream, String)> = Vec::new();
        let out = {
            let mut on_line = |stream: OutputStream, line: &str| {
                seen.push((stream, line.to_owned()));
            };
            CapturedOnlySpawner
                .run_streaming("x", &[], &mut on_line)
                .await
                .unwrap()
        };
        assert_eq!(out.stdout, "a\nb");
        assert_eq!(
            seen,
            vec![
                (OutputStream::Stdout, "a".to_owned()),
                (OutputStream::Stdout, "b".to_owned()),
                (OutputStream::Stderr, "c".to_owned()),
            ]
        );
    }
}
