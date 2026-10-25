use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::common::{Page, PageRequest, paginate};
use domain::diagnostics::{Report, ReportId, ReportSummary};
use domain::error::RepositoryError;
use domain::repository::ReportRepository;
use jiff::Timestamp;

#[derive(Clone, Default)]
pub struct MockReportRepo {
    reports: Arc<Mutex<HashMap<ReportId, Report>>>,
    fail: Arc<AtomicBool>,
}

impl MockReportRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&self, report: Report) {
        self.reports.lock().unwrap().insert(report.summary.id.clone(), report);
    }

    pub fn stored(&self) -> Vec<Report> {
        self.reports.lock().unwrap().values().cloned().collect()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    fn guard(&self) -> Result<(), RepositoryError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(RepositoryError::Backend("mock report store failure".to_owned()));
        }
        Ok(())
    }
}

fn matches(summary: &ReportSummary, needle: &str) -> bool {
    [&summary.player.0, &summary.user.0, &summary.app, &summary.platform, &summary.app_version]
        .iter()
        .any(|field| field.to_lowercase().contains(needle))
}

impl ReportRepository for MockReportRepo {
    async fn insert(&self, report: Report) -> Result<(), RepositoryError> {
        self.guard()?;
        self.seed(report);
        Ok(())
    }

    async fn list(
        &self,
        since: Timestamp,
        needle: Option<&str>,
        page: PageRequest,
    ) -> Result<Page<ReportSummary>, RepositoryError> {
        self.guard()?;
        let needle = needle.map(str::to_lowercase);
        let mut kept: Vec<ReportSummary> = self
            .reports
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.summary.created_at >= since)
            .filter(|r| needle.as_deref().is_none_or(|n| matches(&r.summary, n)))
            .map(|r| r.summary.clone())
            .collect();
        kept.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.id.0.cmp(&b.id.0)));
        Ok(paginate(&kept, page))
    }

    async fn get(
        &self,
        id: &ReportId,
        since: Timestamp,
    ) -> Result<Option<Report>, RepositoryError> {
        self.guard()?;
        let reports = self.reports.lock().unwrap();
        Ok(reports.get(id).filter(|r| r.summary.created_at >= since).cloned())
    }

    async fn delete(&self, id: &ReportId) -> Result<bool, RepositoryError> {
        self.guard()?;
        Ok(self.reports.lock().unwrap().remove(id).is_some())
    }

    async fn delete_before(&self, cutoff: Timestamp) -> Result<u64, RepositoryError> {
        self.guard()?;
        let mut reports = self.reports.lock().unwrap();
        let before = reports.len();
        reports.retain(|_, r| r.summary.created_at >= cutoff);
        Ok((before - reports.len()) as u64)
    }
}
