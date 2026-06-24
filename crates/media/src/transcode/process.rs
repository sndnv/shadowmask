use std::io;
use std::process::Stdio;

use tokio::process::{Child, Command};

pub trait ProcessSpawner: Send + Sync {
    type Child: TranscodeChild;

    fn spawn(&self, program: &str, args: &[String]) -> io::Result<Self::Child>;
}

pub trait TranscodeChild: Send {
    fn kill(&mut self);
    fn has_exited(&mut self) -> bool;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TokioProcessSpawner;

impl ProcessSpawner for TokioProcessSpawner {
    type Child = TokioChild;

    fn spawn(&self, program: &str, args: &[String]) -> io::Result<TokioChild> {
        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        Ok(TokioChild(child))
    }
}

pub struct TokioChild(Child);

impl TranscodeChild for TokioChild {
    fn kill(&mut self) {
        let _ = self.0.start_kill();
    }

    fn has_exited(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(Some(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawns_and_observes_real_process_exit() {
        let spawner = TokioProcessSpawner;
        let mut child = spawner
            .spawn("ffmpeg", &["-version".to_owned()])
            .expect("ffmpeg should spawn");
        let mut exited = false;
        for _ in 0..300 {
            if child.has_exited() {
                exited = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(exited);
    }

    #[tokio::test]
    async fn spawn_of_missing_binary_is_err() {
        let spawner = TokioProcessSpawner;
        let result = spawner.spawn("shadowmask-no-such-binary-xyz", &[]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn kill_does_not_panic() {
        let spawner = TokioProcessSpawner;
        let mut child = spawner
            .spawn("ffmpeg", &["-version".to_owned()])
            .expect("ffmpeg should spawn");
        child.kill();
    }
}
