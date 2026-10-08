use std::sync::{Arc, Mutex};

use domain::diagnostics::{CapabilityProbe, DiagnosticsStatus, ServerDiagnostics};
use jiff::Timestamp;

pub struct CachedDiagnostics<P> {
    probe: Arc<P>,
    status: Arc<Mutex<DiagnosticsStatus>>,
}

impl<P> CachedDiagnostics<P> {
    pub fn new(probe: P) -> Self {
        Self { probe: Arc::new(probe), status: Arc::default() }
    }
}

impl<P> Clone for CachedDiagnostics<P> {
    fn clone(&self) -> Self {
        Self { probe: Arc::clone(&self.probe), status: Arc::clone(&self.status) }
    }
}

impl<P: CapabilityProbe + 'static> ServerDiagnostics for CachedDiagnostics<P> {
    fn status(&self) -> DiagnosticsStatus {
        self.status.lock().unwrap().clone()
    }

    fn recheck(&self) {
        {
            let mut status = self.status.lock().unwrap();
            if status.checking {
                return;
            }
            status.checking = true;
        }
        let probe = Arc::clone(&self.probe);
        let status = Arc::clone(&self.status);
        tokio::spawn(async move {
            let _checking = Checking(&status);
            let report = probe.probe().await;
            *status.lock().unwrap() = DiagnosticsStatus {
                checking: false,
                checked_at: Some(Timestamp::now()),
                report: Some(report),
            };
            tracing::info!("server capabilities checked");
        });
    }
}

struct Checking<'a>(&'a Mutex<DiagnosticsStatus>);

impl Drop for Checking<'_> {
    fn drop(&mut self) {
        if let Ok(mut status) = self.0.lock() {
            status.checking = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::diagnostics::{
        AccelerationMode, CapabilityReport, FfmpegReport, HardwareReport, HardwareTest, HostReport,
    };
    use tokio::sync::Notify;

    use super::*;

    fn report(threads: usize) -> CapabilityReport {
        CapabilityReport {
            host: HostReport { cpu: None, cores: None, threads },
            ffmpeg: FfmpegReport {
                version: None,
                error: None,
                hwaccels: Vec::new(),
                encoders: Vec::new(),
                filters: Vec::new(),
            },
            hardware: HardwareReport {
                mode: AccelerationMode::Off,
                device: String::new(),
                device_present: false,
                in_use: false,
                test: HardwareTest::Skipped { reason: String::new() },
            },
        }
    }

    #[derive(Default)]
    struct HeldProbe {
        release: Notify,
        runs: AtomicUsize,
    }

    impl CapabilityProbe for HeldProbe {
        async fn probe(&self) -> CapabilityReport {
            let run = self.runs.fetch_add(1, Ordering::SeqCst) + 1;
            self.release.notified().await;
            report(run)
        }
    }

    async fn settled(diagnostics: &CachedDiagnostics<HeldProbe>) -> DiagnosticsStatus {
        diagnostics.probe.release.notify_one();
        loop {
            let status = diagnostics.status();
            if !status.checking {
                return status;
            }
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test]
    async fn nothing_is_known_before_the_first_check() {
        let diagnostics = CachedDiagnostics::new(HeldProbe::default());

        assert_eq!(diagnostics.status(), DiagnosticsStatus::default());
    }

    #[tokio::test]
    async fn a_check_runs_in_the_background_and_then_reports() {
        let diagnostics = CachedDiagnostics::new(HeldProbe::default());

        diagnostics.recheck();
        let checking = diagnostics.status();
        let done = settled(&diagnostics).await;

        assert!(checking.checking, "the caller is answered before the probe finishes");
        assert_eq!(checking.report, None);
        assert!(done.checked_at.is_some());
        assert_eq!(done.report, Some(report(1)));
    }

    #[tokio::test]
    async fn a_recheck_during_a_check_starts_no_second_probe() {
        let diagnostics = CachedDiagnostics::new(HeldProbe::default());

        diagnostics.recheck();
        diagnostics.clone().recheck();
        settled(&diagnostics).await;

        assert_eq!(diagnostics.probe.runs.load(Ordering::SeqCst), 1);
    }

    struct CrashingProbe;

    impl CapabilityProbe for CrashingProbe {
        async fn probe(&self) -> CapabilityReport {
            panic!("the probe crashed")
        }
    }

    #[tokio::test]
    async fn a_check_that_crashes_is_not_left_checking() {
        let diagnostics = CachedDiagnostics::new(CrashingProbe);

        diagnostics.recheck();
        while diagnostics.status().checking {
            tokio::task::yield_now().await;
        }

        assert_eq!(diagnostics.status(), DiagnosticsStatus::default());
    }

    #[tokio::test]
    async fn the_last_report_stays_while_a_recheck_runs() {
        let diagnostics = CachedDiagnostics::new(HeldProbe::default());
        diagnostics.recheck();
        settled(&diagnostics).await;

        diagnostics.recheck();
        let rechecking = diagnostics.status();
        let done = settled(&diagnostics).await;

        assert!(rechecking.checking);
        assert_eq!(rechecking.report, Some(report(1)));
        assert_eq!(done.report, Some(report(2)));
    }
}
