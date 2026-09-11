use crate::error::ProfileError;
use crate::media::HdrFormat;
use crate::profile::{AudioCodecCap, ClientDecoding, Container, VideoCodecCap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityProfile {
    pub containers: Vec<Container>,
    pub video: Vec<VideoCodecCap>,
    pub audio: Vec<AudioCodecCap>,
    pub hdr: Vec<HdrFormat>,
    pub max_width: u32,
    pub max_height: u32,
    pub max_bitrate: u64,
    pub max_frame_rate: Option<u32>,
}

impl CapabilityProfile {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.containers.is_empty() {
            return Err(ProfileError::Invalid("no containers".to_owned()));
        }
        if self.video.is_empty() {
            return Err(ProfileError::Invalid("no video codecs".to_owned()));
        }
        if self.audio.is_empty() {
            return Err(ProfileError::Invalid("no audio codecs".to_owned()));
        }
        if self.max_width == 0 || self.max_height == 0 || self.max_bitrate == 0 {
            return Err(ProfileError::Invalid("non-positive limits".to_owned()));
        }
        if self.max_frame_rate == Some(0) {
            return Err(ProfileError::Invalid("non-positive frame rate".to_owned()));
        }
        Ok(())
    }

    pub fn merged_with(&self, reported: &ClientDecoding) -> Result<Self, ProfileError> {
        let merged = Self {
            containers: reported.containers.clone().unwrap_or_else(|| self.containers.clone()),
            video: reported.video.clone().unwrap_or_else(|| self.video.clone()),
            audio: reported.audio.clone().unwrap_or_else(|| self.audio.clone()),
            hdr: reported.hdr.clone().unwrap_or_else(|| self.hdr.clone()),
            max_width: reported.max_width.unwrap_or(self.max_width),
            max_height: reported.max_height.unwrap_or(self.max_height),
            max_bitrate: reported.max_bitrate.unwrap_or(self.max_bitrate),
            max_frame_rate: reported.max_frame_rate.or(self.max_frame_rate),
        };
        merged.validate()?;
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
                smooth: true,
            }],
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 2 }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 8_000_000,
            max_frame_rate: None,
        }
    }

    #[test]
    fn a_report_replaces_only_what_it_measured() {
        // Android cannot measure container support, so a report that leaves it
        // out has to keep the named profile's containers rather than lose them.
        let reported = ClientDecoding {
            video: Some(vec![VideoCodecCap {
                codec: "hevc".to_owned(),
                max_level: None,
                max_bit_depth: 10,
                smooth: true,
            }]),
            max_height: Some(2160),
            max_frame_rate: Some(60),
            ..ClientDecoding::default()
        };
        let merged = profile().merged_with(&reported).unwrap();
        assert_eq!(merged.containers, vec![Container::Mp4]);
        assert_eq!(merged.audio, profile().audio);
        assert_eq!(merged.video[0].codec, "hevc");
        assert_eq!(merged.max_height, 2160);
        assert_eq!(merged.max_width, 1920);
        assert_eq!(merged.max_frame_rate, Some(60));
    }

    #[test]
    fn an_empty_report_leaves_the_profile_alone() {
        assert_eq!(profile().merged_with(&ClientDecoding::default()).unwrap(), profile());
    }

    #[test]
    fn a_report_that_would_leave_nothing_playable_is_rejected() {
        // The server re-runs its own validation rather than trusting the client,
        // so a client that reports no video codecs at all cannot disable
        // negotiation for itself.
        let reported = ClientDecoding { video: Some(Vec::new()), ..ClientDecoding::default() };
        assert!(matches!(profile().merged_with(&reported), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn a_zero_frame_rate_is_rejected() {
        let reported = ClientDecoding { max_frame_rate: Some(0), ..ClientDecoding::default() };
        assert!(matches!(profile().merged_with(&reported), Err(ProfileError::Invalid(_))));
    }

    #[test]
    fn a_valid_profile_validates() {
        assert!(profile().validate().is_ok());
    }
}
