#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundSettings {
    pub normalize: bool,
    pub dialogue: DialogueLift,
    pub night: NightMode,
}

const STEREO: u8 = 2;
const AAC_LAYOUT_CHANNELS: [u8; 7] = [1, 2, 3, 4, 5, 6, 8];

impl Default for SoundSettings {
    fn default() -> Self {
        Self { normalize: true, dialogue: DialogueLift::None, night: NightMode::Off }
    }
}

impl SoundSettings {
    pub fn channels_sent(&self, source: u8, limit: Option<u8>) -> u8 {
        let lift = self.dialogue != DialogueLift::None;
        if lift && source == STEREO {
            return STEREO;
        }
        let outgoing = limit.map_or(source, |limit| source.min(limit));
        if (lift && outgoing > STEREO) || self.night != NightMode::Off {
            return AAC_LAYOUT_CHANNELS
                .iter()
                .copied()
                .rfind(|channels| *channels <= outgoing)
                .unwrap_or(outgoing);
        }
        outgoing
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DialogueLift {
    #[default]
    None,
    Db3,
    Db6,
    Db9,
}

impl DialogueLift {
    pub fn from_db(db: u8) -> Self {
        match db {
            3 => Self::Db3,
            6 => Self::Db6,
            9 => Self::Db9,
            _ => Self::None,
        }
    }

    pub fn db(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Db3 => 3,
            Self::Db6 => 6,
            Self::Db9 => 9,
        }
    }

    pub fn others_gain(&self) -> f64 {
        10f64.powf(-f64::from(self.db()) / 20.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NightMode {
    #[default]
    Off,
    Light,
    Strong,
}

impl NightMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "strong" => Self::Strong,
            _ => Self::Off,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Light => "light",
            Self::Strong => "strong",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_normalizes_and_leaves_dialogue_and_night_alone_by_default() {
        let settings = SoundSettings::default();
        assert!(settings.normalize);
        assert_eq!(settings.dialogue, DialogueLift::None);
        assert_eq!(settings.night, NightMode::Off);
    }

    #[test]
    fn a_dialogue_lift_reads_from_its_decibels_and_any_other_value_is_none() {
        assert_eq!(DialogueLift::from_db(0), DialogueLift::None);
        assert_eq!(DialogueLift::from_db(3), DialogueLift::Db3);
        assert_eq!(DialogueLift::from_db(6), DialogueLift::Db6);
        assert_eq!(DialogueLift::from_db(9), DialogueLift::Db9);
        assert_eq!(DialogueLift::from_db(4), DialogueLift::None);
        assert_eq!(DialogueLift::from_db(30), DialogueLift::None);
    }

    #[test]
    fn every_dialogue_lift_names_its_decibels() {
        for db in [0, 3, 6, 9] {
            assert_eq!(DialogueLift::from_db(db).db(), db);
        }
    }

    #[test]
    fn a_dialogue_lift_lowers_everything_else_by_its_decibels() {
        for (lift, gain) in [
            (DialogueLift::None, 1.0),
            (DialogueLift::Db3, 0.707_946),
            (DialogueLift::Db6, 0.501_187),
            (DialogueLift::Db9, 0.354_813),
        ] {
            assert!((lift.others_gain() - gain).abs() < 0.000_001, "{lift:?}");
        }
    }

    #[test]
    fn an_unknown_night_mode_reads_as_off() {
        assert_eq!(NightMode::parse("light"), NightMode::Light);
        assert_eq!(NightMode::parse("strong"), NightMode::Strong);
        assert_eq!(NightMode::parse("off"), NightMode::Off);
        assert_eq!(NightMode::parse("loud"), NightMode::Off);
    }

    #[test]
    fn a_conversion_sends_the_source_or_the_limit_whichever_is_fewer() {
        let plain = SoundSettings::default();
        assert_eq!(plain.channels_sent(6, None), 6);
        assert_eq!(plain.channels_sent(6, Some(8)), 6);
        assert_eq!(plain.channels_sent(6, Some(2)), 2);
        assert_eq!(plain.channels_sent(7, None), 7);
        assert_eq!(plain.channels_sent(1, None), 1);
    }

    #[test]
    fn a_lift_on_stereo_sends_stereo_whatever_the_limit() {
        let lifted = SoundSettings { dialogue: DialogueLift::Db9, ..SoundSettings::default() };
        assert_eq!(lifted.channels_sent(2, None), 2);
        assert_eq!(lifted.channels_sent(2, Some(8)), 2);
        assert_eq!(lifted.channels_sent(2, Some(1)), 2);
    }

    #[test]
    fn night_mode_or_a_surround_lift_sends_the_nearest_aac_layout() {
        let night = SoundSettings { night: NightMode::Light, ..SoundSettings::default() };
        let lifted = SoundSettings { dialogue: DialogueLift::Db6, ..SoundSettings::default() };
        for sound in [night, lifted] {
            assert_eq!(sound.channels_sent(7, None), 6, "{sound:?}");
            assert_eq!(sound.channels_sent(7, Some(8)), 6, "{sound:?}");
            assert_eq!(sound.channels_sent(10, None), 8, "{sound:?}");
            assert_eq!(sound.channels_sent(8, None), 8, "{sound:?}");
            assert_eq!(sound.channels_sent(4, None), 4, "{sound:?}");
            assert_eq!(sound.channels_sent(6, Some(2)), 2, "{sound:?}");
            assert_eq!(sound.channels_sent(1, None), 1, "{sound:?}");
        }
    }

    #[test]
    fn every_night_mode_names_itself_for_the_wire() {
        assert_eq!(NightMode::Off.as_str(), "off");
        assert_eq!(NightMode::Light.as_str(), "light");
        assert_eq!(NightMode::Strong.as_str(), "strong");
    }
}
