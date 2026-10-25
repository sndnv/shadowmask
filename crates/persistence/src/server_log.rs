use std::path::{Path, PathBuf};

use domain::common::LogLevel;
use domain::diagnostics::{
    ServerLogCursor, ServerLogFrom, ServerLogPage, ServerLogRequest, ServerLogStore,
};
use domain::error::ServerLogError;
use jiff::Timestamp;
use jiff::civil::DateTime;

pub const SERVER_LOG_PREFIX: &str = "server.log";

struct Entry {
    level: Option<LogLevel>,
    text: String,
}

#[derive(Debug, Clone)]
pub struct FsServerLogStore {
    root: PathBuf,
}

impl FsServerLogStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    async fn hours(&self) -> Result<Vec<(DateTime, PathBuf)>, ServerLogError> {
        let mut dir = match tokio::fs::read_dir(&self.root).await {
            Ok(dir) => dir,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(backend(e)),
        };
        let mut hours = Vec::new();
        while let Some(item) = dir.next_entry().await.map_err(backend)? {
            let name = item.file_name();
            let hour = name
                .to_str()
                .and_then(|name| name.strip_prefix(SERVER_LOG_PREFIX)?.strip_prefix('.'))
                .and_then(ServerLogCursor::parse_hour);
            if let Some(hour) = hour {
                hours.push((hour, item.path()));
            }
        }
        hours.sort();
        Ok(hours)
    }

    async fn backward(
        &self,
        hours: &[(DateTime, PathBuf)],
        from: Option<ServerLogCursor>,
        request: ServerLogRequest,
    ) -> Result<ServerLogPage, ServerLogError> {
        let mut page = ServerLogPage::default();
        let mut found = Vec::new();
        for (position, (hour, path)) in hours.iter().enumerate().rev() {
            if from.is_some_and(|cursor| *hour > cursor.hour) {
                continue;
            }
            let entries = entries_of(path).await?;
            let end = match from {
                Some(cursor) if cursor.hour == *hour => within(cursor, entries.len())?,
                _ => entries.len(),
            };
            if from.is_none() && page.after.is_none() {
                page.after = Some(ServerLogCursor { hour: *hour, entry: entries.len() as u64 });
            }
            for (index, entry) in entries.into_iter().take(end).enumerate().rev() {
                if !keeps(&entry, request.min_level) {
                    continue;
                }
                found.push(entry.text);
                if found.len() == request.limit {
                    if index > 0 || position > 0 {
                        page.before = Some(ServerLogCursor { hour: *hour, entry: index as u64 });
                    }
                    found.reverse();
                    page.entries = found;
                    return Ok(page);
                }
            }
        }
        found.reverse();
        page.entries = found;
        Ok(page)
    }

    async fn forward(
        &self,
        hours: &[(DateTime, PathBuf)],
        from: ServerLogCursor,
        request: ServerLogRequest,
    ) -> Result<ServerLogPage, ServerLogError> {
        let mut page = ServerLogPage { after: Some(from), ..ServerLogPage::default() };
        for (hour, path) in hours.iter().filter(|(hour, _)| *hour >= from.hour) {
            let entries = entries_of(path).await?;
            let count = entries.len();
            let start = if *hour == from.hour { within(from, count)? } else { 0 };
            for (index, entry) in entries.into_iter().enumerate().skip(start) {
                if page.entries.len() == request.limit {
                    page.more = true;
                    return Ok(page);
                }
                if keeps(&entry, request.min_level) {
                    page.entries.push(entry.text);
                }
                page.after = Some(ServerLogCursor { hour: *hour, entry: index as u64 + 1 });
            }
            page.after = Some(ServerLogCursor { hour: *hour, entry: count as u64 });
        }
        Ok(page)
    }
}

impl ServerLogStore for FsServerLogStore {
    async fn read(&self, request: ServerLogRequest) -> Result<ServerLogPage, ServerLogError> {
        let hours = self.hours().await?;
        match request.from {
            ServerLogFrom::Newest => self.backward(&hours, None, request).await,
            ServerLogFrom::Before(cursor) => self.backward(&hours, Some(cursor), request).await,
            ServerLogFrom::After(cursor) => self.forward(&hours, cursor, request).await,
        }
    }
}

fn backend(err: std::io::Error) -> ServerLogError {
    ServerLogError::Backend(err.to_string())
}

fn within(cursor: ServerLogCursor, count: usize) -> Result<usize, ServerLogError> {
    usize::try_from(cursor.entry)
        .ok()
        .filter(|entry| *entry <= count)
        .ok_or(ServerLogError::InvalidCursor)
}

fn keeps(entry: &Entry, min_level: Option<LogLevel>) -> bool {
    min_level.is_none_or(|min| entry.level.is_none_or(|level| level >= min))
}

