use std::future::Future;
use std::io;
use std::process::Stdio;

use tokio::process::Command;

pub trait ProcessSpawner: Send + Sync {
    fn run(&self, program: &str, args: &[String]) -> impl Future<Output = io::Result<bool>> + Send;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TokioProcessSpawner;

impl ProcessSpawner for TokioProcessSpawner {
    async fn run(&self, program: &str, args: &[String]) -> io::Result<bool> {
        let status = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;
        Ok(status.success())
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
}
