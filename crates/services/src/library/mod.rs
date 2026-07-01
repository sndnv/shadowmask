mod cron;
mod debouncer;
mod dedup;
mod external_subtitles;
mod matcher;
mod parse;
mod scanner;

pub use cron::next_fire_after;
pub use debouncer::Debouncer;
pub use dedup::find_duplicates;
pub use external_subtitles::discover_subtitles;
pub use matcher::Matcher;
pub use parse::{confidence, normalize_title, parse_filename};
pub use scanner::Scanner;
