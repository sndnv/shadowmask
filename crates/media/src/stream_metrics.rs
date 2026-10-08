use std::time::Duration;

use crate::transcode::VideoEncoder;

pub(crate) fn encoder_label(copy: bool, encoder: &VideoEncoder) -> &'static str {
    if copy {
        "remux"
    } else if encoder.is_hardware() {
        "vaapi"
    } else {
        "software"
    }
}

pub(crate) fn segment_produced(encoder: &'static str, elapsed: Duration, media_ms: u64) {
    metrics::counter!("transcode_segments_total", "encoder" => encoder, "outcome" => "ok")
        .increment(1);
    metrics::histogram!("transcode_segment_duration_milliseconds", "encoder" => encoder)
        .record(millis(elapsed));
    metrics::counter!("transcode_media_milliseconds_total", "encoder" => encoder)
        .increment(media_ms);
}

pub(crate) fn segment_failed(encoder: &'static str) {
    metrics::counter!("transcode_segments_total", "encoder" => encoder, "outcome" => "failed")
        .increment(1);
}

pub(crate) fn fallback() {
    metrics::counter!("transcode_fallbacks_total").increment(1);
}

pub(crate) fn segment_abandoned() {
    metrics::counter!("transcode_segments_abandoned_total").increment(1);
}

pub(crate) fn stream_started(copy: bool, ok: bool) {
    let outcome = if ok { "ok" } else { "failed" };
    metrics::counter!("stream_starts_total", "delivery" => delivery(copy), "outcome" => outcome)
        .increment(1);
}

pub(crate) fn keyframes_read(elapsed: Duration, failed: bool) {
    metrics::histogram!("stream_keyframe_probe_duration_milliseconds").record(millis(elapsed));
    if failed {
        metrics::counter!("stream_keyframe_probe_failures_total").increment(1);
    }
}

pub(crate) fn first_segment(copy: bool, elapsed: Duration) {
    metrics::histogram!("stream_first_segment_milliseconds", "delivery" => delivery(copy))
        .record(millis(elapsed));
}

fn delivery(copy: bool) -> &'static str {
    if copy { "remux" } else { "transcode" }
}

fn millis(elapsed: Duration) -> f64 {
    elapsed.as_nanos() as f64 / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_of_segment_has_its_label() {
        let vaapi = VideoEncoder::Vaapi { device: "/dev/dri/renderD128".to_owned() };

        assert_eq!(encoder_label(true, &vaapi), "remux");
        assert_eq!(encoder_label(false, &vaapi), "vaapi");
        assert_eq!(encoder_label(false, &VideoEncoder::Software), "software");
    }
}
