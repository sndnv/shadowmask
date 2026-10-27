#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum VideoEncoder {
    #[default]
    Software,
    VaapiEncode {
        device: String,
    },
    Vaapi {
        device: String,
        tone_map: bool,
    },
}

impl VideoEncoder {
    pub fn is_hardware(&self) -> bool {
        !matches!(self, VideoEncoder::Software)
    }

    pub fn device(&self) -> Option<&str> {
        match self {
            VideoEncoder::Software => None,
            VideoEncoder::VaapiEncode { device } | VideoEncoder::Vaapi { device, .. } => {
                Some(device.as_str())
            }
        }
    }

    pub fn from_device(device: Option<String>) -> Self {
        match device {
            Some(device) => VideoEncoder::VaapiEncode { device },
            None => VideoEncoder::Software,
        }
    }

    pub fn for_streaming(device: Option<String>, decoding: bool, tone_map: bool) -> Self {
        match device {
            Some(device) if decoding => VideoEncoder::Vaapi { device, tone_map },
            device => Self::from_device(device),
        }
    }

    pub fn attempts(&self) -> Vec<VideoEncoder> {
        match self {
            VideoEncoder::Software => vec![VideoEncoder::Software],
            VideoEncoder::VaapiEncode { .. } => vec![self.clone(), VideoEncoder::Software],
            VideoEncoder::Vaapi { device, .. } => vec![
                self.clone(),
                VideoEncoder::VaapiEncode { device: device.clone() },
                VideoEncoder::Software,
            ],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            VideoEncoder::Software => "software",
            VideoEncoder::VaapiEncode { .. } => "vaapi-encode",
            VideoEncoder::Vaapi { .. } => "vaapi",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICE: &str = "/dev/dri/renderD128";

    fn encode_only() -> VideoEncoder {
        VideoEncoder::VaapiEncode { device: DEVICE.to_owned() }
    }

    fn on_the_gpu(tone_map: bool) -> VideoEncoder {
        VideoEncoder::Vaapi { device: DEVICE.to_owned(), tone_map }
    }

    #[test]
    fn software_is_not_hardware() {
        assert!(!VideoEncoder::Software.is_hardware());
        assert_eq!(VideoEncoder::default(), VideoEncoder::Software);
        assert_eq!(VideoEncoder::Software.device(), None);
    }

    #[test]
    fn both_vaapi_paths_are_hardware_on_their_device() {
        for encoder in [encode_only(), on_the_gpu(true)] {
            assert!(encoder.is_hardware());
            assert_eq!(encoder.device(), Some(DEVICE));
        }
    }

    #[test]
    fn from_device_encodes_on_the_gpu_only() {
        assert_eq!(VideoEncoder::from_device(None), VideoEncoder::Software);
        assert_eq!(VideoEncoder::from_device(Some(DEVICE.to_owned())), encode_only());
    }

    #[test]
    fn streaming_decodes_on_the_gpu_unless_decoding_is_off() {
        let device = || Some(DEVICE.to_owned());
        assert_eq!(VideoEncoder::for_streaming(device(), true, false), on_the_gpu(false));
        assert_eq!(VideoEncoder::for_streaming(device(), true, true), on_the_gpu(true));
        assert_eq!(VideoEncoder::for_streaming(device(), false, true), encode_only());
        assert_eq!(VideoEncoder::for_streaming(None, true, true), VideoEncoder::Software);
    }

    #[test]
    fn each_path_falls_back_one_step_at_a_time_to_software() {
        assert_eq!(
            on_the_gpu(true).attempts(),
            [on_the_gpu(true), encode_only(), VideoEncoder::Software]
        );
        assert_eq!(encode_only().attempts(), [encode_only(), VideoEncoder::Software]);
        assert_eq!(VideoEncoder::Software.attempts(), [VideoEncoder::Software]);
    }

    #[test]
    fn each_path_has_its_label() {
        assert_eq!(on_the_gpu(false).label(), "vaapi");
        assert_eq!(encode_only().label(), "vaapi-encode");
        assert_eq!(VideoEncoder::Software.label(), "software");
    }
}
