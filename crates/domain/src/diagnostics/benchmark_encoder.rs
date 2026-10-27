#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkEncoder {
    Vaapi,
    VaapiEncode,
    Software,
}

impl BenchmarkEncoder {
    pub fn as_str(self) -> &'static str {
        match self {
            BenchmarkEncoder::Vaapi => "vaapi",
            BenchmarkEncoder::VaapiEncode => "vaapi-encode",
            BenchmarkEncoder::Software => "software",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_encoder_reads_as_its_label() {
        assert_eq!(BenchmarkEncoder::Vaapi.as_str(), "vaapi");
        assert_eq!(BenchmarkEncoder::VaapiEncode.as_str(), "vaapi-encode");
        assert_eq!(BenchmarkEncoder::Software.as_str(), "software");
    }
}
