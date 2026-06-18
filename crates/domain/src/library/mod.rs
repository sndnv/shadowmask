mod duplicate_candidate;
#[allow(clippy::module_inception)]
mod library;
mod scan_state;
mod unmatched_file;

pub use duplicate_candidate::{DuplicateCandidate, DuplicateCandidateId};
pub use library::{Library, LibraryId, LibraryKind, WatcherStrategy};
pub use scan_state::{ScanState, ScanStatus};
pub use unmatched_file::{MatchCandidate, UnmatchedFile, UnmatchedFileId};
