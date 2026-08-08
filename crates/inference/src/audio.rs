use domain::error::TranscriptionError;
use domain::process::ProcessSpawner;
use uuid::Uuid;

pub const SAMPLE_RATE: u32 = 16_000;

pub fn pcm_to_samples(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0)
        .collect()
}

pub async fn decode_audio<S: ProcessSpawner>(
    spawner: &S,
    ffmpeg: &str,
    input_path: &str,
    audio_track_index: Option<u32>,
) -> Result<Vec<f32>, TranscriptionError> {
    let out = std::env::temp_dir().join(format!("shadowmask-asr-{}.pcm", Uuid::new_v4()));
    let out_arg = out.to_string_lossy().into_owned();
    let mut args = vec!["-i".to_owned(), input_path.to_owned()];
    if let Some(index) = audio_track_index {
        args.push("-map".to_owned());
        args.push(format!("0:{index}"));
    }
    args.extend([
        "-vn".to_owned(),
        "-ac".to_owned(),
        "1".to_owned(),
        "-ar".to_owned(),
        SAMPLE_RATE.to_string(),
        "-f".to_owned(),
        "s16le".to_owned(),
        "-y".to_owned(),
        out_arg,
    ]);
    let ok = spawner
        .run(ffmpeg, &args)
        .await
        .map_err(|e| TranscriptionError::Backend(e.to_string()))?;
    if !ok {
        return Err(TranscriptionError::Backend(format!(
            "ffmpeg failed to decode audio from {input_path}"
        )));
    }
    let bytes = tokio::fs::read(&out)
        .await
        .map_err(|e| TranscriptionError::Backend(e.to_string()))?;
    let _ = tokio::fs::remove_file(&out).await;
    Ok(pcm_to_samples(&bytes))
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::{Arc, Mutex};

    use super::*;

    enum Mode {
        Writes(Vec<u8>),
        NoWrite,
        Fails,
        Errors,
    }

    struct MockSpawner {
        mode: Mode,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> io::Result<bool> {
            match &self.mode {
                Mode::Writes(bytes) => {
                    let path = args.last().expect("output path arg");
                    std::fs::write(path, bytes).expect("write mock pcm");
                    Ok(true)
                }
                Mode::NoWrite => Ok(true),
                Mode::Fails => Ok(false),
                Mode::Errors => Err(io::Error::other("spawn failed")),
            }
        }
    }

    #[derive(Clone, Default)]
    struct CapturingSpawner {
        args: Arc<Mutex<Vec<String>>>,
    }

    impl ProcessSpawner for CapturingSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> io::Result<bool> {
            *self.args.lock().unwrap() = args.to_vec();
            let path = args.last().expect("output path arg");
            std::fs::write(path, [0x00, 0x40]).expect("write mock pcm");
            Ok(true)
        }
    }

    #[test]
    fn pcm_to_samples_converts_le_i16_and_drops_odd_tail() {
        let bytes = [0x00, 0x00, 0x00, 0x40, 0x00, 0x80, 0x7f];
        let samples = pcm_to_samples(&bytes);
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0], 0.0);
        assert!((samples[1] - 0.5).abs() < 1e-6);
        assert_eq!(samples[2], -1.0);
    }

    #[tokio::test]
    async fn decode_audio_returns_samples_on_success() {
        let spawner = MockSpawner {
            mode: Mode::Writes(vec![0x00, 0x40]),
        };
        let samples = decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", None)
            .await
            .unwrap();
        assert_eq!(samples.len(), 1);
        assert!((samples[0] - 0.5).abs() < 1e-6);
    }

    #[tokio::test]
    async fn decode_audio_errors_when_ffmpeg_fails() {
        let spawner = MockSpawner { mode: Mode::Fails };
        let err = decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", None)
            .await
            .unwrap_err();
        assert!(matches!(err, TranscriptionError::Backend(_)));
    }

    #[tokio::test]
    async fn decode_audio_errors_when_spawn_errors() {
        let spawner = MockSpawner { mode: Mode::Errors };
        let err = decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", None)
            .await
            .unwrap_err();
        assert!(matches!(err, TranscriptionError::Backend(_)));
    }

    #[tokio::test]
    async fn decode_audio_errors_when_output_missing() {
        let spawner = MockSpawner {
            mode: Mode::NoWrite,
        };
        let err = decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", None)
            .await
            .unwrap_err();
        assert!(matches!(err, TranscriptionError::Backend(_)));
    }

    #[tokio::test]
    async fn decode_audio_maps_specific_track_when_index_given() {
        let spawner = CapturingSpawner::default();
        decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", Some(2))
            .await
            .unwrap();
        let args = spawner.args.lock().unwrap().clone();
        let map = args.iter().position(|a| a == "-map").expect("-map present");
        assert_eq!(args[map + 1], "0:2");
    }

    #[tokio::test]
    async fn decode_audio_omits_map_without_an_index() {
        let spawner = CapturingSpawner::default();
        decode_audio(&spawner, "ffmpeg", "/media/v1.mkv", None)
            .await
            .unwrap();
        let args = spawner.args.lock().unwrap().clone();
        assert!(!args.iter().any(|a| a == "-map"));
    }
}
