use serde::Serialize;

use domain::metadata::ContentRating;

#[derive(Debug, Serialize)]
pub struct RatingSystem {
    pub system: String,
    pub codes: Vec<String>,
}

impl RatingSystem {
    pub fn known() -> Vec<RatingSystem> {
        ContentRating::known_systems()
            .into_iter()
            .map(|(system, codes)| RatingSystem {
                system: system.to_owned(),
                codes: codes.into_iter().map(|c| c.to_owned()).collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_every_system_with_its_codes() {
        let value = serde_json::to_value(RatingSystem::known()).unwrap();

        let systems = value.as_array().unwrap();
        assert_eq!(systems.len(), 4);
        assert_eq!(systems[0]["system"], "mpaa");
        assert!(
            systems[0]["codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c == "pg-13")
        );
    }
}
