use serde::{Deserialize, Serialize};

use domain::common::Quality;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityDto {
    Sd,
    Hd,
    Fhd,
    Uhd,
}

impl From<Quality> for QualityDto {
    fn from(q: Quality) -> Self {
        match q {
            Quality::Sd => QualityDto::Sd,
            Quality::Hd => QualityDto::Hd,
            Quality::Fhd => QualityDto::Fhd,
            Quality::Uhd => QualityDto::Uhd,
        }
    }
}
