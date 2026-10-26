use serde::Deserialize;

use domain::library::CancelScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelBlocking {
    Purge,
    Clean,
}

impl From<CancelBlocking> for CancelScope {
    fn from(blocking: CancelBlocking) -> Self {
        match blocking {
            CancelBlocking::Purge => CancelScope::PurgeBlockers,
            CancelBlocking::Clean => CancelScope::CleanBlockers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_operation_cancels_what_blocks_it() {
        let purge: CancelBlocking = serde_json::from_str(r#""purge""#).unwrap();
        let clean: CancelBlocking = serde_json::from_str(r#""clean""#).unwrap();

        assert_eq!(CancelScope::from(purge), CancelScope::PurgeBlockers);
        assert_eq!(CancelScope::from(clean), CancelScope::CleanBlockers);
        assert!(serde_json::from_str::<CancelBlocking>(r#""scan""#).is_err());
    }
}
