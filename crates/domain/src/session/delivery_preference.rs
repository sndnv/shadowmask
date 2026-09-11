#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeliveryPreference {
    #[default]
    Auto,
    NeverConvert,
    AlwaysConvert,
}

impl DeliveryPreference {
    pub fn parse(value: &str) -> Self {
        match value {
            "never" => Self::NeverConvert,
            "always" => Self::AlwaysConvert,
            _ => Self::Auto,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::NeverConvert => "never",
            Self::AlwaysConvert => "always",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_defaults_to_deciding_for_itself() {
        assert_eq!(DeliveryPreference::default(), DeliveryPreference::Auto);
    }

    // An unknown value has to read as auto rather than fail the session: a
    // client on a newer build must not be able to break playback outright.
    #[test]
    fn an_unknown_preference_reads_as_auto() {
        assert_eq!(DeliveryPreference::parse("never"), DeliveryPreference::NeverConvert);
        assert_eq!(DeliveryPreference::parse("always"), DeliveryPreference::AlwaysConvert);
        assert_eq!(DeliveryPreference::parse("auto"), DeliveryPreference::Auto);
        assert_eq!(DeliveryPreference::parse("nonsense"), DeliveryPreference::Auto);
    }

    #[test]
    fn every_preference_names_itself_for_the_wire() {
        assert_eq!(DeliveryPreference::Auto.as_str(), "auto");
        assert_eq!(DeliveryPreference::NeverConvert.as_str(), "never");
        assert_eq!(DeliveryPreference::AlwaysConvert.as_str(), "always");
    }
}
