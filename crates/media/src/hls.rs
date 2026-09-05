use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use jiff::{SignedDuration, Timestamp};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::spawn_blocking;

use domain::error::{StreamError, TranscodeError};
use domain::media::KeyframeProbe;
use domain::session::{
    DeliveryMode, SegmentContainer, SegmentPlan, SessionId, SoftSubtitle, StreamClaims,
    StreamRegistration, StreamRegistry, StreamSource, TranscodeManager, TranscodeSpec,
    TranscodeStarted, plan_segments, plan_segments_on_grid,
};

use crate::probe::FfprobeMediaProbe;
use crate::transcode::{
    DEFAULT_READ_RATE, INIT_SEGMENT, TARGET_MS, TokioProcessSpawner, VideoEncoder,
    build_producer_args, build_segment_args, build_subtitle_extract_args, media_playlist,
    segment_file_name, subtitle_media_playlist,
};
use domain::process::ProcessSpawner;

pub(crate) const VARIANT: &str = "v0";
pub(crate) const MEDIA_PLAYLIST: &str = "index.m3u8";
const SUBTITLE_GROUP: &str = "subs";
pub(crate) const SUBTITLE_VARIANT: &str = "subs";
const DEFAULT_BINARY: &str = "ffmpeg";
const DEFAULT_IDLE_TIMEOUT: SignedDuration = SignedDuration::from_secs(30);
const PRODUCED_WAIT: Duration = Duration::from_secs(30);
const PRODUCED_POLL: Duration = Duration::from_millis(100);

struct Session {
    registration: StreamRegistration,
    jit: Option<Jit>,
    producer: Option<Producer>,
    last_active: Timestamp,
}

struct Jit {
    plan: SegmentPlan,
    spec: TranscodeSpec,
}

#[derive(Default)]
struct ProducerState {
    failure: Option<String>,
    finished: bool,
}

struct Producer {
    task: Option<tokio::task::JoinHandle<()>>,
    state: Arc<Mutex<ProducerState>>,
}

impl Drop for Producer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

type SegmentLocks = HashMap<(SessionId, usize), Arc<AsyncMutex<()>>>;

struct Inner<S, P> {
    binary: String,
    cache_root: PathBuf,
    idle_timeout: SignedDuration,
    produced_wait: Duration,
    encoder: VideoEncoder,
    read_rate: f64,
    spawner: S,
    prober: P,
    sessions: RwLock<HashMap<SessionId, Session>>,
    locks: Mutex<SegmentLocks>,
}

pub struct HlsStreamSource<
    S: ProcessSpawner = TokioProcessSpawner,
    P: KeyframeProbe = FfprobeMediaProbe,
> {
    inner: Arc<Inner<S, P>>,
}

impl<S: ProcessSpawner, P: KeyframeProbe> Clone for HlsStreamSource<S, P> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl HlsStreamSource {
    pub fn new(cache_root: impl Into<PathBuf>) -> Self {
        Self::with_parts(
            cache_root,
            TokioProcessSpawner,
            FfprobeMediaProbe::default(),
        )
    }
}

impl<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static> HlsStreamSource<S, P> {
    pub fn with_parts(cache_root: impl Into<PathBuf>, spawner: S, prober: P) -> Self {
        Self {
            inner: Arc::new(Inner {
                binary: DEFAULT_BINARY.to_owned(),
                cache_root: cache_root.into(),
                idle_timeout: DEFAULT_IDLE_TIMEOUT,
                produced_wait: PRODUCED_WAIT,
                encoder: VideoEncoder::Software,
                read_rate: DEFAULT_READ_RATE,
                spawner,
                prober,
                sessions: RwLock::new(HashMap::new()),
                locks: Mutex::new(HashMap::new()),
            }),
        }
    }

