mod discovered_file;
mod duplicate_candidate;
#[allow(clippy::module_inception)]
mod library;
mod scan_report;
mod scan_state;
mod skip_reason;
mod skipped_file;
mod source_walker;
mod unmatched_file;
mod walked_entry;
mod watch_plan;
mod watcher_trigger;

pub use discovered_file::DiscoveredFile;
pub use duplicate_candidate::{DuplicateCandidate, DuplicateCandidateId};
pub use library::{Library, LibraryId, LibraryKind, WatcherStrategy};
pub use scan_report::ScanReport;
pub use scan_state::{ScanState, ScanStatus};
pub use skip_reason::SkipReason;
pub use skipped_file::SkippedFile;
pub use source_walker::SourceWalker;
pub use unmatched_file::{MatchCandidate, UnmatchedFile, UnmatchedFileId};
pub use walked_entry::WalkedEntry;
pub use watch_plan::WatchPlan;
pub use watcher_trigger::plan_for;
