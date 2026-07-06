mod cron;
mod debouncer;
mod dedup;
mod external_subtitles;
mod matcher;
mod parse;
mod scanner;
mod service;

pub use cron::next_fire_after;
pub use debouncer::Debouncer;
pub use dedup::find_duplicates;
pub use domain::text::normalize_title;
pub use external_subtitles::discover_subtitles;
pub use matcher::Matcher;
pub use parse::{confidence, parse_filename};
pub use scanner::Scanner;
pub use service::LibraryServiceImpl;