    #[cfg(test)]
    pub fn with_idle_timeout(mut self, idle_timeout: SignedDuration) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("the idle timeout is set before the source is shared")
            .idle_timeout = idle_timeout;
        self
    }

    #[cfg(test)]
    pub fn with_produced_wait(mut self, produced_wait: Duration) -> Self {
        Arc::get_mut(&mut self.inner)
            .expect("the produced wait is set before the source is shared")
            .produced_wait = produced_wait;
        self
    }

    pub fn with_encoder(mut self, encoder: VideoEncoder) -> Self {
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.encoder = encoder;
        }
        self
    }

    pub fn with_read_rate(mut self, read_rate: f64) -> Self {
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.read_rate = read_rate;
        }
        self
    }

    fn touch_now(&self, session: &SessionId) {
        if let Some(s) = self.inner.sessions.write().unwrap().get_mut(session) {
            s.last_active = Timestamp::now();
        }
    }

    fn segment_lock(&self, session: &SessionId, index: usize) -> Arc<AsyncMutex<()>> {
        self.inner
            .locks
            .lock()
            .unwrap()
            .entry((session.clone(), index))
            .or_default()
            .clone()
    }

    fn purge_locks(&self, session: &SessionId) {
        self.inner
            .locks
            .lock()
            .unwrap()
            .retain(|(s, _), _| s != session);
    }

    async fn evict(&self, session: &SessionId) {
        let removed = self.inner.sessions.write().unwrap().remove(session);
        self.purge_locks(session);
        if let Some(entry) = removed {
            remove_dir_off_lock(entry.registration.output_dir).await;
        }
    }

    fn spawn_producer(
        &self,
        session: &SessionId,
        spec: &TranscodeSpec,
        variant_dir: &Path,
        origin_ms: u64,
        first_index: usize,
    ) -> Producer {
        let args = build_producer_args(
            spec,
            &variant_dir.to_string_lossy(),
            origin_ms,
            first_index,
            self.inner.read_rate,
        );
        let state = Arc::new(Mutex::new(ProducerState::default()));
        let task_state = Arc::clone(&state);
        let inner = Arc::clone(&self.inner);
        let id = session.0.clone();
        let task = tokio::spawn(async move {
            let outcome = inner.spawner.run_captured(&inner.binary, &args).await;
            let mut guard = task_state.lock().unwrap();
            guard.finished = true;
            match outcome {
                Ok(output) if output.success => {}
                Ok(output) => {
                    let detail = output.failure_detail(10);
                    tracing::warn!("producer for session [{id}] failed: {detail}");
                    guard.failure = Some(detail);
                }
                Err(err) => {
                    let binary = inner.binary.clone();
                    tracing::warn!("producer for session [{id}] could not run [{binary}]: {err}");
                    guard.failure = Some(err.to_string());
                }
            }
        });
        Producer {
            task: Some(task),
            state,
        }
    }

    async fn stop_producer(&self, session: &SessionId) {
        let handle = {
            let mut sessions = self.inner.sessions.write().unwrap();
            sessions
                .get_mut(session)
                .and_then(|s| s.producer.as_mut())
                .and_then(|p| p.task.take())
        };
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
    }

    fn session_container(&self, session: &SessionId) -> Result<SegmentContainer, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        let entry = sessions.get(session).ok_or(StreamError::NotLive)?;
        let jit = entry.jit.as_ref().ok_or(StreamError::Invalid)?;
        Ok(jit.spec.container)
    }

    fn producer_verdict(&self, session: &SessionId) -> Option<(bool, Option<String>)> {
        let sessions = self.inner.sessions.read().unwrap();
        let producer = sessions.get(session)?.producer.as_ref()?;
        let state = producer.state.lock().unwrap();
        Some((state.finished, state.failure.clone()))
    }

    async fn await_produced(&self, session: &SessionId, path: &Path) -> Result<(), StreamError> {
        let wait = self.inner.produced_wait;
        let deadline = Instant::now() + wait;
        loop {
            if has_content(path).await {
                self.touch_now(session);
                return Ok(());
            }
            match self.producer_verdict(session) {
                None => return Err(StreamError::NotLive),
                Some((_, Some(detail))) => {
                    let file = path.display().to_string();
                    tracing::warn!(
                        "session [{}] cannot serve [{file}] because the producer failed: {detail}",
                        session.0
                    );
                    return Err(StreamError::Invalid);
                }
                Some((true, None)) => {
                    if has_content(path).await {
                        self.touch_now(session);
                        return Ok(());
                    }
                    let file = path.display().to_string();
                    tracing::warn!(
                        "session [{}] was asked for [{file}], which the producer finished \
                         without writing",
                        session.0
                    );
                    return Err(StreamError::Invalid);
                }
                Some((false, None)) => {}
            }
            if Instant::now() >= deadline {
                let file = path.display().to_string();
                let seconds = wait.as_secs_f64();
                tracing::warn!(
                    "session [{}] gave up waiting [{seconds:.0}s] for [{file}]; the producer is \
                     still running but has not reached it",
                    session.0
                );
                return Err(StreamError::Invalid);
            }
            tokio::time::sleep(PRODUCED_POLL).await;
        }
    }

    async fn ensure_segment(
        &self,
        session: &SessionId,
        variant: &str,
        file: &str,
    ) -> Result<(), StreamError> {
        if variant != VARIANT {
            return Err(StreamError::Invalid);
        }
        let container = self.session_container(session)?;
        if container == SegmentContainer::Fmp4 {
            let base = {
                let sessions = self.inner.sessions.read().unwrap();
                sessions
                    .get(session)
                    .ok_or(StreamError::NotLive)?
                    .registration
                    .output_dir
                    .join(VARIANT)
            };
            return self.await_produced(session, &base.join(file)).await;
        }
        let index = parse_segment_index(file, container).ok_or(StreamError::Invalid)?;
        let (out_path, part_path, attempts) = {
            let sessions = self.inner.sessions.read().unwrap();
            let entry = sessions.get(session).ok_or(StreamError::NotLive)?;
            let jit = entry.jit.as_ref().ok_or(StreamError::Invalid)?;
            let segment = jit.plan.segments.get(index).ok_or(StreamError::Invalid)?;
            let variant_dir = entry.registration.output_dir.join(VARIANT);
            let out_path = variant_dir.join(segment_file_name(index, container));
            let part_path = variant_dir.join(segment_part_name(index, container));
            let part = part_path.to_string_lossy().into_owned();
            let mut attempts = vec![build_segment_args(
                &jit.spec,
                segment,
                &part,
                &self.inner.encoder,
            )];
            if self.inner.encoder.is_hardware() {
                attempts.push(build_segment_args(
                    &jit.spec,
                    segment,
                    &part,
                    &VideoEncoder::Software,
                ));
            }
            (out_path, part_path, attempts)
        };
        if out_path.is_file() {
            self.touch_now(session);
            return Ok(());
        }
        let lock = self.segment_lock(session, index);
        let _guard = lock.lock().await;
        if out_path.is_file() {
            self.touch_now(session);
            return Ok(());
        }
        let mut attempt = SegmentAttempt::started(session, index);
        let mut produced = false;
        for args in &attempts {
            match self
                .inner
                .spawner
                .run_captured(&self.inner.binary, args)
                .await
            {
                Ok(output) if output.success && has_content(&part_path).await => {
                    produced = true;
                    break;
                }
                Ok(output) => {
                    let detail = output.failure_detail(10);
                    tracing::warn!(
                        "segment [{index}] of session [{}] was not produced: {detail}",
                        session.0
                    );
                }
                Err(err) => {
                    let binary = self.inner.binary.clone();
                    tracing::warn!(
                        "segment [{index}] of session [{}] could not run [{binary}]: {err}",
                        session.0
                    );
                }
            }
        }
        attempt.settled();
        if !produced {
            remove_file_off_lock(part_path).await;
            return Err(StreamError::Invalid);
        }
        if !rename_off_lock(part_path, out_path).await {
            return Err(StreamError::Invalid);
        }
        self.touch_now(session);
        Ok(())
    }

    async fn write_subtitle_rendition(
        &self,
        output_dir: &Path,
        input_path: &str,
        soft: &SoftSubtitle,
    ) {
        let _ = self
            .try_write_subtitle_rendition(output_dir, input_path, soft)
            .await;
    }

    async fn try_write_subtitle_rendition(
        &self,
        output_dir: &Path,
        input_path: &str,
        soft: &SoftSubtitle,
    ) -> Result<(), TranscodeError> {
        let subs_dir = output_dir.join(SUBTITLE_VARIANT);
        let vtt_path = subs_dir.join("subs.vtt");
        let make = subs_dir.clone();
        spawn_blocking(move || std::fs::create_dir_all(make))
            .await
            .expect("create_dir_all task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let mut args = vec!["-v".to_owned(), "error".to_owned()];
        args.extend(build_subtitle_extract_args(
            input_path,
            &soft.source,
            &vtt_path.to_string_lossy(),
        ));
        let produced = self
            .inner
            .spawner
            .run(&self.inner.binary, &args)
            .await
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        if !produced {
            return Err(TranscodeError::Spawn(
                "subtitle extraction failed".to_owned(),
            ));
        }
        let offset = soft.offset_ms;
        let duration_ms = soft.duration_ms;
        spawn_blocking(move || -> std::io::Result<()> {
            let content = std::fs::read_to_string(&vtt_path)?;
            std::fs::write(&vtt_path, crate::subtitle::shift(&content, offset))?;
            let playlist = subtitle_media_playlist(duration_ms, "subs.vtt");
            std::fs::write(subs_dir.join(MEDIA_PLAYLIST), playlist)?;
            Ok(())
        })
        .await
        .expect("subtitle post-process task panicked")
        .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        Ok(())
    }

    async fn reap_at(&self, now: Timestamp) -> usize {
        let timeout = self.inner.idle_timeout;
        let evicted: Vec<(SessionId, PathBuf)> = {
            let mut sessions = self.inner.sessions.write().unwrap();
            let expired: Vec<SessionId> = sessions
                .iter()
                .filter(|(_, s)| s.jit.is_some() && is_expired(now, s.last_active, timeout))
                .map(|(id, _)| id.clone())
                .collect();
            expired
                .into_iter()
                .filter_map(|id| {
                    sessions
                        .remove(&id)
                        .map(|s| (id, s.registration.output_dir))
                })
                .collect()
        };
        for (id, dir) in &evicted {
            self.purge_locks(id);
            remove_dir_off_lock(dir.clone()).await;
        }
        evicted.len()
    }
}

