use std::future::Future;
use std::io;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status: String,
}

impl CommandOutput {
    pub fn failure_detail(&self, max_lines: usize) -> String {
        let text = if self.stderr.trim().is_empty() {
            self.stdout.trim_end()
        } else {
            self.stderr.trim_end()
        };
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return match self.status.trim() {
                "" => "no diagnostic output captured".to_owned(),
                status => format!("no diagnostic output captured ({status})"),
            };
        }
        let tail = lines[lines.len().saturating_sub(max_lines)..].join("\n");
        match self.status.trim() {
            "" => tail,
            status => format!("({status}) {tail}"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_detail_tails_stderr_to_the_last_lines() {
        let stderr = (0..20)
            .map(|i| format!("L{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let detail = CommandOutput {
            success: false,
            stderr,
            status: "exit code 1".to_owned(),
            ..CommandOutput::default()
        }
        .failure_detail(15);
        assert!(detail.contains("L19"));
        assert!(detail.contains("L5"));
        assert!(!detail.contains("L4"));
    }

    #[test]
    fn failure_detail_names_the_status_alongside_the_output() {
        // A killed ffmpeg still lets the encoder print its statistics block, which
        // filled the tail and left no sign that the process had been terminated.
        let detail = CommandOutput {
            success: false,
            stderr: "kb/s:1220.04".to_owned(),
            status: "terminated by signal 15".to_owned(),
            ..CommandOutput::default()
        }
        .failure_detail(10);
        assert!(detail.contains("terminated by signal 15"));
        assert!(detail.contains("kb/s:1220.04"));
    }

    #[test]
    fn failure_detail_falls_back_to_stdout_when_stderr_is_empty() {
        let detail = CommandOutput {
            success: false,
            stdout: "only stdout diagnostic".to_owned(),
            status: "exit code 1".to_owned(),
            ..CommandOutput::default()
        }
        .failure_detail(15);
        assert!(detail.contains("only stdout diagnostic"));
    }

    #[test]
    fn failure_detail_without_output_reports_the_status() {
        let detail = CommandOutput {
            success: false,
            status: "terminated by signal 4".to_owned(),
            ..CommandOutput::default()
        }
        .failure_detail(15);
        assert_eq!(
            detail,
            "no diagnostic output captured (terminated by signal 4)"
        );
    }

    #[test]
    fn failure_detail_without_output_or_status_is_labelled() {
        assert_eq!(
            CommandOutput::default().failure_detail(15),
            "no diagnostic output captured"
        );
    }
}
