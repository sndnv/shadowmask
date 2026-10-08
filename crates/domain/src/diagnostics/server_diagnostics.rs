use crate::diagnostics::DiagnosticsStatus;

pub trait ServerDiagnostics: Send + Sync {
    fn status(&self) -> DiagnosticsStatus;

    fn recheck(&self);
}
