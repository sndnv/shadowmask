#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum VideoEncoder {
    #[default]
    Software,
    Vaapi {
        device: String,
    },
}

impl VideoEncoder {
    pub fn is_hardware(&self) -> bool {
        matches!(self, VideoEncoder::Vaapi { .. })
    }

    pub fn from_device(device: Option<String>) -> Self {
        match device {
            Some(device) => VideoEncoder::Vaapi { device },
            None => VideoEncoder::Software,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_is_not_hardware() {
        assert!(!VideoEncoder::Software.is_hardware());
        assert_eq!(VideoEncoder::default(), VideoEncoder::Software);
    }

    #[test]
    fn vaapi_is_hardware() {
        let encoder = VideoEncoder::Vaapi {
            device: "/dev/dri/renderD128".to_owned(),
        };
        assert!(encoder.is_hardware());
    }

    #[test]
    fn from_device_maps_presence() {
        assert_eq!(VideoEncoder::from_device(None), VideoEncoder::Software);
        assert_eq!(
            VideoEncoder::from_device(Some("/dev/dri/renderD128".to_owned())),
            VideoEncoder::Vaapi {
                device: "/dev/dri/renderD128".to_owned()
            }
        );
    }
}
