#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationReason {
    VideoCodec,
    VideoBitDepth,
    VideoResolution,
    VideoFrameRate,
    VideoHdr,
    VideoBitrate,
    VideoSoftwareOnly,
    AudioCodec,
    AudioChannels,
    Container,
    SubtitleBurn,
    SubtitleSoft,
    Downmix,
    Forced,
}

impl NegotiationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VideoCodec => "video_codec",
            Self::VideoBitDepth => "video_bit_depth",
            Self::VideoResolution => "video_resolution",
            Self::VideoFrameRate => "video_frame_rate",
            Self::VideoHdr => "video_hdr",
            Self::VideoBitrate => "video_bitrate",
            Self::VideoSoftwareOnly => "video_software_only",
            Self::AudioCodec => "audio_codec",
            Self::AudioChannels => "audio_channels",
            Self::Container => "container",
            Self::SubtitleBurn => "subtitle_burn",
            Self::SubtitleSoft => "subtitle_soft",
            Self::Downmix => "downmix",
            Self::Forced => "forced",
        }
    }
}

impl std::fmt::Display for NegotiationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reason_names_itself_for_the_log() {
        // The negotiation log is the only consumer, and a support report is
        // read by a person, so each variant needs a distinct readable name.
        let all = [
            NegotiationReason::VideoCodec,
            NegotiationReason::VideoBitDepth,
            NegotiationReason::VideoResolution,
            NegotiationReason::VideoFrameRate,
            NegotiationReason::VideoHdr,
            NegotiationReason::VideoBitrate,
            NegotiationReason::VideoSoftwareOnly,
            NegotiationReason::AudioCodec,
            NegotiationReason::AudioChannels,
            NegotiationReason::Container,
            NegotiationReason::SubtitleBurn,
            NegotiationReason::SubtitleSoft,
            NegotiationReason::Downmix,
            NegotiationReason::Forced,
        ];
        let mut names: Vec<&str> = all.iter().map(|reason| reason.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), all.len());
        assert_eq!(NegotiationReason::VideoSoftwareOnly.to_string(), "video_software_only");
    }
}
