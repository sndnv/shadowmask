use std::collections::HashMap;
use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp};
use tokio::sync::Mutex;

use domain::error::TranscodeError;
use domain::session::{SessionId, TranscodeManager, TranscodeSpec, TranscodeStarted};

use crate::transcode::args::build_hls_args;
use crate::transcode::process::{ProcessSpawner, TokioProcessSpawner, TranscodeChild};

const DEFAULT_BINARY: &str = "ffmpeg";
const DEFAULT_IDLE_TIMEOUT: SignedDuration = SignedDuration::from_secs(30);

pub struct FfmpegTranscodeManager<S: ProcessSpawner = TokioProcessSpawner> {
    binary: String,
    cache_root: PathBuf,
    idle_timeout: SignedDuration,
    spawner: S,
    sessions: Mutex<HashMap<SessionId, TranscodeSession<S::Child>>>,
}

struct TranscodeSession<C: TranscodeChild> {
    child: C,
    output_dir: PathBuf,
    last_active: Timestamp,
}

impl<C: TranscodeChild> Drop for TranscodeSession<C> {
    fn drop(&mut self) {
        self.child.kill();
        let _ = std::fs::remove_dir_all(&self.output_dir);
    }
}

impl FfmpegTranscodeManager {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self::with_spawner(cache_root, TokioProcessSpawner)
    }
}

impl<S: ProcessSpawner> FfmpegTranscodeManager<S> {
    pub fn with_spawner(cache_root: impl Into<PathBuf>, spawner: S) -> Self {
        Self {
            binary: DEFAULT_BINARY.to_owned(),
            cache_root: cache_root.into(),
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
            spawner,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_binary(mut self, binary: impl Into<String>) -> Self {
        self.binary = binary.into();
        self
    }

    pub fn with_idle_timeout(mut self, idle_timeout: SignedDuration) -> Self {
        self.idle_timeout = idle_timeout;
        self
    }

    async fn reap_at(&self, now: Timestamp) -> usize {
        let mut sessions = self.sessions.lock().await;
        let before = sessions.len();
        let timeout = self.idle_timeout;
        sessions.retain(|_, s| !is_expired(now, s.last_active, timeout) && !s.child.has_exited());
        before - sessions.len()
    }
}

impl<S: ProcessSpawner> TranscodeManager for FfmpegTranscodeManager<S> {
    async fn start(&self, spec: TranscodeSpec) -> Result<TranscodeStarted, TranscodeError> {
        let output_dir = self.cache_root.join(&spec.session.0);
        let args = build_hls_args(&spec, &output_dir);
        let mut sessions = self.sessions.lock().await;
        sessions.remove(&spec.session);
        std::fs::create_dir_all(output_dir.join(crate::hls::VARIANT))
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let child = self
            .spawner
            .spawn(&self.binary, &args)
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        sessions.insert(
            spec.session.clone(),
            TranscodeSession {
                child,
                output_dir: output_dir.clone(),
                last_active: Timestamp::now(),
            },
        );
        Ok(TranscodeStarted {
            session: spec.session,
            output_dir: output_dir.to_string_lossy().into_owned(),
        })
    }

    async fn touch(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut sessions = self.sessions.lock().await;
        let entry = sessions.get_mut(session).ok_or(TranscodeError::NotFound)?;
        entry.last_active = Timestamp::now();
        Ok(())
    }

    async fn stop(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut sessions = self.sessions.lock().await;
        sessions
            .remove(session)
            .map(|_| ())
            .ok_or(TranscodeError::NotFound)
    }

    async fn reap_idle(&self) -> usize {
        self.reap_at(Timestamp::now()).await
    }
}

fn is_expired(now: Timestamp, last_active: Timestamp, timeout: SignedDuration) -> bool {
    now.duration_since(last_active) > timeout
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Clone)]
    struct Spawn {
        program: String,
        killed: Arc<AtomicBool>,
        exited: Arc<AtomicBool>,
    }

    #[derive(Clone, Default)]
    struct MockSpawner {
        fail: bool,
        spawns: Arc<std::sync::Mutex<Vec<Spawn>>>,
    }

    struct MockChild {
        killed: Arc<AtomicBool>,
        exited: Arc<AtomicBool>,
    }

    impl ProcessSpawner for MockSpawner {
        type Child = MockChild;

        fn spawn(&self, program: &str, _args: &[String]) -> std::io::Result<MockChild> {
            if self.fail {
                return Err(std::io::Error::other("spawn failed"));
            }
            let killed = Arc::new(AtomicBool::new(false));
            let exited = Arc::new(AtomicBool::new(false));
            self.spawns.lock().unwrap().push(Spawn {
                program: program.to_owned(),
                killed: killed.clone(),
                exited: exited.clone(),
            });
            Ok(MockChild { killed, exited })
        }
    }

    impl TranscodeChild for MockChild {
        fn kill(&mut self) {
            self.killed.store(true, Ordering::SeqCst);
        }

        fn has_exited(&mut self) -> bool {
            self.exited.load(Ordering::SeqCst)
        }
    }

