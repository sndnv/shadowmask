use std::io::Write as _;
use std::path::{Path, PathBuf};

use domain::error::JobLogError;
use domain::job::{JobId, JobLogLevel, JobLogStore};
use jiff::Timestamp;
use tokio::io::AsyncWriteExt;

fn format_line(at: Timestamp, level: JobLogLevel, message: &str) -> String {
    format!("{at} {} {}\n", level.as_str(), message.replace('\n', " "))
}

#[derive(Debug, Clone)]
pub struct FsJobLogStore {
    root: PathBuf,
}

impl FsJobLogStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, job: &JobId) -> Result<PathBuf, JobLogError> {
        let id = job.0.as_str();
        let safe =
            !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !safe {
            return Err(JobLogError::InvalidId);
        }
        Ok(self.root.join(format!("{id}.log")))
    }

    async fn write_line(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.root).await?;
        let mut file = tokio::fs::OpenOptions::new().create(true).append(true).open(path).await?;
        file.write_all(bytes).await
    }

    fn write_line_blocking(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(bytes)
    }

    pub fn append_blocking(
        &self,
        job: &JobId,
        at: Timestamp,
        level: JobLogLevel,
        message: &str,
    ) -> Result<(), JobLogError> {
        let path = self.path_for(job)?;
        let line = format_line(at, level, message);
        self.write_line_blocking(&path, line.as_bytes())
            .map_err(|e| JobLogError::Backend(e.to_string()))
    }
}

impl JobLogStore for FsJobLogStore {
    async fn append(
        &self,
        job: &JobId,
        at: Timestamp,
        level: JobLogLevel,
        message: &str,
    ) -> Result<(), JobLogError> {
        let path = self.path_for(job)?;
        let line = format_line(at, level, message);
        self.write_line(&path, line.as_bytes())
            .await
            .map_err(|e| JobLogError::Backend(e.to_string()))
    }

    async fn read(&self, job: &JobId, tail: Option<usize>) -> Result<Vec<String>, JobLogError> {
        let path = self.path_for(job)?;
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(JobLogError::Backend(e.to_string())),
        };
        let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
        if let Some(limit) = tail
            && lines.len() > limit
        {
            lines = lines.split_off(lines.len() - limit);
        }
        Ok(lines)
    }

    async fn wipe(&self, job: &JobId) -> Result<(), JobLogError> {
        let path = self.path_for(job)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(JobLogError::Backend(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, FsJobLogStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FsJobLogStore::new(dir.path().join("job-logs"));
        (dir, store)
    }

    #[tokio::test]
    async fn append_then_read_round_trips_in_order() {
        let (_dir, store) = store();
        let job = JobId("job-1".into());
        let at = Timestamp::UNIX_EPOCH;
        store.append(&job, at, JobLogLevel::Info, "started").await.unwrap();
        store.append(&job, at, JobLogLevel::Error, "boom\nmid\nline").await.unwrap();

        let lines = store.read(&job, None).await.unwrap();
        assert_eq!(
            lines,
            vec![
                "1970-01-01T00:00:00Z INFO started".to_owned(),
                "1970-01-01T00:00:00Z ERROR boom mid line".to_owned(),
            ]
        );
    }

    #[tokio::test]
    async fn append_blocking_writes_readable_lines() {
        let (_dir, store) = store();
        let job = JobId("job-b".into());
        store
            .append_blocking(&job, Timestamp::UNIX_EPOCH, JobLogLevel::Warn, "sync\nline")
            .unwrap();
        let lines = store.read(&job, None).await.unwrap();
        assert_eq!(lines, vec!["1970-01-01T00:00:00Z WARN sync line".to_owned()]);
    }

    #[test]
    fn append_blocking_reports_backend_error_when_root_is_a_file() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let store = FsJobLogStore::new(file.path().join("nested"));
        let err = store
            .append_blocking(&JobId("job-1".into()), Timestamp::UNIX_EPOCH, JobLogLevel::Info, "x")
            .expect_err("backend error");
        assert!(matches!(err, JobLogError::Backend(_)));
    }

    #[test]
    fn append_blocking_rejects_unsafe_job_id() {
        let (_dir, store) = store();
        assert!(matches!(
            store.append_blocking(
                &JobId("../x".into()),
                Timestamp::UNIX_EPOCH,
                JobLogLevel::Info,
                "x"
            ),
            Err(JobLogError::InvalidId)
        ));
    }

    #[tokio::test]
    async fn read_tail_returns_last_lines() {
        let (_dir, store) = store();
        let job = JobId("job-2".into());
        let at = Timestamp::UNIX_EPOCH;
        for n in 0..5 {
            store.append(&job, at, JobLogLevel::Info, &format!("line {n}")).await.unwrap();
        }

        let lines = store.read(&job, Some(2)).await.unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("line 3"));
        assert!(lines[1].ends_with("line 4"));

        let all = store.read(&job, Some(50)).await.unwrap();
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn read_missing_is_empty() {
        let (_dir, store) = store();
        let lines = store.read(&JobId("nope".into()), None).await.unwrap();
        assert!(lines.is_empty());
    }

    #[tokio::test]
    async fn wipe_removes_file_and_is_idempotent() {
        let (_dir, store) = store();
        let job = JobId("job-3".into());
        store.append(&job, Timestamp::UNIX_EPOCH, JobLogLevel::Info, "x").await.unwrap();
        assert_eq!(store.read(&job, None).await.unwrap().len(), 1);
        store.wipe(&job).await.unwrap();
        assert!(store.read(&job, None).await.unwrap().is_empty());
        store.wipe(&job).await.unwrap();
    }

    #[tokio::test]
    async fn rejects_unsafe_job_id() {
        let (_dir, store) = store();
        for bad in ["", "../escape", "a/b", "with.dot"] {
            let job = JobId(bad.into());
            assert!(matches!(
                store.append(&job, Timestamp::UNIX_EPOCH, JobLogLevel::Info, "x").await,
                Err(JobLogError::InvalidId)
            ));
            assert!(matches!(store.read(&job, None).await, Err(JobLogError::InvalidId)));
            assert!(matches!(store.wipe(&job).await, Err(JobLogError::InvalidId)));
        }
    }

    #[tokio::test]
    async fn append_reports_backend_error_when_root_is_a_file() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let store = FsJobLogStore::new(file.path().join("nested"));
        let err = store
            .append(&JobId("job-1".into()), Timestamp::UNIX_EPOCH, JobLogLevel::Info, "x")
            .await
            .expect_err("backend error");
        assert!(matches!(err, JobLogError::Backend(_)));
    }

    #[test]
    fn error_display_is_stable() {
        assert_eq!(JobLogError::InvalidId.to_string(), "invalid job id");
        assert_eq!(
            JobLogError::Backend("disk full".into()).to_string(),
            "job log backend error: disk full"
        );
    }

    #[tokio::test]
    async fn read_and_wipe_report_backend_error_on_directory_path() {
        let (_dir, store) = store();
        let job = JobId("job-4".into());
        let path = store.path_for(&job).unwrap();
        tokio::fs::create_dir_all(&path).await.unwrap();

        assert!(matches!(
            store.append(&job, Timestamp::UNIX_EPOCH, JobLogLevel::Info, "x").await,
            Err(JobLogError::Backend(_))
        ));
        assert!(matches!(store.read(&job, None).await, Err(JobLogError::Backend(_))));
        assert!(matches!(store.wipe(&job).await, Err(JobLogError::Backend(_))));
    }
}
