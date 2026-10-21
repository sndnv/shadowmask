use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use domain::error::TranscodeError;
use domain::session::{
    SessionId, SubtitleDelivery, SubtitleFileSpec, TranscodeManager, TranscodeSpec,
    TranscodeStarted,
};

use crate::hold::{Hold, NextHold};

#[derive(Default)]
struct State {
    started: Vec<TranscodeSpec>,
    subtitle_files: Vec<SubtitleFileSpec>,
    active: HashMap<SessionId, String>,
    touched: Vec<SessionId>,
    stopped: Vec<SessionId>,
    reaped: usize,
}

#[derive(Clone, Default)]
pub struct MockTranscodeManager {
    state: Arc<Mutex<State>>,
    fail: Arc<AtomicBool>,
    subtitles_missing: Arc<AtomicBool>,
    hold: NextHold,
    stop_hold: NextHold,
}

impl MockTranscodeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail(&self) {
        self.fail.store(true, Ordering::Relaxed);
    }

    pub fn set_subtitles_missing(&self) {
        self.subtitles_missing.store(true, Ordering::Relaxed);
    }

    pub fn hold_next_start(&self) -> Hold {
        self.hold.arm()
    }

    pub fn hold_next_stop(&self) -> Hold {
        self.stop_hold.arm()
    }

    pub fn started(&self) -> Vec<TranscodeSpec> {
        self.state.lock().unwrap().started.clone()
    }

    pub fn subtitle_files(&self) -> Vec<SubtitleFileSpec> {
        self.state.lock().unwrap().subtitle_files.clone()
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
        self.hold.pass().await;
        if self.fail.load(Ordering::Relaxed) {
            return Err(TranscodeError::Spawn("mock transcode failure".to_owned()));
        }
        let output_dir = format!("/mock/cache/{}/{}", spec.session.0, spec.generation.0);
        let session = spec.session.clone();
        let origin_ms = spec.seek_ms.unwrap_or(0);
        let subtitles =
            spec.soft_subtitle.is_some() && !self.subtitles_missing.load(Ordering::Relaxed);
        let subtitle_file = spec
            .soft_subtitle
            .as_ref()
            .filter(|soft| subtitles && soft.delivery == SubtitleDelivery::External)
            .map(|_| PathBuf::from(&output_dir).join("subtitles.vtt"));
        let mut state = self.state.lock().unwrap();
        state.active.insert(session.clone(), output_dir.clone());
        state.started.push(spec);
        Ok(TranscodeStarted {
            session,
            output_dir,
            origin_ms,
            sequential: true,
            subtitles,
            subtitle_file,
        })
    }

    async fn prepare_subtitle_file(
        &self,
        spec: SubtitleFileSpec,
    ) -> Result<PathBuf, TranscodeError> {
        let path = PathBuf::from(format!(
            "/mock/cache/{}/{}/subtitles.vtt",
            spec.session.0, spec.generation.0
        ));
        self.state.lock().unwrap().subtitle_files.push(spec);
        if self.subtitles_missing.load(Ordering::Relaxed) {
            return Err(TranscodeError::Spawn("mock subtitle failure".to_owned()));
        }
        Ok(path)
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
        self.stop_hold.pass().await;
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
    use domain::catalog::VersionId;
    use domain::session::{SegmentContainer, SoftSubtitle, SoftSubtitleSource, StreamGeneration};

    fn spec(id: &str) -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId(id.to_owned()),
            generation: StreamGeneration(1),
            version: VersionId("ver-1".to_owned()),
            input_path: "/media/m1.mkv".to_owned(),
            duration_ms: 100_000,
            keyframes_us: None,
            copy: false,
            container: SegmentContainer::MpegTs,
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle: None,
            soft_subtitle: None,
            convert_audio: false,
            audio_channels: None,
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
        assert!(matches!(manager.touch(&missing).await.unwrap_err(), TranscodeError::NotFound));
        assert!(matches!(manager.stop(&missing).await.unwrap_err(), TranscodeError::NotFound));
    }

    #[tokio::test]
    async fn a_held_start_waits_for_its_release_and_only_the_next_one_is_held() {
        let manager = MockTranscodeManager::new();
        let hold = manager.hold_next_start();
        let (held, ()) = tokio::join!(manager.start(spec("s1")), async {
            hold.reached().await;
            assert!(manager.started().is_empty());
            hold.release();
        });
        assert!(held.is_ok());
        manager.start(spec("s2")).await.unwrap();
        assert_eq!(manager.started().len(), 2);
    }

    fn soft(delivery: SubtitleDelivery) -> SoftSubtitle {
        SoftSubtitle { source: SoftSubtitleSource::Embedded(3), offset_ms: 0, delivery }
    }

    #[tokio::test]
    async fn a_soft_subtitle_is_written_unless_it_is_set_missing() {
        let manager = MockTranscodeManager::new();
        let with_subtitle =
            |delivery| TranscodeSpec { soft_subtitle: Some(soft(delivery)), ..spec("s1") };
        assert!(!manager.start(spec("s1")).await.unwrap().subtitles);
        let rendition = manager.start(with_subtitle(SubtitleDelivery::HlsVtt)).await.unwrap();
        assert!(rendition.subtitles);
        assert_eq!(rendition.subtitle_file, None);
        let file = manager.start(with_subtitle(SubtitleDelivery::External)).await.unwrap();
        assert_eq!(file.subtitle_file, Some(PathBuf::from("/mock/cache/s1/1/subtitles.vtt")));
        manager.set_subtitles_missing();
        let missing = manager.start(with_subtitle(SubtitleDelivery::External)).await.unwrap();
        assert!(!missing.subtitles);
        assert_eq!(missing.subtitle_file, None);
    }

    #[tokio::test]
    async fn a_subtitle_file_is_recorded_and_fails_when_set_missing() {
        let manager = MockTranscodeManager::new();
        let request = SubtitleFileSpec {
            session: SessionId("s1".to_owned()),
            generation: StreamGeneration(2),
            version: VersionId("ver-1".to_owned()),
            input_path: "/media/m1.mkv".to_owned(),
            subtitle: soft(SubtitleDelivery::External),
        };
        let path = manager.prepare_subtitle_file(request.clone()).await.unwrap();
        assert_eq!(path, PathBuf::from("/mock/cache/s1/2/subtitles.vtt"));
        assert_eq!(manager.subtitle_files(), vec![request.clone()]);
        manager.set_subtitles_missing();
        assert!(manager.prepare_subtitle_file(request).await.is_err());
    }

    #[tokio::test]
    async fn start_failure_maps_to_spawn_error() {
        let manager = MockTranscodeManager::new();
        manager.set_fail();
        assert!(matches!(manager.start(spec("s1")).await.unwrap_err(), TranscodeError::Spawn(_)));
    }
}
