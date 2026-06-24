use std::future::Future;

use crate::error::TranscodeError;
use crate::session::{SessionId, TranscodeSpec, TranscodeStarted};

pub trait TranscodeManager {
    fn start(
        &self,
        spec: TranscodeSpec,
    ) -> impl Future<Output = Result<TranscodeStarted, TranscodeError>> + Send;
    fn touch(&self, session: &SessionId)
    -> impl Future<Output = Result<(), TranscodeError>> + Send;
    fn stop(&self, session: &SessionId) -> impl Future<Output = Result<(), TranscodeError>> + Send;
    fn reap_idle(&self) -> impl Future<Output = usize> + Send;
}
