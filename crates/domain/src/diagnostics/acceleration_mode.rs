#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelerationMode {
    Off,
    Auto,
    Vaapi,
}

impl AccelerationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AccelerationMode::Off => "off",
            AccelerationMode::Auto => "auto",
            AccelerationMode::Vaapi => "vaapi",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_mode_reads_as_its_setting() {
        assert_eq!(AccelerationMode::Off.as_str(), "off");
        assert_eq!(AccelerationMode::Auto.as_str(), "auto");
        assert_eq!(AccelerationMode::Vaapi.as_str(), "vaapi");
    }
}
