use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::error::TranscodeError;
use domain::session::{SessionId, TranscodeManager, TranscodeSpec, TranscodeStarted};

#[derive(Default)]
struct State {
    started: Vec<TranscodeSpec>,
    active: HashMap<SessionId, String>,
    touched: Vec<SessionId>,
    stopped: Vec<SessionId>,
    reaped: usize,
}

#[derive(Clone, Default)]
pub struct MockTranscodeManager {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
}

impl MockTranscodeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    pub fn started(&self) -> Vec<TranscodeSpec> {
        self.state.lock().unwrap().started.clone()
    }

    pub fn stopped(&self) -> Vec<SessionId> {
        self.state.lock().unwrap().stopped.clone()
    }

    pub fn touched(&self) -> Vec<SessionId> {
        self.state.lock().unwrap().touched.clone()
    }

    pub fn reaped(&self) -> usize {
        self.state.lock().unwrap().reaped
    }
}

impl TranscodeManager for MockTranscodeManager {
    async fn start(&self, spec: TranscodeSpec) -> Result<TranscodeStarted, TranscodeError> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(TranscodeError::Spawn("mock transcode failure".to_owned()));
        }
        let output_dir = format!("/mock/cache/{}/{}", spec.session.0, spec.generation.0);
        let session = spec.session.clone();
        let origin_ms = spec.seek_ms.unwrap_or(0);
        let mut state = self.state.lock().unwrap();
        state.active.insert(session.clone(), output_dir.clone());
        state.started.push(spec);
        Ok(TranscodeStarted {
            session,
            output_dir,
            origin_ms,
            sequential: false,
        })
    }

    async fn touch(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut state = self.state.lock().unwrap();
        if !state.active.contains_key(session) {
            return Err(TranscodeError::NotFound);
        }
        state.touched.push(session.clone());
        Ok(())
    }

    async fn stop(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut state = self.state.lock().unwrap();
        if state.active.remove(session).is_none() {
            return Err(TranscodeError::NotFound);
        }
        state.stopped.push(session.clone());
        Ok(())
    }

    async fn reap_idle(&self) -> usize {
        let mut state = self.state.lock().unwrap();
        state.reaped += 1;
        state.active.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::session::{SegmentContainer, StreamGeneration};

    fn spec(id: &str) -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId(id.to_owned()),
            generation: StreamGeneration(1),
            input_path: "/media/m1.mkv".to_owned(),
            duration_ms: 100_000,
            copy: false,
            container: SegmentContainer::MpegTs,
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle_path: None,
            soft_subtitle: None,
            downmix_stereo: false,
            source_hdr: None,
        }
    }

    #[tokio::test]
    async fn start_touch_stop_reap() {
        let manager = MockTranscodeManager::new();
        let session = SessionId("s1".to_owned());
        let started = manager.start(spec("s1")).await.unwrap();
        assert_eq!(started.session, session);
        assert_eq!(manager.started().len(), 1);

        manager.touch(&session).await.unwrap();
        assert_eq!(manager.touched(), vec![session.clone()]);

        assert_eq!(manager.reap_idle().await, 1);
        assert_eq!(manager.reaped(), 1);

        manager.stop(&session).await.unwrap();
        assert_eq!(manager.stopped(), vec![session.clone()]);
        assert_eq!(manager.reap_idle().await, 0);
    }

    #[tokio::test]
    async fn touch_and_stop_unknown_error() {
        let manager = MockTranscodeManager::new();
        let missing = SessionId("nope".to_owned());
        assert!(matches!(
            manager.touch(&missing).await.unwrap_err(),
            TranscodeError::NotFound
        ));
        assert!(matches!(
            manager.stop(&missing).await.unwrap_err(),
            TranscodeError::NotFound
        ));
    }

    #[tokio::test]
    async fn start_failure_maps_to_spawn_error() {
        let manager = MockTranscodeManager::new();
        manager.set_fail();
        assert!(matches!(
            manager.start(spec("s1")).await.unwrap_err(),
            TranscodeError::Spawn(_)
        ));
    }
}
