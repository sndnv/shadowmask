#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentContainer {
    MpegTs,
    Fmp4,
}

const TS_VIDEO: [&str; 4] = ["h264", "hevc", "mpeg2video", "mpeg1video"];
const TS_AUDIO: [&str; 6] = ["aac", "ac3", "eac3", "mp3", "mp2", "truehd"];
const FMP4_VIDEO: [&str; 7] = ["h264", "hevc", "mpeg1video", "mpeg2video", "mjpeg", "vp9", "av1"];
const FMP4_AUDIO: [&str; 8] = ["aac", "ac3", "eac3", "mp3", "dts", "opus", "vorbis", "alac"];

pub fn segment_container_for(
    copy: bool,
    video: Option<&str>,
    audio: Option<&str>,
) -> SegmentContainer {
    if !copy || SegmentContainer::MpegTs.carries(video, audio) {
        SegmentContainer::MpegTs
    } else {
        SegmentContainer::Fmp4
    }
}

pub fn segments_carry(video: Option<&str>, audio: Option<&str>) -> bool {
    SegmentContainer::MpegTs.carries(video, audio) || SegmentContainer::Fmp4.carries(video, audio)
}

impl SegmentContainer {
    fn carries(self, video: Option<&str>, audio: Option<&str>) -> bool {
        let (videos, audios): (&[&str], &[&str]) = match self {
            SegmentContainer::MpegTs => (&TS_VIDEO, &TS_AUDIO),
            SegmentContainer::Fmp4 => (&FMP4_VIDEO, &FMP4_AUDIO),
        };
        video.is_none_or(|codec| videos.contains(&codec))
            && audio.is_none_or(|codec| audios.contains(&codec))
    }

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
    fn every_container_names_itself_for_the_log_and_the_file() {
        assert_eq!(SegmentContainer::MpegTs.as_str(), "mpegts");
        assert_eq!(SegmentContainer::Fmp4.as_str(), "fmp4");
        assert_eq!(SegmentContainer::MpegTs.segment_extension(), "ts");
        assert_eq!(SegmentContainer::Fmp4.segment_extension(), "m4s");
    }

    #[test]
    fn a_transcode_always_uses_mpegts_whatever_the_source_codecs() {
        for (video, audio) in
            [(Some("vp9"), Some("opus")), (Some("av1"), Some("flac")), (Some("h264"), Some("aac"))]
        {
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
        assert_eq!(segment_container_for(true, None, Some("aac")), SegmentContainer::MpegTs);
        assert_eq!(segment_container_for(true, Some("h264"), None), SegmentContainer::MpegTs);
        assert_eq!(segment_container_for(true, None, None), SegmentContainer::MpegTs);
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
    fn truehd_rides_in_mpegts_beside_a_video_mpegts_carries() {
        assert_eq!(
            segment_container_for(true, Some("h264"), Some("truehd")),
            SegmentContainer::MpegTs
        );
        assert!(segments_carry(Some("hevc"), Some("truehd")));
    }

    #[test]
    fn a_copy_is_carried_when_one_container_takes_both_streams() {
        for (video, audio) in [
            (Some("h264"), Some("aac")),
            (Some("mpeg1video"), Some("mp3")),
            (Some("vp9"), Some("opus")),
            (Some("av1"), Some("vorbis")),
            (Some("mjpeg"), Some("alac")),
            (Some("hevc"), Some("dts")),
            (Some("mpeg2video"), Some("mp2")),
            (Some("h264"), None),
            (None, None),
        ] {
            assert!(segments_carry(video, audio), "[{video:?}] with [{audio:?}]");
        }
    }

    #[test]
    fn a_codec_neither_container_takes_cannot_be_copied() {
        for video in [
            "mpeg4",
            "msmpeg4v2",
            "msmpeg4v3",
            "wmv1",
            "wmv2",
            "h263",
            "flv1",
            "theora",
            "vp8",
            "dvvideo",
        ] {
            assert!(!segments_carry(Some(video), None), "{video}");
        }
        for audio in ["pcm_s16le", "wmav1", "wmav2", "flac"] {
            assert!(!segments_carry(Some("h264"), Some(audio)), "{audio}");
        }
    }

    #[test]
    fn a_pair_split_across_the_containers_cannot_be_copied() {
        assert!(!segments_carry(Some("vp9"), Some("truehd")));
        assert!(!segments_carry(Some("av1"), Some("truehd")));
        assert!(segments_carry(Some("vp9"), Some("aac")));
    }

    #[test]
    fn the_extension_matches_the_container() {
        assert_eq!(SegmentContainer::MpegTs.segment_extension(), "ts");
        assert_eq!(SegmentContainer::Fmp4.segment_extension(), "m4s");
    }
}
