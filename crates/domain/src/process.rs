use std::future::Future;
use std::io;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

pub trait ProcessSpawner: Send + Sync {
    fn run(&self, program: &str, args: &[String]) -> impl Future<Output = io::Result<bool>> + Send;

    fn run_captured(
        &self,
        program: &str,
        args: &[String],
    ) -> impl Future<Output = io::Result<CommandOutput>> + Send {
        async move {
            let success = self.run(program, args).await?;
            Ok(CommandOutput {
                success,
                ..CommandOutput::default()
            })
        }
    }

    fn run_streaming(
        &self,
        program: &str,
        args: &[String],
        on_line: &mut (dyn FnMut(OutputStream, &str) + Send),
    ) -> impl Future<Output = io::Result<CommandOutput>> + Send {
        async move {
            let output = self.run_captured(program, args).await?;
            for line in output.stdout.lines() {
                on_line(OutputStream::Stdout, line);
            }
            for line in output.stderr.lines() {
                on_line(OutputStream::Stderr, line);
            }
            Ok(output)
        }
    }
}
