use domain::library::{LibraryId, ScanMode};
use serde::{Deserialize, Serialize};

use crate::job::encode_payload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanJobPayload {
    pub library: LibraryId,
    pub mode: ScanMode,
}

impl ScanJobPayload {
    pub fn encode(&self) -> String {
        match self.mode {
            ScanMode::Normal => self.library.0.clone(),
            ScanMode::Reread => {
                encode_payload(&Wire { library: self.library.0.clone(), reread: true })
            }
        }
    }

    pub fn decode(raw: &str) -> Self {
        match serde_json::from_str::<Wire>(raw) {
            Ok(wire) => Self {
                library: LibraryId(wire.library),
                mode: if wire.reread { ScanMode::Reread } else { ScanMode::Normal },
            },
            Err(_) => Self { library: LibraryId(raw.to_owned()), mode: ScanMode::Normal },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Wire {
    library: String,
    reread: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(library: &str, mode: ScanMode) -> ScanJobPayload {
        ScanJobPayload { library: LibraryId(library.into()), mode }
    }

    #[test]
    fn a_normal_scan_keeps_the_plain_library_id() {
        assert_eq!(payload("lib1", ScanMode::Normal).encode(), "lib1");
        assert_eq!(ScanJobPayload::decode("lib1"), payload("lib1", ScanMode::Normal));
    }

    #[test]
    fn a_reread_round_trips() {
        let reread = payload("lib1", ScanMode::Reread);

        assert_eq!(ScanJobPayload::decode(&reread.encode()), reread);
    }
}
