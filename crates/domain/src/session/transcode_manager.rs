use std::future::Future;
use std::path::PathBuf;

use crate::error::TranscodeError;
use crate::session::{SessionId, SubtitleFileSpec, TranscodeSpec, TranscodeStarted};

pub trait TranscodeManager {
    fn start(
        &self,
        spec: TranscodeSpec,
    ) -> impl Future<Output = Result<TranscodeStarted, TranscodeError>> + Send;
    fn prepare_subtitle_file(
        &self,
        spec: SubtitleFileSpec,
    ) -> impl Future<Output = Result<PathBuf, TranscodeError>> + Send;
    fn touch(&self, session: &SessionId)
    -> impl Future<Output = Result<(), TranscodeError>> + Send;
    fn stop(&self, session: &SessionId) -> impl Future<Output = Result<(), TranscodeError>> + Send;
    fn reap_idle(&self) -> impl Future<Output = usize> + Send;
}
