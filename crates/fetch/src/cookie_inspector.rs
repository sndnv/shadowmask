use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use domain::media::{CookieInspector, CookieVerdict, check_host_cookies};

#[derive(Debug, Clone)]
pub struct CookieFileInspector {
    path: PathBuf,
}

impl CookieFileInspector {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl CookieInspector for CookieFileInspector {
    fn verdict_for(&self, host: &str) -> CookieVerdict {
        let Ok(contents) = std::fs::read_to_string(&self.path) else {
            return CookieVerdict::NotApplicable;
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        check_host_cookies(&contents, host, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_file_is_not_applicable() {
        let inspector = CookieFileInspector::new(PathBuf::from("/no/such/shadowmask/cookies.txt"));
        assert_eq!(
            inspector.verdict_for("nebula.tv"),
            CookieVerdict::NotApplicable
        );
    }

    #[test]
    fn live_cookie_reads_as_live() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.txt");
        let far_future = i64::MAX / 2;
        std::fs::write(
            &path,
            format!(".nebula.tv\tTRUE\t/\tTRUE\t{far_future}\ttoken\tvalue"),
        )
        .unwrap();
        let inspector = CookieFileInspector::new(path);
        assert_eq!(inspector.verdict_for("nebula.tv"), CookieVerdict::Live);
    }

    #[test]
    fn expired_cookie_reads_as_expired() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.txt");
        std::fs::write(&path, ".nebula.tv\tTRUE\t/\tTRUE\t1\ttoken\tvalue").unwrap();
        let inspector = CookieFileInspector::new(path);
        assert_eq!(inspector.verdict_for("nebula.tv"), CookieVerdict::Expired);
    }
}