async fn entries_of(path: &Path) -> Result<Vec<Entry>, ServerLogError> {
    let bytes = tokio::fs::read(path).await.map_err(backend)?;
    let text = String::from_utf8_lossy(&bytes);
    let complete = &text[..text.rfind('\n').map_or(0, |end| end + 1)];
    let mut entries: Vec<Entry> = Vec::new();
    for line in complete.lines() {
        let level = header_level(line);
        match entries.last_mut() {
            Some(entry) if level.is_none() => {
                entry.text.push('\n');
                entry.text.push_str(line);
            }
            _ => entries.push(Entry { level, text: line.to_owned() }),
        }
    }
    Ok(entries)
}

fn header_level(line: &str) -> Option<LogLevel> {
    let mut words = line.split_whitespace();
    words.next()?.parse::<Timestamp>().ok()?;
    LogLevel::parse(words.next()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn hour(h: i8) -> DateTime {
        date(2026, 10, 24).at(h, 0, 0, 0)
    }

    fn cursor(h: i8, entry: u64) -> ServerLogCursor {
        ServerLogCursor { hour: hour(h), entry }
    }

    fn line(level: &str, message: &str) -> String {
        format!("2026-10-24T09:00:00.000001Z {level:>5} server::test: {message}\n")
    }

    fn file(dir: &Path, h: i8) -> PathBuf {
        dir.join(format!("{SERVER_LOG_PREFIX}.2026-10-24-{h:02}"))
    }

    fn write(dir: &Path, h: i8, content: &str) {
        std::fs::write(file(dir, h), content).unwrap();
    }

    fn append(dir: &Path, h: i8, content: &str) {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new().append(true).open(file(dir, h)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn lines(level: &str, names: &[&str]) -> String {
        names.iter().map(|name| line(level, name)).collect()
    }

    fn messages(page: &ServerLogPage) -> Vec<String> {
        page.entries.iter().map(|e| e.rsplit(": ").next().unwrap().to_owned()).collect()
    }

    fn request(from: ServerLogFrom, limit: usize) -> ServerLogRequest {
        ServerLogRequest { from, limit, min_level: None }
    }

    fn two_hours() -> (tempfile::TempDir, FsServerLogStore) {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), 8, &lines("INFO", &["a", "b", "c"]));
        write(dir.path(), 9, &lines("INFO", &["d", "e"]));
        let store = FsServerLogStore::new(dir.path());
        (dir, store)
    }

    #[tokio::test]
    async fn the_newest_entries_come_oldest_first_across_hours() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::Newest, 4)).await.unwrap();

        assert_eq!(messages(&page), ["b", "c", "d", "e"]);
        assert_eq!(page.before, Some(cursor(8, 1)));
        assert_eq!(page.after, Some(cursor(9, 2)));
        assert!(!page.more);
    }

    #[tokio::test]
    async fn reading_back_continues_without_a_gap_and_ends_at_the_oldest_kept() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::Before(cursor(8, 1)), 4)).await.unwrap();

        assert_eq!(messages(&page), ["a"]);
        assert_eq!(page.before, None);
        assert_eq!(page.after, None);
    }

    #[tokio::test]
    async fn reading_back_from_the_start_of_an_hour_goes_to_the_hour_before() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::Before(cursor(9, 0)), 2)).await.unwrap();

        assert_eq!(messages(&page), ["b", "c"]);
        assert_eq!(page.before, Some(cursor(8, 1)));
    }

    #[tokio::test]
    async fn a_page_that_ends_on_the_first_entry_kept_has_nothing_before_it() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::Newest, 5)).await.unwrap();

        assert_eq!(messages(&page), ["a", "b", "c", "d", "e"]);
        assert_eq!(page.before, None);
    }

    #[tokio::test]
    async fn polling_picks_up_what_was_appended_including_a_new_hour() {
        let (dir, store) = two_hours();
        let after = store.read(request(ServerLogFrom::Newest, 10)).await.unwrap().after.unwrap();
        append(dir.path(), 9, &line("WARN", "f"));
        write(dir.path(), 10, &line("INFO", "g"));

        let page = store.read(request(ServerLogFrom::After(after), 10)).await.unwrap();

        assert_eq!(messages(&page), ["f", "g"]);
        assert_eq!(page.after, Some(cursor(10, 1)));
        assert!(!page.more);
    }

    #[tokio::test]
    async fn polling_with_nothing_new_keeps_its_place() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::After(cursor(9, 2)), 10)).await.unwrap();

        assert!(page.entries.is_empty());
        assert_eq!(page.after, Some(cursor(9, 2)));
    }

    #[tokio::test]
    async fn polling_stops_at_the_limit_and_says_there_is_more() {
        let (_dir, store) = two_hours();

        let page = store.read(request(ServerLogFrom::After(cursor(8, 0)), 2)).await.unwrap();

        assert_eq!(messages(&page), ["a", "b"]);
        assert_eq!(page.after, Some(cursor(8, 2)));
        assert!(page.more);
    }

    #[tokio::test]
    async fn a_line_still_being_written_waits_for_its_end() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), 9, &format!("{}2026-10-24T09:00:00Z  INFO x: half", line("INFO", "a")));
        let store = FsServerLogStore::new(dir.path());

        let page = store.read(request(ServerLogFrom::Newest, 10)).await.unwrap();
        assert_eq!(messages(&page), ["a"]);
        assert_eq!(page.after, Some(cursor(9, 1)));

        append(dir.path(), 9, "\n");
        let next = store.read(request(ServerLogFrom::After(cursor(9, 1)), 10)).await.unwrap();
        assert_eq!(messages(&next), ["half"]);
    }

    #[tokio::test]
    async fn a_message_over_several_lines_is_one_entry() {
        let dir = tempfile::tempdir().unwrap();
        let content = format!(
            "orphan from the hour before\n{}  frame=12\n  speed=0.9x\n{}",
            line("WARN", "ffmpeg said"),
            line("INFO", "next")
        );
        write(dir.path(), 9, &content);
        let store = FsServerLogStore::new(dir.path());

        let page = store.read(request(ServerLogFrom::Newest, 10)).await.unwrap();

        assert_eq!(page.entries.len(), 3);
        assert_eq!(page.entries[0], "orphan from the hour before");
        assert!(page.entries[1].ends_with("ffmpeg said\n  frame=12\n  speed=0.9x"));
        assert_eq!(page.after, Some(cursor(9, 3)));
    }

    #[tokio::test]
    async fn the_level_filter_skips_lower_entries_and_still_fills_the_page() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), 8, &format!("{}{}", line("WARN", "w1"), line("DEBUG", "d1")));
        write(dir.path(), 9, &format!("{}{}", line("ERROR", "e1"), line("DEBUG", "d2")));
        let store = FsServerLogStore::new(dir.path());
        let warn = |from| ServerLogRequest { from, limit: 2, min_level: Some(LogLevel::Warn) };

        let page = store.read(warn(ServerLogFrom::Newest)).await.unwrap();
        assert_eq!(messages(&page), ["w1", "e1"]);
        assert_eq!(page.after, Some(cursor(9, 2)));

        let polled = store.read(warn(ServerLogFrom::After(cursor(8, 0)))).await.unwrap();
        assert_eq!(messages(&polled), ["w1", "e1"]);
        assert_eq!(polled.after, Some(cursor(9, 1)));
        assert!(polled.more);
    }

    #[tokio::test]
    async fn a_cursor_past_the_end_of_its_hour_is_refused() {
        let (_dir, store) = two_hours();

        for from in [ServerLogFrom::Before(cursor(9, 3)), ServerLogFrom::After(cursor(9, 3))] {
            let err = store.read(request(from, 10)).await.unwrap_err();
            assert!(matches!(err, ServerLogError::InvalidCursor), "{from:?}");
        }
    }

    #[tokio::test]
    async fn an_hour_retention_removed_ends_reading_back_and_polls_from_the_oldest_kept() {
        let (_dir, store) = two_hours();

        let back = store.read(request(ServerLogFrom::Before(cursor(7, 5)), 10)).await.unwrap();
        assert!(back.entries.is_empty());
        assert_eq!(back.before, None);

        let polled = store.read(request(ServerLogFrom::After(cursor(7, 5)), 10)).await.unwrap();
        assert_eq!(messages(&polled), ["a", "b", "c", "d", "e"]);
    }

    #[tokio::test]
    async fn no_directory_has_no_entries() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsServerLogStore::new(dir.path().join("missing"));

        let page = store.read(request(ServerLogFrom::Newest, 10)).await.unwrap();

        assert_eq!(page, ServerLogPage::default());
    }

    #[tokio::test]
    async fn files_that_are_not_hourly_logs_are_ignored() {
        let (dir, store) = two_hours();
        std::fs::write(dir.path().join("notes.txt"), line("INFO", "x")).unwrap();
        std::fs::write(dir.path().join("server.log.today"), line("INFO", "x")).unwrap();
        std::fs::write(dir.path().join("server.logs.2026-10-24-10"), line("INFO", "x")).unwrap();

        let page = store.read(request(ServerLogFrom::Newest, 10)).await.unwrap();

        assert_eq!(messages(&page), ["a", "b", "c", "d", "e"]);
    }

    #[tokio::test]
    async fn an_unreadable_directory_or_file_is_a_backend_error() {
        let dir = tempfile::tempdir().unwrap();
        let not_a_dir = dir.path().join("plain");
        std::fs::write(&not_a_dir, "x").unwrap();
        std::fs::create_dir(file(dir.path(), 9)).unwrap();

        for root in [not_a_dir, dir.path().to_path_buf()] {
            let err = FsServerLogStore::new(root)
                .read(request(ServerLogFrom::Newest, 10))
                .await
                .unwrap_err();
            assert!(matches!(err, ServerLogError::Backend(_)));
        }
    }
}
