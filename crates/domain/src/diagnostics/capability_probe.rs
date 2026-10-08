use std::future::Future;

use crate::diagnostics::CapabilityReport;

pub trait CapabilityProbe: Send + Sync {
    fn probe(&self) -> impl Future<Output = CapabilityReport> + Send;
}