    fn spec(id: &str) -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId(id.to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            seek_ms: None,
            audio_track: None,
            max_height: None,
            max_bitrate: None,
            burn_subtitle_path: None,
        }
    }

    fn manager(spawner: MockSpawner) -> (tempfile::TempDir, FfmpegTranscodeManager<MockSpawner>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = FfmpegTranscodeManager::with_spawner(dir.path(), spawner);
        (dir, mgr)
    }

    #[tokio::test]
    async fn start_spawns_and_creates_output_dir() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        let started = mgr.start(spec("s1")).await.expect("start");
        assert!(PathBuf::from(&started.output_dir).is_dir());
        assert_eq!(started.session, SessionId("s1".to_owned()));
        let spawns = spawner.spawns.lock().unwrap();
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].program, "ffmpeg");
    }

    #[tokio::test]
    async fn start_uses_configured_binary() {
        let spawner = MockSpawner::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = FfmpegTranscodeManager::with_spawner(dir.path(), spawner.clone())
            .with_binary("custom-ffmpeg");
        mgr.start(spec("s1")).await.expect("start");
        assert_eq!(spawner.spawns.lock().unwrap()[0].program, "custom-ffmpeg");
    }

    #[tokio::test]
    async fn start_spawn_failure_maps_to_error() {
        let spawner = MockSpawner {
            fail: true,
            ..Default::default()
        };
        let (_dir, mgr) = manager(spawner);
        let err = mgr.start(spec("s1")).await.unwrap_err();
        assert!(matches!(err, TranscodeError::Spawn(_)));
    }

    #[tokio::test]
    async fn start_create_dir_failure_maps_to_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let mgr = FfmpegTranscodeManager::with_spawner(file.path(), MockSpawner::default());
        let err = mgr.start(spec("s1")).await.unwrap_err();
        assert!(matches!(err, TranscodeError::Spawn(_)));
    }

    #[tokio::test]
    async fn restarting_a_session_replaces_the_prior_one() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        mgr.start(spec("s1")).await.expect("start");
        mgr.start(spec("s1")).await.expect("restart");
        assert!(
            spawner.spawns.lock().unwrap()[0]
                .killed
                .load(Ordering::SeqCst)
        );
        assert_eq!(mgr.sessions.lock().await.len(), 1);
    }

    #[tokio::test]
    async fn touch_updates_last_active() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner);
        mgr.start(spec("s1")).await.expect("start");
        let id = SessionId("s1".to_owned());
        mgr.sessions.lock().await.get_mut(&id).unwrap().last_active = Timestamp::UNIX_EPOCH;
        mgr.touch(&id).await.expect("touch");
        assert_eq!(mgr.reap_idle().await, 0);
    }

    #[tokio::test]
    async fn touch_unknown_session_errors() {
        let (_dir, mgr) = manager(MockSpawner::default());
        let err = mgr.touch(&SessionId("nope".to_owned())).await.unwrap_err();
        assert!(matches!(err, TranscodeError::NotFound));
    }

    #[tokio::test]
    async fn stop_kills_child_and_evicts_dir() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        let started = mgr.start(spec("s1")).await.expect("start");
        mgr.stop(&SessionId("s1".to_owned())).await.expect("stop");
        assert!(
            spawner.spawns.lock().unwrap()[0]
                .killed
                .load(Ordering::SeqCst)
        );
        assert!(!PathBuf::from(&started.output_dir).exists());
        assert!(mgr.sessions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn stop_unknown_session_errors() {
        let (_dir, mgr) = manager(MockSpawner::default());
        let err = mgr.stop(&SessionId("nope".to_owned())).await.unwrap_err();
        assert!(matches!(err, TranscodeError::NotFound));
    }

    #[tokio::test]
    async fn reap_at_evicts_only_sessions_past_idle_timeout() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        mgr.start(spec("s1")).await.expect("start");
        let now = Timestamp::now();
        assert_eq!(mgr.reap_at(now + SignedDuration::from_secs(10)).await, 0);
        assert_eq!(mgr.reap_at(now + SignedDuration::from_secs(60)).await, 1);
        assert!(
            spawner.spawns.lock().unwrap()[0]
                .killed
                .load(Ordering::SeqCst)
        );
        assert!(mgr.sessions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn reap_idle_evicts_exited_child_even_when_fresh() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        mgr.start(spec("s1")).await.expect("start");
        spawner.spawns.lock().unwrap()[0]
            .exited
            .store(true, Ordering::SeqCst);
        assert_eq!(mgr.reap_idle().await, 1);
        assert!(mgr.sessions.lock().await.is_empty());
    }

    #[tokio::test]
    async fn dropping_manager_kills_children() {
        let spawner = MockSpawner::default();
        let (_dir, mgr) = manager(spawner.clone());
        mgr.start(spec("s1")).await.expect("start");
        drop(mgr);
        assert!(
            spawner.spawns.lock().unwrap()[0]
                .killed
                .load(Ordering::SeqCst)
        );
    }

    #[tokio::test]
    async fn configured_idle_timeout_drives_reaping() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = FfmpegTranscodeManager::with_spawner(dir.path(), MockSpawner::default())
            .with_idle_timeout(SignedDuration::from_secs(5));
        mgr.start(spec("s1")).await.expect("start");
        let now = Timestamp::now();
        assert_eq!(mgr.reap_at(now + SignedDuration::from_secs(3)).await, 0);
        assert_eq!(mgr.reap_at(now + SignedDuration::from_secs(6)).await, 1);
    }

    #[tokio::test]
    async fn new_constructs_with_default_spawner() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mgr = FfmpegTranscodeManager::new(dir.path());
        assert_eq!(mgr.reap_idle().await, 0);
    }

    #[test]
    fn is_expired_respects_timeout_boundary() {
        let base = Timestamp::UNIX_EPOCH;
        let timeout = SignedDuration::from_secs(30);
        assert!(!is_expired(
            base + SignedDuration::from_secs(30),
            base,
            timeout
        ));
        assert!(is_expired(
            base + SignedDuration::from_secs(31),
            base,
            timeout
        ));
    }
}
