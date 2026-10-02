use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureMode {
    #[default]
    Off,
    On,
    Auto,
}

impl FeatureMode {
    pub fn installed(self) -> bool {
        self != FeatureMode::Off
    }

    pub fn automatic(self) -> bool {
        self == FeatureMode::Auto
    }
}

impl std::fmt::Display for FeatureMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FeatureMode::Off => "off",
            FeatureMode::On => "on",
            FeatureMode::Auto => "auto",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_installs_nothing_and_schedules_nothing() {
        assert!(!FeatureMode::Off.installed());
        assert!(!FeatureMode::Off.automatic());
    }

    #[test]
    fn on_installs_without_scheduling() {
        assert!(FeatureMode::On.installed());
        assert!(!FeatureMode::On.automatic());
    }

    #[test]
    fn auto_does_both() {
        assert!(FeatureMode::Auto.installed());
        assert!(FeatureMode::Auto.automatic());
    }

    #[test]
    fn a_mode_reads_back_as_it_was_written() {
        for (mode, text) in
            [(FeatureMode::Off, "off"), (FeatureMode::On, "on"), (FeatureMode::Auto, "auto")]
        {
            assert_eq!(mode.to_string(), text);
            assert_eq!(serde_json::from_str::<FeatureMode>(&format!("\"{text}\"")).unwrap(), mode);
        }
    }
}
