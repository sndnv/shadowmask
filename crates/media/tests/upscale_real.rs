use domain::media::{MediaProbe, UpscaleProvider, UpscaleSpec};
use media::probe::FfprobeMediaProbe;
use media::upscale::FfmpegUpscaler;

fn make_small_clip(path: &str) {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=320x240:rate=15",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            path,
        ])
        .status()
        .expect("ffmpeg should be installed and on PATH");
    assert!(
        status.success(),
        "ffmpeg failed to generate the source clip"
    );
}

#[tokio::test]
async fn upscales_a_low_res_clip_to_the_target_height() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = dir.path().join("source.mp4").to_string_lossy().into_owned();
    make_small_clip(&source);

    let output = dir.path().join("out.mp4").to_string_lossy().into_owned();
    let out = FfmpegUpscaler::new()
        .upscale(&UpscaleSpec {
            source_path: source,
            target_height: 480,
            output_path: output.clone(),
        })
        .await
        .expect("ffmpeg should upscale; is ffmpeg installed and on PATH?");

    assert_eq!(out.path, output);
    assert!(out.size_bytes > 0, "upscaled file must have bytes");

    let probe = FfprobeMediaProbe::default()
        .probe(&output)
        .await
        .expect("ffprobe should read the upscaled output");
    assert_eq!(
        probe.video[0].height, 480,
        "output height must match target"
    );
    assert_eq!(
        probe.video[0].width, 640,
        "output width must preserve 4:3 aspect"
    );
}
