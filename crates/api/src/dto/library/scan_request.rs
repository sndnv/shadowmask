use serde::Deserialize;

use domain::library::ScanMode;

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    #[serde(default)]
    pub reread: bool,
}

impl ScanRequest {
    pub fn mode(&self) -> ScanMode {
        if self.reread { ScanMode::Reread } else { ScanMode::Normal }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_reread_flag_asks_for_a_reread() {
        let reread: ScanRequest =
            serde_json::from_value(serde_json::json!({"reread": true})).unwrap();
        let empty: ScanRequest = serde_json::from_value(serde_json::json!({})).unwrap();

        assert_eq!(reread.mode(), ScanMode::Reread);
        assert_eq!(empty.mode(), ScanMode::Normal);
    }
}
