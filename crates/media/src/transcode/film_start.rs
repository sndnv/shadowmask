use domain::process::ProcessSpawner;

const PROBE_BINARY: &str = "ffprobe";
const FAILURE_LINES: usize = 5;

pub(crate) async fn film_start<S: ProcessSpawner>(spawner: &S, input: &str) -> Result<i64, String> {
    let output = spawner
        .run_captured(PROBE_BINARY, &start_probe_args(input))
        .await
        .map_err(|e| format!("could not run [{PROBE_BINARY}]: {e}"))?;
    match output.success {
        true => Ok(parse_start_us(&output.stdout)),
        false => Err(format!(
            "could not read where [{input}] starts: {}",
            output.failure_detail(FAILURE_LINES)
        )),
    }
}

pub(crate) fn start_probe_args(input: &str) -> Vec<String> {
    ["-v", "error", "-show_entries", "format=start_time", "-of", "csv=p=0", input]
        .map(str::to_owned)
        .to_vec()
}

pub(crate) fn parse_start_us(text: &str) -> i64 {
    text.trim()
        .trim_end_matches(',')
        .parse::<f64>()
        .ok()
        .filter(|seconds| seconds.is_finite())
        .map_or(0, |seconds| (seconds * 1_000_000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use std::io;

    use domain::process::CommandOutput;

    use super::*;

    struct Probe(io::Result<CommandOutput>);

    impl ProcessSpawner for Probe {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            unreachable!("the start is read with captured output")
        }

        async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
            assert_eq!(program, "ffprobe");
            assert_eq!(args, start_probe_args("/media/film.ts"));
            match &self.0 {
                Ok(output) => Ok(output.clone()),
                Err(err) => Err(io::Error::new(err.kind(), err.to_string())),
            }
        }
    }

    fn answered(success: bool, stdout: &str) -> Probe {
        Probe(Ok(CommandOutput {
            success,
            stdout: stdout.to_owned(),
            stderr: if success { String::new() } else { "Invalid data".to_owned() },
            status: if success { String::new() } else { "exit code 1".to_owned() },
        }))
    }

    #[test]
    fn a_start_is_read_to_the_microsecond_on_either_side_of_zero() {
        assert_eq!(parse_start_us("1.400000\n"), 1_400_000);
        assert_eq!(parse_start_us("9.976780"), 9_976_780);
        assert_eq!(parse_start_us("-0.023220\n"), -23_220);
        assert_eq!(parse_start_us("0.000000,"), 0);
    }

    #[test]
    fn a_start_ffprobe_cannot_name_is_zero() {
        assert_eq!(parse_start_us("N/A\n"), 0);
        assert_eq!(parse_start_us(""), 0);
        assert_eq!(parse_start_us("inf"), 0);
    }

    #[tokio::test]
    async fn the_start_comes_from_the_container() {
        assert_eq!(
            film_start(&answered(true, "1.400000\n"), "/media/film.ts").await,
            Ok(1_400_000)
        );
    }

    #[tokio::test]
    async fn a_probe_that_fails_or_cannot_run_says_why() {
        let failed = film_start(&answered(false, ""), "/media/film.ts").await.unwrap_err();
        assert!(failed.starts_with("could not read where [/media/film.ts] starts"), "{failed}");
        let missing = Probe(Err(io::Error::new(io::ErrorKind::NotFound, "No such file")));
        let missing = film_start(&missing, "/media/film.ts").await.unwrap_err();
        assert_eq!(missing, "could not run [ffprobe]: No such file");
    }
}