impl<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static> StreamRegistry
    for HlsStreamSource<S, P>
{
    fn register(&self, session: SessionId, entry: StreamRegistration) {
        let mut sessions = self.inner.sessions.write().unwrap();
        match sessions.get_mut(&session) {
            Some(existing) => {
                existing.registration = entry;
                existing.last_active = Timestamp::now();
            }
            None => {
                sessions.insert(
                    session,
                    Session {
                        registration: entry,
                        jit: None,
                        producer: None,
                        last_active: Timestamp::now(),
                    },
                );
            }
        }
    }

    fn remove(&self, session: &SessionId) {
        self.inner.sessions.write().unwrap().remove(session);
        self.purge_locks(session);
    }
}

impl<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static> StreamSource
    for HlsStreamSource<S, P>
{
    fn master_playlist(&self, claims: &StreamClaims) -> Result<String, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        let reg = &sessions
            .get(&claims.session)
            .ok_or(StreamError::NotLive)?
            .registration;
        if reg.mode == DeliveryMode::Direct {
            return Err(StreamError::Invalid);
        }
        let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:3\n");
        let mut stream_inf = format!("#EXT-X-STREAM-INF:BANDWIDTH={}", reg.bandwidth);
        if let Some(sub) = &reg.subtitle {
            out.push_str(&format!(
                "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"{SUBTITLE_GROUP}\",NAME=\"{}\",LANGUAGE=\"{}\",AUTOSELECT=YES,DEFAULT=YES,URI=\"{SUBTITLE_VARIANT}/{MEDIA_PLAYLIST}\"\n",
                sub.name, sub.language,
            ));
            stream_inf.push_str(&format!(",SUBTITLES=\"{SUBTITLE_GROUP}\""));
        }
        out.push_str(&stream_inf);
        out.push('\n');
        out.push_str(&format!("{VARIANT}/{MEDIA_PLAYLIST}\n"));
        Ok(out)
    }

    async fn media_path(
        &self,
        claims: &StreamClaims,
        variant: &str,
        file: &str,
    ) -> Result<PathBuf, StreamError> {
        if !is_safe_segment(variant) || !is_safe_segment(file) {
            return Err(StreamError::Invalid);
        }
        let base = {
            let sessions = self.inner.sessions.read().unwrap();
            sessions
                .get(&claims.session)
                .ok_or(StreamError::NotLive)?
                .registration
                .output_dir
                .clone()
        };
        if is_segment_request(file) {
            self.ensure_segment(&claims.session, variant, file).await?;
        }
        resolve_media_path(&base, variant, file)
    }

    fn direct_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        sessions
            .get(&claims.session)
            .ok_or(StreamError::NotLive)?
            .registration
            .direct_path
            .clone()
            .ok_or(StreamError::Invalid)
    }
}

impl<S: ProcessSpawner + 'static, P: KeyframeProbe + 'static> TranscodeManager
    for HlsStreamSource<S, P>
{
    async fn start(&self, spec: TranscodeSpec) -> Result<TranscodeStarted, TranscodeError> {
        let session = spec.session.clone();
        let output_dir = self.inner.cache_root.join(&session.0);
        self.stop_producer(&session).await;
        self.evict(&session).await;
        let variant_dir = output_dir.join(VARIANT);
        let make = variant_dir.clone();
        spawn_blocking(move || std::fs::create_dir_all(make))
            .await
            .expect("create_dir_all task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let keyframes = self
            .inner
            .prober
            .keyframes(&spec.input_path)
            .await
            .unwrap_or_default();
        let container = spec.container;
        let full = match container {
            SegmentContainer::MpegTs => plan_segments(&keyframes, spec.duration_ms, TARGET_MS),
            SegmentContainer::Fmp4 => {
                plan_segments_on_grid(&keyframes, spec.duration_ms, TARGET_MS)
            }
        };
        let first = match container {
            SegmentContainer::Fmp4 => spec.seek_ms.map_or(0, |ms| full.index_at(ms)),
            SegmentContainer::MpegTs => 0,
        };
        let origin_ms = full.start_of(first);
        let plan = full.from_index(first);
        let playlist = media_playlist(&plan, spec.container, first);
        let index_path = variant_dir.join(MEDIA_PLAYLIST);
        spawn_blocking(move || std::fs::write(index_path, playlist))
            .await
            .expect("write playlist task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        if let Some(soft) = spec.soft_subtitle.clone() {
            self.write_subtitle_rendition(&output_dir, &spec.input_path, &soft)
                .await;
        }
        let producer = match spec.container {
            SegmentContainer::Fmp4 => {
                Some(self.spawn_producer(&session, &spec, &variant_dir, origin_ms, first))
            }
            SegmentContainer::MpegTs => None,
        };
        {
            let mut sessions = self.inner.sessions.write().unwrap();
            let entry = sessions.entry(session.clone()).or_insert_with(|| Session {
                registration: StreamRegistration {
                    mode: DeliveryMode::Remux,
                    output_dir: output_dir.clone(),
                    direct_path: None,
                    bandwidth: 0,
                    subtitle: None,
                },
                jit: None,
                producer: None,
                last_active: Timestamp::now(),
            });
            entry.registration.output_dir = output_dir.clone();
            entry.jit = Some(Jit { plan, spec });
            entry.producer = producer;
            entry.last_active = Timestamp::now();
        }
        Ok(TranscodeStarted {
            session,
            output_dir: output_dir.to_string_lossy().into_owned(),
            origin_ms,
            sequential: container == SegmentContainer::Fmp4,
        })
    }

    async fn touch(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut sessions = self.inner.sessions.write().unwrap();
        match sessions.get_mut(session) {
            Some(s) if s.jit.is_some() => {
                s.last_active = Timestamp::now();
                Ok(())
            }
            _ => Err(TranscodeError::NotFound),
        }
    }

    async fn stop(&self, session: &SessionId) -> Result<(), TranscodeError> {
        self.stop_producer(session).await;
        let removed = self.inner.sessions.write().unwrap().remove(session);
        let entry = removed.ok_or(TranscodeError::NotFound)?;
        self.purge_locks(session);
        remove_dir_off_lock(entry.registration.output_dir).await;
        Ok(())
    }

    async fn reap_idle(&self) -> usize {
        self.reap_at(Timestamp::now()).await
    }
}

async fn remove_dir_off_lock(dir: PathBuf) {
    let _ = spawn_blocking(move || std::fs::remove_dir_all(&dir)).await;
}

struct SegmentAttempt {
    session: String,
    index: usize,
    started: Instant,
    settled: bool,
}

impl SegmentAttempt {
    fn started(session: &SessionId, index: usize) -> Self {
        Self {
            session: session.0.clone(),
            index,
            started: Instant::now(),
            settled: false,
        }
    }

    fn settled(&mut self) {
        self.settled = true;
    }
}

impl Drop for SegmentAttempt {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        let seconds = self.started.elapsed().as_secs_f64();
        tracing::warn!(
            "segment [{}] of session [{}] was abandoned after [{seconds:.1}s]: the client stopped waiting before it could be produced",
            self.index,
            self.session
        );
    }
}

