use crate::diagnostics::Presence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegReport {
    pub version: Option<String>,
    pub error: Option<String>,
    pub hwaccels: Vec<String>,
    pub encoders: Vec<Presence>,
    pub filters: Vec<Presence>,
}

impl FfmpegReport {
    pub fn has_encoder(&self, name: &str) -> bool {
        self.encoders.iter().any(|encoder| encoder.name == name && encoder.present)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_encoder_counts_only_when_present() {
        let report = FfmpegReport {
            version: None,
            error: None,
            hwaccels: Vec::new(),
            encoders: vec![
                Presence { name: "libx264".to_owned(), present: true },
                Presence { name: "h264_vaapi".to_owned(), present: false },
            ],
            filters: Vec::new(),
        };

        assert!(report.has_encoder("libx264"));
        assert!(!report.has_encoder("h264_vaapi"));
        assert!(!report.has_encoder("hevc_vaapi"));
    }
}
