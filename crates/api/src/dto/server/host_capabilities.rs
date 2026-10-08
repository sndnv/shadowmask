use domain::diagnostics::HostReport;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostCapabilities {
    pub cpu: Option<String>,
    pub cores: Option<usize>,
    pub threads: usize,
}

impl From<HostReport> for HostCapabilities {
    fn from(host: HostReport) -> Self {
        Self { cpu: host.cpu, cores: host.cores, threads: host.threads }
    }
}