async fn remove_file_off_lock(path: PathBuf) {
    let _ = spawn_blocking(move || std::fs::remove_file(&path)).await;
}

async fn rename_off_lock(from: PathBuf, to: PathBuf) -> bool {
    spawn_blocking(move || std::fs::rename(&from, &to).is_ok())
        .await
        .unwrap_or(false)
}

async fn has_content(path: &Path) -> bool {
    let path = path.to_path_buf();
    spawn_blocking(move || {
        std::fs::metadata(&path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

fn segment_part_name(index: usize, container: SegmentContainer) -> String {
    format!("{}.part", segment_file_name(index, container))
}

fn resolve_media_path(base: &Path, variant: &str, file: &str) -> Result<PathBuf, StreamError> {
    let base = base.canonicalize().map_err(|_| StreamError::NotLive)?;
    let dir = base.join(variant);
    let dir = match dir.canonicalize() {
        Ok(real) if real.starts_with(&base) => real,
        Ok(_) => return Err(StreamError::Invalid),
        Err(_) => return Ok(dir.join(file)),
    };
    let candidate = dir.join(file);
    match candidate.canonicalize() {
        Ok(real) if real.starts_with(&base) => Ok(real),
        Ok(_) => Err(StreamError::Invalid),
        Err(_) => Ok(candidate),
    }
}

fn is_safe_segment(segment: &str) -> bool {
    !segment.is_empty() && segment != ".." && !segment.contains('/') && !segment.contains('\\')
}

fn parse_segment_index(file: &str, container: SegmentContainer) -> Option<usize> {
    let suffix = format!(".{}", container.segment_extension());
    file.strip_prefix("seg_")?
        .strip_suffix(&suffix)?
        .parse()
        .ok()
}

fn is_segment_request(file: &str) -> bool {
    file.ends_with(".ts") || file.ends_with(".m4s") || file == INIT_SEGMENT
}

fn is_expired(now: Timestamp, last_active: Timestamp, timeout: SignedDuration) -> bool {
    now.duration_since(last_active) > timeout
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::catalog::VersionId;
    use domain::error::ProbeError;
    use domain::session::{SoftSubtitleSource, SubtitleRendition};
    use domain::user::UserId;

    #[derive(Clone, Default)]
    struct MockSpawner {
        fail: bool,
        unspawnable: bool,
        silent: bool,
        calls: Arc<AtomicUsize>,
    }

    impl ProcessSpawner for MockSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.unspawnable {
                return Err(std::io::Error::other("no such binary"));
            }
            if self.fail {
                return Ok(false);
            }
            if !self.silent
                && let Some(out) = args.last()
            {
                std::fs::write(out, b"SEGMENT").ok();
            }
            Ok(true)
        }
    }

    #[derive(Clone, Default)]
    struct KilledMidWriteSpawner {
        calls: Arc<AtomicUsize>,
    }

    impl ProcessSpawner for KilledMidWriteSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(out) = args.last() {
                std::fs::write(out, b"HALF").ok();
            }
            Ok(false)
        }
    }

    #[derive(Clone, Default)]
    struct FailFirstSpawner {
        calls: Arc<AtomicUsize>,
    }

    impl ProcessSpawner for FailFirstSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                return Ok(false);
            }
            if let Some(out) = args.last() {
                std::fs::write(out, b"SEGMENT").ok();
            }
            Ok(true)
        }
    }

    #[derive(Clone, Default)]
    struct MockProbe {
        keyframes: Vec<u64>,
        fail: bool,
    }

    impl KeyframeProbe for MockProbe {
        async fn keyframes(&self, _path: &str) -> Result<Vec<u64>, ProbeError> {
            if self.fail {
                return Err(ProbeError::Backend("mock".to_owned()));
            }
            Ok(self.keyframes.clone())
        }
    }

    fn engine(
        spawner: MockSpawner,
        prober: MockProbe,
    ) -> (tempfile::TempDir, HlsStreamSource<MockSpawner, MockProbe>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), spawner, prober);
        (dir, src)
    }

    fn claims(session: &str) -> StreamClaims {
        StreamClaims {
            session: SessionId(session.to_owned()),
            user: UserId("u1".to_owned()),
            version: VersionId("ver-1".to_owned()),
            expires_at: Timestamp::now(),
            nonce: String::new(),
        }
    }

    fn transcode_registration(output_dir: PathBuf) -> StreamRegistration {
        StreamRegistration {
            mode: DeliveryMode::Transcode,
            output_dir,
            direct_path: None,
            bandwidth: 4_000_000,
            subtitle: None,
        }
    }

    fn spec(id: &str, duration_ms: u64) -> TranscodeSpec {
        TranscodeSpec {
            session: SessionId(id.to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            duration_ms,
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
    async fn master_playlist_lists_single_variant() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/cache/s1")),
        );
        let master = src.master_playlist(&claims("s1")).unwrap();
        assert!(master.starts_with("#EXTM3U"));
        assert!(master.contains("#EXT-X-STREAM-INF:BANDWIDTH=4000000"));
        assert!(master.contains("v0/index.m3u8"));
        assert!(!master.contains("SUBTITLES"));
    }

    #[tokio::test]
    async fn master_playlist_includes_subtitle_rendition() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            StreamRegistration {
                subtitle: Some(SubtitleRendition {
                    name: "English".to_owned(),
                    language: "en".to_owned(),
                }),
                ..transcode_registration(PathBuf::from("/cache/s1"))
            },
        );
        let master = src.master_playlist(&claims("s1")).unwrap();
        assert!(master.contains(
            "#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"subs\",NAME=\"English\",LANGUAGE=\"en\""
        ));
        assert!(master.contains("URI=\"subs/index.m3u8\""));
        assert!(master.contains("#EXT-X-STREAM-INF:BANDWIDTH=4000000,SUBTITLES=\"subs\""));
    }

    #[tokio::test]
    async fn master_playlist_direct_is_invalid() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            StreamRegistration {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/media/movie.mkv")),
                ..transcode_registration(PathBuf::new())
            },
        );
        assert!(matches!(
            src.master_playlist(&claims("s1")),
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn master_playlist_unknown_session_not_live() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        assert!(matches!(
            src.master_playlist(&claims("nope")),
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn remove_deregisters_session() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/cache/s1")),
        );
        src.remove(&SessionId("s1".to_owned()));
        assert!(matches!(
            src.master_playlist(&claims("s1")),
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn direct_file_returns_path() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            StreamRegistration {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/media/movie.mkv")),
                ..transcode_registration(PathBuf::new())
            },
        );
        assert_eq!(
            src.direct_file(&claims("s1")).unwrap(),
            PathBuf::from("/media/movie.mkv")
        );
    }

    #[tokio::test]
    async fn direct_file_without_path_is_invalid() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/cache/s1")),
        );
        assert!(matches!(
            src.direct_file(&claims("s1")),
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn direct_file_unknown_session_not_live() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        assert!(matches!(
            src.direct_file(&claims("nope")),
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn start_writes_vod_playlist_and_serves_it() {
        let prober = MockProbe {
            keyframes: vec![4_000, 8_000, 12_000],
            fail: false,
        };
        let (_dir, src) = engine(MockSpawner::default(), prober);
        let started = src.start(spec("s1", 15_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        let index = PathBuf::from(&started.output_dir)
            .join("v0")
            .join("index.m3u8");
        let text = std::fs::read_to_string(&index).unwrap();
        assert!(text.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(text.contains("#EXT-X-ENDLIST"));
        assert!(text.contains("seg_00000.ts"));
        let path = src
            .media_path(&claims("s1"), "v0", "index.m3u8")
            .await
            .unwrap();
        assert_eq!(path, index.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn start_probe_failure_falls_back_to_single_segment() {
        let prober = MockProbe {
            keyframes: vec![],
            fail: true,
        };
        let (_dir, src) = engine(MockSpawner::default(), prober);
        let started = src.start(spec("s1", 15_000)).await.expect("start");
        let index = PathBuf::from(&started.output_dir)
            .join("v0")
            .join("index.m3u8");
        let text = std::fs::read_to_string(&index).unwrap();
        assert_eq!(text.matches("#EXTINF").count(), 1);
        assert!(text.contains("seg_00000.ts"));
    }

    #[tokio::test]
    async fn start_create_dir_failure_maps_to_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let src =
            HlsStreamSource::with_parts(file.path(), MockSpawner::default(), MockProbe::default());
        let err = src.start(spec("s1", 1_000)).await.unwrap_err();
        assert!(matches!(err, TranscodeError::Spawn(_)));
    }

    #[tokio::test]
    async fn restarting_a_session_replaces_the_prior_one() {
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(MockSpawner::default(), prober);
        src.start(spec("s1", 8_000)).await.expect("start");
        src.start(spec("s1", 8_000)).await.expect("restart");
        let sessions = src.inner.sessions.read().unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(
            sessions
                .get(&SessionId("s1".to_owned()))
                .unwrap()
                .jit
                .is_some()
        );
    }

    #[tokio::test]
    async fn start_writes_subtitle_rendition() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        let mut spec = spec("s1", 10_000);
        spec.soft_subtitle = Some(SoftSubtitle {
            source: SoftSubtitleSource::File("/media/movie.en.srt".to_owned()),
            offset_ms: 0,
            duration_ms: 10_000,
        });
        let started = src.start(spec).await.expect("start");
        let subs = PathBuf::from(&started.output_dir).join("subs");
        assert!(subs.join("subs.vtt").exists());
        let playlist = std::fs::read_to_string(subs.join("index.m3u8")).unwrap();
        assert!(playlist.contains("subs.vtt"));
        assert!(playlist.contains("#EXT-X-ENDLIST"));
    }

    #[tokio::test]
    async fn start_subtitle_failure_is_best_effort() {
        let spawner = MockSpawner {
            fail: true,
            ..Default::default()
        };
        let (_dir, src) = engine(spawner, MockProbe::default());
        let mut spec = spec("s1", 10_000);
        spec.soft_subtitle = Some(SoftSubtitle {
            source: SoftSubtitleSource::File("/media/missing.srt".to_owned()),
            offset_ms: 0,
            duration_ms: 10_000,
        });
        let started = src.start(spec).await.expect("start");
        let subs = PathBuf::from(&started.output_dir).join("subs");
        assert!(!subs.join("index.m3u8").exists());
    }

    #[tokio::test]
    async fn segment_is_produced_then_cached() {
        let spawner = MockSpawner::default();
        let prober = MockProbe {
            keyframes: vec![4_000, 8_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner.clone(), prober);
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        let path = src
            .media_path(&claims("s1"), "v0", "seg_00001.ts")
            .await
            .unwrap();
        assert!(path.exists());
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 1);
        src.media_path(&claims("s1"), "v0", "seg_00001.ts")
            .await
            .unwrap();
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn hardware_segment_failure_falls_back_to_software() {
        let spawner = FailFirstSpawner::default();
        let prober = MockProbe {
            keyframes: vec![4_000, 8_000],
            fail: false,
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), spawner.clone(), prober).with_encoder(
            VideoEncoder::Vaapi {
                device: "/dev/dri/renderD128".to_owned(),
            },
        );
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        let path = src
            .media_path(&claims("s1"), "v0", "seg_00000.ts")
            .await
            .unwrap();
        assert!(path.exists());
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn concurrent_segment_requests_produce_once() {
        let spawner = MockSpawner::default();
        let prober = MockProbe {
            keyframes: vec![4_000, 8_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner.clone(), prober);
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        let c = claims("s1");
        let a = src.media_path(&c, "v0", "seg_00000.ts");
        let b = src.media_path(&c, "v0", "seg_00000.ts");
        let (ra, rb) = tokio::join!(a, b);
        ra.unwrap();
        rb.unwrap();
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn segment_out_of_range_is_invalid() {
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(MockSpawner::default(), prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_09999.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn segment_bad_name_is_invalid() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        for file in ["seg_x.ts", "seg_.ts", "foo.ts"] {
            assert!(matches!(
                src.media_path(&claims("s1"), "v0", file).await,
                Err(StreamError::Invalid)
            ));
        }
    }

    #[tokio::test]
    async fn ensure_segment_rejects_wrong_variant_and_unknown_session() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        assert!(matches!(
            src.ensure_segment(&SessionId("s1".to_owned()), "subs", "seg_00000.ts")
                .await,
            Err(StreamError::Invalid)
        ));
        assert!(matches!(
            src.ensure_segment(&SessionId("nope".to_owned()), "v0", "seg_00000.ts")
                .await,
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn media_path_falls_back_to_join_for_absent_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::create_dir_all(base.join("v0")).unwrap();
        let (_cache, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(base.clone()),
        );
        let canonical_base = base.canonicalize().unwrap();
        let absent_variant = src
            .media_path(&claims("s1"), "subs", "index.m3u8")
            .await
            .unwrap();
        assert_eq!(
            absent_variant,
            canonical_base.join("subs").join("index.m3u8")
        );
        let absent_file = src
            .media_path(&claims("s1"), "v0", "other.m3u8")
            .await
            .unwrap();
        assert!(absent_file.ends_with("v0/other.m3u8"));
    }

    #[tokio::test]
    async fn media_path_missing_base_is_not_live() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/no/such/shadowmask/dir")),
        );
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "index.m3u8").await,
            Err(StreamError::NotLive)
        ));
    }

    #[test]
    fn is_expired_respects_boundary() {
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

    #[tokio::test]
    async fn segment_without_jit_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::create_dir_all(base.join("v0")).unwrap();
        let (_cache, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(SessionId("s1".to_owned()), transcode_registration(base));
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn segment_production_failure_is_invalid() {
        let spawner = MockSpawner {
            fail: true,
            ..Default::default()
        };
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner, prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn segment_production_that_cannot_spawn_is_invalid() {
        let spawner = MockSpawner {
            unspawnable: true,
            ..Default::default()
        };
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner, prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        assert!(
            matches!(
                src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
                Err(StreamError::Invalid)
            ),
            "a missing ffmpeg is an io error rather than a failed exit, and must not be \
             mistaken for a produced segment"
        );
    }

    #[tokio::test]
    async fn a_segment_killed_part_way_is_never_served_or_cached() {
        // A cancelled request SIGKILLs ffmpeg through kill_on_drop, so it can
        // leave a truncated file behind and no cleanup code of ours ever runs.
        // Writing to a part file and renaming is what keeps that unservable.
        let spawner = KilledMidWriteSpawner::default();
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), spawner.clone(), prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        let out_dir = PathBuf::from(&started.output_dir);
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(out_dir.clone()),
        );

        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
        assert!(
            !out_dir.join(VARIANT).join("seg_00000.ts").exists(),
            "a half written segment must not be left where it can be served"
        );

        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
        assert_eq!(
            spawner.calls.load(Ordering::SeqCst),
            2,
            "the retry has to re-run ffmpeg rather than short circuit on the corpse"
        );
    }

    #[test]
    fn an_unsettled_segment_attempt_reports_itself_on_drop() {
        // Cancelling the request drops this mid-flight, and Drop is the only
        // thing that still runs then, so it is the one place an abandoned
        // encode can be recorded at all.
        drop(SegmentAttempt::started(&SessionId("s1".to_owned()), 3));

        let mut settled = SegmentAttempt::started(&SessionId("s1".to_owned()), 4);
        settled.settled();
        drop(settled);
    }

    #[tokio::test]
    async fn a_segment_that_cannot_be_moved_into_place_is_invalid() {
        let spawner = MockSpawner::default();
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner, prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        let out_dir = PathBuf::from(&started.output_dir);
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(out_dir.clone()),
        );
        // A directory sitting where the segment belongs makes the rename fail,
        // which must surface rather than be reported as a produced segment.
        std::fs::create_dir_all(out_dir.join(VARIANT).join("seg_00000.ts")).unwrap();

        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn segment_reported_produced_but_missing_is_invalid() {
        let spawner = MockSpawner {
            silent: true,
            ..Default::default()
        };
        let prober = MockProbe {
            keyframes: vec![4_000],
            fail: false,
        };
        let (_dir, src) = engine(spawner, prober);
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn media_playlist_resolves_without_production() {
        let spawner = MockSpawner::default();
        let (_dir, src) = engine(spawner.clone(), MockProbe::default());
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from(&started.output_dir)),
        );
        src.media_path(&claims("s1"), "v0", "index.m3u8")
            .await
            .unwrap();
        assert_eq!(spawner.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn media_path_unknown_session_not_live() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        assert!(matches!(
            src.media_path(&claims("nope"), "v0", "index.m3u8").await,
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn media_path_rejects_unsafe_segments() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/cache/s1")),
        );
        for (variant, file) in [("..", "index.m3u8"), ("v0", ""), ("v0", "a\\b")] {
            assert!(matches!(
                src.media_path(&claims("s1"), variant, file).await,
                Err(StreamError::Invalid)
            ));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn media_path_rejects_symlinked_variant_escape() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), b"x").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::os::unix::fs::symlink(outside.path(), base.join("v0")).unwrap();
        let (_cache, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(SessionId("s1".to_owned()), transcode_registration(base));
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "secret").await,
            Err(StreamError::Invalid)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn media_path_rejects_symlinked_file_escape() {
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret");
        std::fs::write(&secret, b"x").unwrap();
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::create_dir_all(base.join("v0")).unwrap();
        std::os::unix::fs::symlink(&secret, base.join("v0").join("leak.bin")).unwrap();
        let (_cache, src) = engine(MockSpawner::default(), MockProbe::default());
        src.register(SessionId("s1".to_owned()), transcode_registration(base));
        assert!(matches!(
            src.media_path(&claims("s1"), "v0", "leak.bin").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn touch_updates_started_and_rejects_others() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.touch(&SessionId("s1".to_owned())).await.expect("touch");
        assert!(matches!(
            src.touch(&SessionId("nope".to_owned())).await,
            Err(TranscodeError::NotFound)
        ));
        src.register(
            SessionId("direct".to_owned()),
            StreamRegistration {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/m.mkv")),
                ..transcode_registration(PathBuf::new())
            },
        );
        assert!(matches!(
            src.touch(&SessionId("direct".to_owned())).await,
            Err(TranscodeError::NotFound)
        ));
    }

    #[tokio::test]
    async fn stop_evicts_cache_and_unknown_errors() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.stop(&SessionId("s1".to_owned())).await.expect("stop");
        assert!(!PathBuf::from(&started.output_dir).exists());
        assert!(matches!(
            src.stop(&SessionId("nope".to_owned())).await,
            Err(TranscodeError::NotFound)
        ));
    }

    #[tokio::test]
    async fn reap_evicts_idle_jit_sessions_only() {
        let dir = tempfile::tempdir().unwrap();
        let src =
            HlsStreamSource::with_parts(dir.path(), MockSpawner::default(), MockProbe::default())
                .with_idle_timeout(SignedDuration::from_secs(5));
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(
            SessionId("direct".to_owned()),
            StreamRegistration {
                mode: DeliveryMode::Direct,
                direct_path: Some(PathBuf::from("/m.mkv")),
                ..transcode_registration(PathBuf::new())
            },
        );
        let now = Timestamp::now();
        assert_eq!(src.reap_at(now + SignedDuration::from_secs(3)).await, 0);
        assert_eq!(src.reap_at(now + SignedDuration::from_secs(6)).await, 1);
        assert!(!PathBuf::from(&started.output_dir).exists());
        assert!(
            src.inner
                .sessions
                .read()
                .unwrap()
                .contains_key(&SessionId("direct".to_owned()))
        );
    }

    #[tokio::test]
    async fn reap_idle_uses_wall_clock() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        assert_eq!(src.reap_idle().await, 0);
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        let cloned = src.clone();
        src.register(
            SessionId("s1".to_owned()),
            transcode_registration(PathBuf::from("/cache/s1")),
        );
        assert!(cloned.master_playlist(&claims("s1")).is_ok());
    }

    struct DropFlag(Arc<AtomicUsize>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Clone, Default)]
    struct ProducerSpawner {
        fail: bool,
        unspawnable: bool,
        hang: bool,
        empty_init: bool,
        segments: usize,
        dropped: Arc<AtomicUsize>,
    }

    impl ProcessSpawner for ProducerSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            if self.hang {
                let _flag = DropFlag(Arc::clone(&self.dropped));
                std::future::pending::<()>().await;
            }
            if self.unspawnable {
                return Err(std::io::Error::other("no such binary"));
            }
            if self.fail {
                return Ok(false);
            }
            let playlist = args.last().expect("the producer is given a playlist path");
            let dir = Path::new(playlist)
                .parent()
                .expect("the playlist sits in the variant directory");
            let init: &[u8] = if self.empty_init { b"" } else { b"INIT" };
            std::fs::write(dir.join(INIT_SEGMENT), init).ok();
            for index in 0..self.segments {
                let name = segment_file_name(index, SegmentContainer::Fmp4);
                std::fs::write(dir.join(name), b"SEGMENT").ok();
            }
            Ok(true)
        }
    }

    fn fmp4_engine(
        spawner: ProducerSpawner,
        prober: MockProbe,
    ) -> (
        tempfile::TempDir,
        HlsStreamSource<ProducerSpawner, MockProbe>,
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), spawner, prober);
        (dir, src)
    }

    fn fmp4_spec(id: &str, duration_ms: u64) -> TranscodeSpec {
        TranscodeSpec {
            copy: true,
            container: SegmentContainer::Fmp4,
            ..spec(id, duration_ms)
        }
    }

    #[tokio::test]
    async fn an_fmp4_session_serves_a_playlist_that_maps_the_init_segment() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                segments: 2,
                ..ProducerSpawner::default()
            },
            MockProbe {
                keyframes: vec![0, 4_000, 8_000],
                fail: false,
            },
        );
        let started = src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        let playlist = std::fs::read_to_string(
            PathBuf::from(&started.output_dir)
                .join(VARIANT)
                .join(MEDIA_PLAYLIST),
        )
        .expect("the playlist is written at start, before any segment exists");
        assert!(playlist.contains("#EXT-X-VERSION:7"));
        assert!(playlist.contains("#EXT-X-MAP:URI=\"init.mp4\""));
        assert!(playlist.contains("seg_00000.m4s"));
        assert!(playlist.contains("#EXT-X-ENDLIST"));
    }

    #[tokio::test]
    async fn an_fmp4_segment_is_served_once_the_producer_has_written_it() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                segments: 3,
                ..ProducerSpawner::default()
            },
            MockProbe {
                keyframes: vec![0, 4_000, 8_000],
                fail: false,
            },
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        let path = src
            .media_path(&claims("s1"), VARIANT, "seg_00000.m4s")
            .await
            .expect("the segment is served after the producer writes it");
        assert!(path.is_file());
        let init = src
            .media_path(&claims("s1"), VARIANT, INIT_SEGMENT)
            .await
            .expect("the init segment is served like any other produced file");
        assert!(init.is_file());
    }

    #[tokio::test]
    async fn an_fmp4_segment_is_invalid_when_the_producer_fails() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                fail: true,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, "seg_00000.m4s")
                    .await,
                Err(StreamError::Invalid)
            ),
            "a failed producer must fail fast, not wait out the clock"
        );
    }

    #[tokio::test]
    async fn an_fmp4_segment_the_producer_finished_without_writing_is_invalid() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                segments: 1,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, "seg_00042.m4s")
                    .await,
                Err(StreamError::Invalid)
            ),
            "the producer is done, so this segment is never coming"
        );
    }

    #[tokio::test]
    async fn stopping_an_fmp4_session_kills_the_producer() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                dropped: Arc::clone(&dropped),
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        tokio::task::yield_now().await;
        assert_eq!(dropped.load(Ordering::SeqCst), 0, "the producer is running");

        src.stop(&SessionId("s1".to_owned())).await.expect("stop");
        for _ in 0..50 {
            if dropped.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "dropping the session must drop the producer future, which is what SIGKILLs ffmpeg; \
             a producer that outlives its session keeps writing to a deleted directory"
        );
    }

    #[tokio::test]
    async fn reaping_an_idle_fmp4_session_kills_the_producer() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                dropped: Arc::clone(&dropped),
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        let src = src.with_idle_timeout(SignedDuration::from_secs(5));
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        tokio::task::yield_now().await;

        let reaped = src
            .reap_at(Timestamp::now() + SignedDuration::from_secs(6))
            .await;
        assert_eq!(reaped, 1);
        for _ in 0..50 {
            if dropped.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_restart_waits_for_the_old_producer_to_die_before_starting_the_new_one() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                dropped: Arc::clone(&dropped),
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        tokio::task::yield_now().await;

        src.start(fmp4_spec("s1", 12_000)).await.expect("restart");

        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "the old ffmpeg must be gone before the directory is recreated; two producers writing \
             one directory mix segments from different timelines, which reads as partial files \
             and decodes as torn video"
        );
    }

    #[tokio::test]
    async fn stopping_waits_for_the_producer_rather_than_only_asking_it_to_stop() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                dropped: Arc::clone(&dropped),
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        tokio::task::yield_now().await;

        src.stop(&SessionId("s1".to_owned())).await.expect("stop");

        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn restarting_an_fmp4_session_kills_the_previous_producer() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                dropped: Arc::clone(&dropped),
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        tokio::task::yield_now().await;
        src.start(fmp4_spec("s1", 12_000)).await.expect("restart");
        for _ in 0..50 {
            if dropped.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            dropped.load(Ordering::SeqCst),
            1,
            "a seek restarts the session, and the old producer must not race the new one"
        );
    }

    #[tokio::test]
    async fn an_fmp4_session_resumed_at_a_seek_serves_only_the_tail() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                segments: 4,
                ..ProducerSpawner::default()
            },
            MockProbe {
                keyframes: vec![0, 4_000, 8_000, 12_000],
                fail: false,
            },
        );
        let started = src
            .start(TranscodeSpec {
                seek_ms: Some(9_000),
                ..fmp4_spec("s1", 16_000)
            })
            .await
            .expect("start");
        assert_eq!(
            started.origin_ms, 8_000,
            "the timeline begins at the segment boundary holding the seek, not at the seek itself"
        );
        let playlist = std::fs::read_to_string(
            PathBuf::from(&started.output_dir)
                .join(VARIANT)
                .join(MEDIA_PLAYLIST),
        )
        .expect("playlist");
        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:2"));
        assert!(playlist.contains("seg_00002.m4s"));
        assert!(
            !playlist.contains("seg_00000.m4s"),
            "a restarted producer never writes the earlier segments, so listing them breaks \
             the demuxer, which opens segment zero regardless of where the player seeks"
        );
    }

    #[tokio::test]
    async fn a_transcode_session_ignores_the_seek_and_keeps_the_whole_timeline() {
        let (_dir, src) = engine(
            MockSpawner::default(),
            MockProbe {
                keyframes: vec![0, 4_000, 8_000, 12_000],
                fail: false,
            },
        );
        let started = src
            .start(TranscodeSpec {
                seek_ms: Some(9_000),
                ..spec("s1", 16_000)
            })
            .await
            .expect("start");
        assert_eq!(started.origin_ms, 0);
        let playlist = std::fs::read_to_string(
            PathBuf::from(&started.output_dir)
                .join(VARIANT)
                .join(MEDIA_PLAYLIST),
        )
        .expect("playlist");
        assert!(
            playlist.contains("seg_00000.ts"),
            "mpegts stays just in time, so every segment remains reachable and seeking is local"
        );
    }

    #[derive(Clone, Default)]
    struct ArgRecordingSpawner {
        args: Arc<Mutex<Vec<String>>>,
    }

    impl ProcessSpawner for ArgRecordingSpawner {
        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            *self.args.lock().unwrap() = args.to_vec();
            let playlist = args.last().expect("the producer is given a playlist path");
            let dir = Path::new(playlist).parent().expect("a variant directory");
            std::fs::write(dir.join(INIT_SEGMENT), b"INIT").ok();
            Ok(true)
        }
    }

    #[tokio::test]
    async fn the_configured_read_rate_reaches_the_producer() {
        let spawner = ArgRecordingSpawner::default();
        let seen = Arc::clone(&spawner.args);
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), spawner, MockProbe::default())
            .with_read_rate(3.5);
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        for _ in 0..50 {
            if !seen.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let args = seen.lock().unwrap().clone();
        let rate = args
            .iter()
            .position(|a| a == "-readrate")
            .map(|at| args[at + 1].clone());
        assert_eq!(
            rate.as_deref(),
            Some("3.5"),
            "an admin who lowers the read rate to protect their disks must actually see it applied"
        );
    }

    #[tokio::test]
    async fn an_init_segment_the_producer_has_only_created_is_never_served() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                empty_init: true,
                segments: 1,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        let src = src.with_produced_wait(Duration::from_millis(250));
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, INIT_SEGMENT).await,
                Err(StreamError::Invalid)
            ),
            "ffmpeg creates init.mp4 empty and only fills it when the first segment closes, and \
             hls_flags temp_file does not cover it; serving the empty file leaves every client \
             unable to decode a single segment"
        );
    }

    #[tokio::test]
    async fn an_fmp4_segment_is_invalid_when_the_producer_cannot_be_spawned() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                unspawnable: true,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, "seg_00000.m4s")
                    .await,
                Err(StreamError::Invalid)
            ),
            "a missing ffmpeg must surface as a failed segment, not a thirty second hang"
        );
    }

    #[tokio::test]
    async fn an_fmp4_segment_gives_up_when_the_producer_never_reaches_it() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        let src = src.with_produced_wait(Duration::from_millis(250));
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, "seg_00000.m4s")
                    .await,
                Err(StreamError::Invalid)
            ),
            "a producer that is alive but far behind must not hold the request open forever"
        );
    }

    #[tokio::test]
    async fn an_fmp4_segment_is_not_live_once_the_producer_is_gone() {
        let (_dir, src) = fmp4_engine(
            ProducerSpawner {
                hang: true,
                ..ProducerSpawner::default()
            },
            MockProbe::default(),
        );
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        src.inner
            .sessions
            .write()
            .unwrap()
            .get_mut(&SessionId("s1".to_owned()))
            .expect("the session exists")
            .producer = None;
        assert!(
            matches!(
                src.media_path(&claims("s1"), VARIANT, "seg_00000.m4s")
                    .await,
                Err(StreamError::NotLive)
            ),
            "without a producer nothing will ever write the segment, so waiting is pointless"
        );
    }

    #[tokio::test]
    async fn a_transcode_session_starts_no_producer() {
        let (_dir, src) = engine(MockSpawner::default(), MockProbe::default());
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        assert!(
            !PathBuf::from(&started.output_dir)
                .join(VARIANT)
                .join(INIT_SEGMENT)
                .exists(),
            "mpegts sessions stay just in time and spawn nothing"
        );
        assert!(
            src.inner
                .sessions
                .read()
                .unwrap()
                .get(&SessionId("s1".to_owned()))
                .expect("the session exists")
                .producer
                .is_none()
        );
    }
}
