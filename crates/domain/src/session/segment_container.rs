#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentContainer {
    MpegTs,
    Fmp4,
}

const TS_VIDEO: [&str; 5] = ["h264", "hevc", "mpeg2video", "mpeg1video", "mpeg4"];
const TS_AUDIO: [&str; 5] = ["aac", "ac3", "eac3", "mp3", "mp2"];

pub fn segment_container_for(
    copy: bool,
    video: Option<&str>,
    audio: Option<&str>,
) -> SegmentContainer {
    if !copy {
        return SegmentContainer::MpegTs;
    }
    let video_ok = video.is_none_or(|codec| TS_VIDEO.contains(&codec));
    let audio_ok = audio.is_none_or(|codec| TS_AUDIO.contains(&codec));
    if video_ok && audio_ok {
        SegmentContainer::MpegTs
    } else {
        SegmentContainer::Fmp4
    }
}

impl SegmentContainer {
    pub fn segment_extension(self) -> &'static str {
        match self {
            SegmentContainer::MpegTs => "ts",
            SegmentContainer::Fmp4 => "m4s",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SegmentContainer::MpegTs => "mpegts",
            SegmentContainer::Fmp4 => "fmp4",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transcode_always_uses_mpegts_whatever_the_source_codecs() {
        for (video, audio) in [
            (Some("vp9"), Some("opus")),
            (Some("av1"), Some("flac")),
            (Some("h264"), Some("aac")),
        ] {
            assert_eq!(
                segment_container_for(false, video, audio),
                SegmentContainer::MpegTs,
                "a transcode re-encodes to h264 and aac, which mpegts carries"
            );
        }
    }

    #[test]
    fn a_remux_of_codecs_mpegts_carries_stays_on_mpegts() {
        for video in TS_VIDEO {
            for audio in TS_AUDIO {
                assert_eq!(
                    segment_container_for(true, Some(video), Some(audio)),
                    SegmentContainer::MpegTs,
                    "mpegts carries [{video}] with [{audio}]"
                );
            }
        }
    }

    #[test]
    fn a_remux_of_a_video_codec_mpegts_cannot_carry_uses_fmp4() {
        for video in ["vp9", "av1", "vp8", "theora"] {
            assert_eq!(
                segment_container_for(true, Some(video), Some("aac")),
                SegmentContainer::Fmp4,
                "mpegts silently discards [{video}] as a private data stream"
            );
        }
    }

    #[test]
    fn a_remux_of_an_audio_codec_mpegts_cannot_carry_uses_fmp4() {
        for audio in ["opus", "flac", "vorbis", "pcm_s16le"] {
            assert_eq!(
                segment_container_for(true, Some("h264"), Some(audio)),
                SegmentContainer::Fmp4,
                "mpegts cannot carry [{audio}]"
            );
        }
    }

    #[test]
    fn an_absent_track_does_not_force_fmp4() {
        assert_eq!(
            segment_container_for(true, None, Some("aac")),
            SegmentContainer::MpegTs
        );
        assert_eq!(
            segment_container_for(true, Some("h264"), None),
            SegmentContainer::MpegTs
        );
        assert_eq!(
            segment_container_for(true, None, None),
            SegmentContainer::MpegTs
        );
    }

    #[test]
    fn an_unknown_codec_is_treated_as_unsupported() {
        assert_eq!(
            segment_container_for(true, Some("something_new"), Some("aac")),
            SegmentContainer::Fmp4,
            "an unrecognised codec routes to the container that carries anything"
        );
    }

    #[test]
    fn the_extension_matches_the_container() {
        assert_eq!(SegmentContainer::MpegTs.segment_extension(), "ts");
        assert_eq!(SegmentContainer::Fmp4.segment_extension(), "m4s");
    }
}
