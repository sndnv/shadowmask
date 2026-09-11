use std::collections::HashSet;

use domain::error::ProbeError;
use domain::media::{MediaProbe, ProbeResult};

#[derive(Clone, Default)]
pub struct MockMediaProbe {
    failing: HashSet<String>,
    duration_ms: u64,
}

impl MockMediaProbe {
    pub fn new() -> Self {
        Self { failing: HashSet::new(), duration_ms: 1000 }
    }

    pub fn failing_on(mut self, path: &str) -> Self {
        self.failing.insert(path.to_owned());
        self
    }
}

impl MediaProbe for MockMediaProbe {
    async fn probe(&self, path: &str) -> Result<ProbeResult, ProbeError> {
        if self.failing.contains(path) {
            return Err(ProbeError::Backend("mock probe failure".to_owned()));
        }
        Ok(ProbeResult {
            duration_ms: self.duration_ms,
            video: Vec::new(),
            audio: Vec::new(),
            subtitles: Vec::new(),
            chapters: Vec::new(),
        })
    }
}
