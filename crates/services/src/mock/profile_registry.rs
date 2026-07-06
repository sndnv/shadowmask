use std::sync::Arc;

use domain::profile::{CapabilityProfile, ProfileRegistry};

#[derive(Clone)]
pub struct MockProfileRegistry {
    profile: Arc<CapabilityProfile>,
}

impl MockProfileRegistry {
    pub fn new(profile: CapabilityProfile) -> Self {
        Self {
            profile: Arc::new(profile),
        }
    }
}

impl ProfileRegistry for MockProfileRegistry {
    fn resolve(&self, _platform: &str) -> CapabilityProfile {
        self.profile.as_ref().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::profile::{AudioCodecCap, Container, VideoCodecCap};

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
            }],
            audio: vec![AudioCodecCap {
                codec: "aac".to_owned(),
                max_channels: 2,
            }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
        }
    }

    #[test]
    fn resolve_returns_configured_profile() {
        let registry = MockProfileRegistry::new(profile());
        assert_eq!(registry.resolve("web"), profile());
    }
}
