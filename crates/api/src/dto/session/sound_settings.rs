use serde::Deserialize;

use domain::session::{DialogueLift, NightMode, SoundSettings};

#[derive(Debug, Deserialize)]
pub struct SoundSettingsDto {
    #[serde(default = "normalized")]
    pub normalize_downmix: bool,
    #[serde(default)]
    pub dialogue_db: u8,
    pub night: Option<String>,
}

fn normalized() -> bool {
    true
}

impl From<SoundSettingsDto> for SoundSettings {
    fn from(s: SoundSettingsDto) -> Self {
        SoundSettings {
            normalize: s.normalize_downmix,
            dialogue: DialogueLift::from_db(s.dialogue_db),
            night: s.night.as_deref().map_or(NightMode::Off, NightMode::parse),
        }
    }
}
