use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use jiff::{SignedDuration, Timestamp};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::spawn_blocking;

use domain::catalog::VersionId;
use domain::error::{StreamError, TranscodeError};
use domain::session::{
    BurnSubtitle, DeliveryMode, SegmentContainer, SegmentPlan, SessionId, SoftSubtitle,
    SoftSubtitleSource, StreamClaims, StreamGeneration, StreamRegistration, StreamRegistry,
    StreamSource, SubtitleDelivery, SubtitleFileSpec, TranscodeManager, TranscodeSpec,
    TranscodeStarted, plan_cuts, plan_fixed_segments,
};

use crate::playback_prep_store::{FsPlaybackPrepStore, PrepEntry, embedded_prefix};
use crate::stream_metrics;
use crate::subtitle::{shift, with_timestamp_map};
use crate::transcode::{
    Burn, DEFAULT_READ_RATE, INIT_SEGMENT, OUTPUT_OFFSET_MS, RunWindow, TARGET_MS,
    TokioProcessSpawner, VideoEncoder, build_run_args, build_subtitle_extract_args, film_start,
    media_playlist, segment_file_name, subtitle_media_playlist,
};
use domain::process::ProcessSpawner;

pub(crate) const VARIANT: &str = "v0";
pub(crate) const MEDIA_PLAYLIST: &str = "index.m3u8";
const SUBTITLE_GROUP: &str = "subs";
pub(crate) const SUBTITLE_VARIANT: &str = "subs";
pub(crate) const SUBTITLE_FILE: &str = "subtitles.vtt";
const DEFAULT_BINARY: &str = "ffmpeg";
const DEFAULT_IDLE_TIMEOUT: SignedDuration = SignedDuration::from_secs(30);
const PRODUCED_WAIT: Duration = Duration::from_secs(30);
const PRODUCED_POLL: Duration = Duration::from_millis(100);
const PREPARE_LIMIT: Duration = Duration::from_secs(20);

struct Session {
    generations: HashMap<StreamGeneration, Generation>,
    last_active: Timestamp,
    start_us: Option<i64>,
}

struct Prepared {
    start_us: i64,
    known_start: Option<i64>,
    subtitles: bool,
    subtitle_file: Option<PathBuf>,
    burn: Option<Burn>,
}

struct Generation {
    registration: StreamRegistration,
    run: Option<Run>,
}

#[derive(Clone, Copy)]
struct StreamOf<'a> {
    session: &'a SessionId,
    generation: StreamGeneration,
}

#[derive(Clone, Copy)]
struct SourceOf<'a> {
    version: &'a VersionId,
    input_path: &'a str,
}

fn source_shift_ms(source: &SoftSubtitleSource, start_us: i64) -> i64 {
    match source {
        SoftSubtitleSource::Embedded(_) => start_us.div_euclid(1_000),
        SoftSubtitleSource::File(_) => 0,
    }
}

struct Run {
    container: SegmentContainer,
    copy: bool,
    segments: usize,
    opened: Instant,
    first_ready: bool,
    producer: Producer,
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

impl Producer {
    async fn halt(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for Producer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

struct Tally {
    label: &'static str,
    started: Instant,
    dir: PathBuf,
    container: SegmentContainer,
    durations: Vec<u64>,
}

impl Tally {
    fn restart(&mut self, label: &'static str) {
        self.label = label;
        self.started = Instant::now();
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        let written: Vec<u64> = self
            .durations
            .iter()
            .enumerate()
            .take_while(|(index, _)| {
                self.dir.join(segment_file_name(*index, self.container)).is_file()
            })
            .map(|(_, duration)| *duration)
            .collect();
        let segments = u32::try_from(written.len()).unwrap_or(u32::MAX);
        let media_ms = written.iter().sum();
        stream_metrics::run_ended(self.label, self.started.elapsed(), segments, media_ms);
    }
}

struct Inner<S> {
    binary: String,
    cache_root: PathBuf,
    prep_store: FsPlaybackPrepStore,
    idle_timeout: SignedDuration,
    produced_wait: Duration,
    prepare_limit: Duration,
    encoder: VideoEncoder,
    read_rate: f64,
    spawner: S,
    sessions: RwLock<HashMap<SessionId, Session>>,
    extractions: Mutex<HashMap<PathBuf, Arc<AsyncMutex<()>>>>,
}

pub struct HlsStreamSource<S: ProcessSpawner = TokioProcessSpawner> {
    inner: Arc<Inner<S>>,
}

impl<S: ProcessSpawner> Clone for HlsStreamSource<S> {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl HlsStreamSource {
    pub fn new(cache_root: impl Into<PathBuf>, prep_root: impl Into<PathBuf>) -> Self {
        Self::with_parts(cache_root, prep_root, TokioProcessSpawner)
    }
}

impl<S: ProcessSpawner + 'static> HlsStreamSource<S> {
    pub fn with_parts(
        cache_root: impl Into<PathBuf>,
        prep_root: impl Into<PathBuf>,
        spawner: S,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                binary: DEFAULT_BINARY.to_owned(),
                cache_root: cache_root.into(),
                prep_store: FsPlaybackPrepStore::new(prep_root),
                idle_timeout: DEFAULT_IDLE_TIMEOUT,
                produced_wait: PRODUCED_WAIT,
                prepare_limit: PREPARE_LIMIT,
                encoder: VideoEncoder::Software,
                read_rate: DEFAULT_READ_RATE,
                spawner,
                sessions: RwLock::new(HashMap::new()),
                extractions: Mutex::new(HashMap::new()),
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

    pub fn with_prepare_limit(mut self, prepare_limit: Duration) -> Self {
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.prepare_limit = prepare_limit;
        }
        self
    }

    fn touch_now(&self, session: &SessionId) {
        if let Some(s) = self.inner.sessions.write().unwrap().get_mut(session) {
            s.last_active = Timestamp::now();
        }
    }

    async fn retire(&self, mut entry: Generation) {
        if let Some(run) = entry.run.as_mut() {
            run.producer.halt().await;
        }
        remove_dir_off_lock(entry.registration.output_dir).await;
    }

    fn spawn_producer(
        &self,
        spec: &TranscodeSpec,
        variant_dir: &Path,
        origin_us: u64,
        start_us: i64,
        plan: &SegmentPlan,
        burn: Option<&Burn>,
    ) -> Producer {
        let out_dir = variant_dir.to_string_lossy().into_owned();
        let encoders = match spec.copy {
            true => vec![self.inner.encoder.clone()],
            false => self.inner.encoder.attempts(),
        };
        let read_rate = self.inner.read_rate;
        let segments = plan.segments.len();
        let window = RunWindow { origin_us, start_us, segments, length_ms: None };
        let attempts: Vec<(&'static str, Vec<String>)> = encoders
            .iter()
            .map(|encoder| {
                let args = build_run_args(spec, encoder, window, read_rate, &out_dir, burn);
                (stream_metrics::encoder_label(spec.copy, encoder), args)
            })
            .collect();
        let mut tally = Tally {
            label: attempts[0].0,
            started: Instant::now(),
            dir: variant_dir.to_path_buf(),
            container: spec.container,
            durations: plan.segments.iter().map(|segment| segment.duration_ms).collect(),
        };
        let first = variant_dir.join(segment_file_name(0, spec.container));
        let state = Arc::new(Mutex::new(ProducerState::default()));
        let task_state = Arc::clone(&state);
        let inner = Arc::clone(&self.inner);
        let id = format!("{}/{}", spec.session.0, spec.generation.0);
        let task = tokio::spawn(async move {
            let mut failure = None;
            for (position, &(label, ref args)) in attempts.iter().enumerate() {
                if position > 0 {
                    stream_metrics::fallback();
                    tally.restart(label);
                    let reason = failure.as_deref().unwrap_or_default();
                    let failed = attempts[position - 1].0;
                    #[rustfmt::skip]
                    tracing::warn!("stream [{id}] is run again with [{label}] after the [{failed}] run failed: {reason}");
                }
                failure = match inner.spawner.run_captured(&inner.binary, args).await {
                    Ok(output) if output.success => None,
                    Ok(output) => Some(output.failure_detail(10)),
                    Err(err) => Some(format!("could not run [{}]: {err}", inner.binary)),
                };
                let Some(detail) = &failure else {
                    break;
                };
                stream_metrics::segment_failed(label);
                tracing::warn!("producer for stream [{id}] failed: {detail}");
                if has_content(&first).await {
                    break;
                }
            }
            drop(tally);
            let mut guard = task_state.lock().unwrap();
            guard.finished = true;
            guard.failure = failure;
        });
        Producer { task: Some(task), state }
    }

    async fn stop_producers(&self, session: &SessionId) {
        let handles: Vec<tokio::task::JoinHandle<()>> = {
            let mut sessions = self.inner.sessions.write().unwrap();
            match sessions.get_mut(session) {
                Some(entry) => entry
                    .generations
                    .values_mut()
                    .filter_map(|g| g.run.as_mut())
                    .filter_map(|run| run.producer.task.take())
                    .collect(),
                None => Vec::new(),
            }
        };
        for handle in handles {
            handle.abort();
            let _ = handle.await;
        }
    }

    fn record_first(&self, session: &SessionId, generation: StreamGeneration) {
        let first = {
            let mut sessions = self.inner.sessions.write().unwrap();
            let run = sessions
                .get_mut(session)
                .and_then(|entry| entry.generations.get_mut(&generation))
                .and_then(|entry| entry.run.as_mut());
            match run {
                Some(run) if !run.first_ready => {
                    run.first_ready = true;
                    Some((run.copy, run.opened.elapsed()))
                }
                _ => None,
            }
        };
        if let Some((copy, elapsed)) = first {
            stream_metrics::first_segment(copy, elapsed);
        }
    }

    fn generation_run(
        &self,
        session: &SessionId,
        generation: StreamGeneration,
    ) -> Result<(PathBuf, SegmentContainer, usize), StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        let entry = sessions
            .get(session)
            .and_then(|s| s.generations.get(&generation))
            .ok_or(StreamError::NotLive)?;
        let run = entry.run.as_ref().ok_or(StreamError::Invalid)?;
        Ok((entry.registration.output_dir.clone(), run.container, run.segments))
    }

    fn producer_verdict(
        &self,
        session: &SessionId,
        generation: StreamGeneration,
    ) -> Option<(bool, Option<String>)> {
        let sessions = self.inner.sessions.read().unwrap();
        let run = sessions.get(session)?.generations.get(&generation)?.run.as_ref()?;
        let state = run.producer.state.lock().unwrap();
        Some((state.finished, state.failure.clone()))
    }

    async fn await_produced(
        &self,
        session: &SessionId,
        generation: StreamGeneration,
        path: &Path,
    ) -> Result<(), StreamError> {
        let wait = self.inner.produced_wait;
        let deadline = Instant::now() + wait;
        loop {
            if has_content(path).await {
                self.touch_now(session);
                return Ok(());
            }
            match self.producer_verdict(session, generation) {
                None => return Err(StreamError::NotLive),
                Some((_, Some(detail))) => {
                    let file = path.display().to_string();
                    tracing::warn!(
                        "stream [{}/{}] cannot serve [{file}] because the producer failed: \
                         {detail}",
                        session.0,
                        generation.0
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
                        "stream [{}/{}] was asked for [{file}], which the producer finished \
                         without writing",
                        session.0,
                        generation.0
                    );
                    return Err(StreamError::Invalid);
                }
                Some((false, None)) => {}
            }
            if Instant::now() >= deadline {
                let file = path.display().to_string();
                let seconds = wait.as_secs_f64();
                tracing::warn!(
                    "stream [{}/{}] gave up waiting [{seconds:.0}s] for [{file}]; the producer is \
                     still running but has not reached it",
                    session.0,
                    generation.0
                );
                return Err(StreamError::Invalid);
            }
            tokio::time::sleep(PRODUCED_POLL).await;
        }
    }

    async fn ensure_segment(
        &self,
        session: &SessionId,
        generation: StreamGeneration,
        variant: &str,
        file: &str,
    ) -> Result<(), StreamError> {
        if variant != VARIANT {
            return Err(StreamError::Invalid);
        }
        let (output_dir, container, segments) = self.generation_run(session, generation)?;
        if !segment_matches(file, container) {
            #[rustfmt::skip]
            tracing::warn!("stream [{}/{}] was asked for [{file}], which its container [{}] does not produce; the playlist and the segments have come apart", session.0, generation.0, container.as_str());
            return Err(StreamError::Invalid);
        }
        if file != INIT_SEGMENT {
            let index = parse_segment_index(file, container).ok_or(StreamError::Invalid)?;
            if index >= segments {
                #[rustfmt::skip]
                tracing::warn!("stream [{}/{}] was asked for [{file}], which is not among the [{segments}] segments of its playlist", session.0, generation.0);
                return Err(StreamError::Invalid);
            }
        }
        let mut wait = SegmentWait::started(session, generation, file);
        let produced =
            self.await_produced(session, generation, &output_dir.join(VARIANT).join(file)).await;
        wait.settled();
        produced?;
        if file != INIT_SEGMENT {
            self.record_first(session, generation);
        }
        Ok(())
    }

    async fn write_soft_subtitle(
        &self,
        output_dir: &Path,
        spec: &TranscodeSpec,
        length_ms: u64,
        start_us: i64,
        origin_ms: u64,
    ) -> (bool, Option<PathBuf>) {
        let Some(soft) = spec.soft_subtitle.as_ref() else {
            return (false, None);
        };
        let shift_ms = source_shift_ms(&soft.source, start_us);
        let stream = StreamOf { session: &spec.session, generation: spec.generation };
        let source = SourceOf { version: &spec.version, input_path: &spec.input_path };
        let written = match soft.delivery {
            SubtitleDelivery::External => {
                let origin = i64::try_from(origin_ms).unwrap_or(i64::MAX);
                let offset = soft.offset_ms.saturating_sub(shift_ms).saturating_sub(origin);
                self.write_subtitle_file(stream, source, soft, output_dir, offset).await.map(Some)
            }
            _ => {
                let offset =
                    soft.offset_ms.saturating_add(OUTPUT_OFFSET_MS as i64).saturating_sub(shift_ms);
                self.write_rendition(output_dir, stream, source, soft, length_ms, offset)
                    .await
                    .map(|()| None)
            }
        };
        match written {
            Ok(file) => (true, file),
            Err(err) => {
                #[rustfmt::skip]
                tracing::warn!("stream [{}/{}] has no subtitles: {err}", spec.session.0, spec.generation.0);
                (false, None)
            }
        }
    }

    async fn subtitle_source(
        &self,
        stream: StreamOf<'_>,
        source: SourceOf<'_>,
        soft: &SoftSubtitle,
    ) -> Result<PathBuf, TranscodeError> {
        let entry =
            self.inner.prep_store.entry(source.version, source.input_path, &soft.source).await?;
        match &soft.source {
            SoftSubtitleSource::Embedded(index) => prepared(&entry, *index).await,
            SoftSubtitleSource::File(path) => {
                self.extracted(stream, source.version, path, &entry).await
            }
        }
    }

    async fn write_subtitle_file(
        &self,
        stream: StreamOf<'_>,
        source: SourceOf<'_>,
        soft: &SoftSubtitle,
        dir: &Path,
        offset: i64,
    ) -> Result<PathBuf, TranscodeError> {
        let cached = self.subtitle_source(stream, source, soft).await?;
        let dir = dir.to_path_buf();
        spawn_blocking(move || -> std::io::Result<PathBuf> {
            std::fs::create_dir_all(&dir)?;
            let content = std::fs::read_to_string(&cached)?;
            let path = dir.join(SUBTITLE_FILE);
            std::fs::write(&path, shift(&content, offset))?;
            Ok(path)
        })
        .await
        .expect("subtitle file task panicked")
        .map_err(|e| TranscodeError::Spawn(e.to_string()))
    }

    async fn write_rendition(
        &self,
        output_dir: &Path,
        stream: StreamOf<'_>,
        source: SourceOf<'_>,
        soft: &SoftSubtitle,
        length_ms: u64,
        offset: i64,
    ) -> Result<(), TranscodeError> {
        let cached = self.subtitle_source(stream, source, soft).await?;
        let subs_dir = output_dir.join(SUBTITLE_VARIANT);
        spawn_blocking(move || -> std::io::Result<()> {
            std::fs::create_dir_all(&subs_dir)?;
            let content = std::fs::read_to_string(&cached)?;
            let cues = with_timestamp_map(&shift(&content, offset), OUTPUT_OFFSET_MS);
            std::fs::write(subs_dir.join("subs.vtt"), cues)?;
            let playlist = subtitle_media_playlist(length_ms, "subs.vtt");
            std::fs::write(subs_dir.join(MEDIA_PLAYLIST), playlist)?;
            Ok(())
        })
        .await
        .expect("subtitle post-process task panicked")
        .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        Ok(())
    }

    async fn burn_for(&self, spec: &TranscodeSpec) -> Option<Burn> {
        let wanted = spec.burn_subtitle.as_ref().filter(|_| !spec.copy)?;
        match self.try_burn_for(spec, wanted).await {
            Ok(burn) => Some(burn),
            Err(err) => {
                #[rustfmt::skip]
                tracing::warn!("stream [{}/{}] burns no subtitles: {err}", spec.session.0, spec.generation.0);
                None
            }
        }
    }

    async fn try_burn_for(
        &self,
        spec: &TranscodeSpec,
        wanted: &BurnSubtitle,
    ) -> Result<Burn, TranscodeError> {
        let index = match wanted {
            BurnSubtitle::File(path) => return Ok(Burn::Sidecar(path.clone())),
            BurnSubtitle::EmbeddedText(index) | BurnSubtitle::EmbeddedImage { index, .. } => *index,
        };
        let prefix = embedded_prefix(index);
        let entry =
            self.inner.prep_store.output(&spec.version, &spec.input_path, &prefix, "mks").await?;
        let path = prepared(&entry, index).await?.to_string_lossy().into_owned();
        match wanted {
            BurnSubtitle::EmbeddedImage { width, height, .. } => {
                Ok(Burn::Image { path, width: *width, height: *height })
            }
            _ => Ok(Burn::Text(path)),
        }
    }

    async fn start_of(&self, spec: &TranscodeSpec) -> Option<i64> {
        match self.known_start(&spec.session) {
            None => {
                let stream = StreamOf { session: &spec.session, generation: spec.generation };
                self.read_start(stream, &spec.input_path).await
            }
            known => known,
        }
    }

    fn known_start(&self, session: &SessionId) -> Option<i64> {
        self.inner.sessions.read().unwrap().get(session).and_then(|s| s.start_us)
    }

    async fn read_start(&self, stream: StreamOf<'_>, input_path: &str) -> Option<i64> {
        match film_start(&self.inner.spawner, input_path).await {
            Ok(start_us) => Some(start_us),
            Err(err) => {
                #[rustfmt::skip]
                tracing::warn!("stream [{}/{}] is timed as if its file started at zero: {err}", stream.session.0, stream.generation.0);
                None
            }
        }
    }

    async fn direct_subtitle_file(
        &self,
        spec: SubtitleFileSpec,
    ) -> Result<PathBuf, TranscodeError> {
        let stream = StreamOf { session: &spec.session, generation: spec.generation };
        let source = SourceOf { version: &spec.version, input_path: &spec.input_path };
        let start_us = match spec.subtitle.source {
            SoftSubtitleSource::Embedded(_) => {
                self.read_start(stream, &spec.input_path).await.unwrap_or(0)
            }
            SoftSubtitleSource::File(_) => 0,
        };
        let offset = spec
            .subtitle
            .offset_ms
            .saturating_sub(source_shift_ms(&spec.subtitle.source, start_us));
        let dir = self.session_dir(&spec.session).join(spec.generation.dir_name());
        self.write_subtitle_file(stream, source, &spec.subtitle, &dir, offset).await
    }

    fn extraction_lock(&self, path: &Path) -> Arc<AsyncMutex<()>> {
        self.inner.extractions.lock().unwrap().entry(path.to_path_buf()).or_default().clone()
    }

    async fn extracted(
        &self,
        stream: StreamOf<'_>,
        version: &VersionId,
        path: &str,
        entry: &PrepEntry,
    ) -> Result<PathBuf, TranscodeError> {
        if has_content(&entry.path).await {
            return Ok(entry.path.clone());
        }
        let lock = self.extraction_lock(&entry.path);
        let guard = lock.lock().await;
        let extracted = self.extract_once(stream, version, path, entry).await;
        drop(guard);
        self.inner.extractions.lock().unwrap().remove(&entry.path);
        extracted
    }

    async fn extract_once(
        &self,
        stream: StreamOf<'_>,
        version: &VersionId,
        path: &str,
        entry: &PrepEntry,
    ) -> Result<PathBuf, TranscodeError> {
        if has_content(&entry.path).await {
            return Ok(entry.path.clone());
        }
        let make = entry.dir.clone();
        spawn_blocking(move || std::fs::create_dir_all(make))
            .await
            .expect("create_dir_all task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let part = entry.dir.join(format!(
            "{}.{}-{}.part",
            entry.path.file_name().unwrap_or_default().to_string_lossy(),
            stream.session.0,
            stream.generation.0
        ));
        let mut args = vec!["-v".to_owned(), "error".to_owned()];
        args.extend(build_subtitle_extract_args(path, &part.to_string_lossy()));
        let started = Instant::now();
        let outcome = self
            .inner
            .spawner
            .run_captured(&self.inner.binary, &args)
            .await
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        if !outcome.success || !has_content(&part).await {
            remove_file_off_lock(part).await;
            let detail = outcome.failure_detail(10);
            return Err(TranscodeError::Spawn(format!("subtitle extraction failed: {detail}")));
        }
        if !rename_off_lock(part, entry.path.clone()).await {
            return Err(TranscodeError::Spawn(
                "the extracted subtitle could not be kept".to_owned(),
            ));
        }
        self.inner.prep_store.prune_siblings(entry).await;
        #[rustfmt::skip]
        tracing::info!("extracted subtitles for version [{}] in [{:.1}s]", version.0, started.elapsed().as_secs_f64());
        Ok(entry.path.clone())
    }

    async fn reap_at(&self, now: Timestamp) -> usize {
        let timeout = self.inner.idle_timeout;
        let expired: Vec<(SessionId, Session)> = {
            let mut sessions = self.inner.sessions.write().unwrap();
            let ids: Vec<SessionId> = sessions
                .iter()
                .filter(|(_, s)| {
                    s.generations.values().any(|g| g.run.is_some())
                        && is_expired(now, s.last_active, timeout)
                })
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter().filter_map(|id| sessions.remove(&id).map(|s| (id, s))).collect()
        };
        let reaped = expired.len();
        for (id, mut session) in expired {
            for run in session.generations.values_mut().filter_map(|g| g.run.as_mut()) {
                run.producer.halt().await;
            }
            remove_dir_off_lock(self.session_dir(&id)).await;
        }
        reaped
    }

    fn session_dir(&self, session: &SessionId) -> PathBuf {
        self.inner.cache_root.join(&session.0)
    }
}

impl<S: ProcessSpawner + 'static> StreamRegistry for HlsStreamSource<S> {
    fn register(
        &self,
        session: SessionId,
        generation: StreamGeneration,
        entry: StreamRegistration,
    ) {
        let mut sessions = self.inner.sessions.write().unwrap();
        let existing = sessions.entry(session).or_insert_with(|| Session {
            generations: HashMap::new(),
            last_active: Timestamp::now(),
            start_us: None,
        });
        match existing.generations.get_mut(&generation) {
            Some(current) => current.registration = entry,
            None => {
                existing
                    .generations
                    .insert(generation, Generation { registration: entry, run: None });
            }
        }
        existing.last_active = Timestamp::now();
    }

    fn remove(&self, session: &SessionId) {
        self.inner.sessions.write().unwrap().remove(session);
    }
}

impl<S: ProcessSpawner + 'static> StreamSource for HlsStreamSource<S> {
    fn master_playlist(&self, claims: &StreamClaims) -> Result<String, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        let reg = &sessions
            .get(&claims.session)
            .and_then(|s| s.generations.get(&claims.generation))
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
                .and_then(|s| s.generations.get(&claims.generation))
                .ok_or(StreamError::NotLive)?
                .registration
                .output_dir
                .clone()
        };
        if is_segment_request(file) {
            self.ensure_segment(&claims.session, claims.generation, variant, file).await?;
        }
        resolve_media_path(&base, variant, file)
    }

    fn direct_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        sessions
            .get(&claims.session)
            .and_then(|s| s.generations.get(&claims.generation))
            .ok_or(StreamError::NotLive)?
            .registration
            .direct_path
            .clone()
            .ok_or(StreamError::Invalid)
    }

    fn subtitle_file(&self, claims: &StreamClaims) -> Result<PathBuf, StreamError> {
        let sessions = self.inner.sessions.read().unwrap();
        sessions
            .get(&claims.session)
            .and_then(|s| s.generations.get(&claims.generation))
            .ok_or(StreamError::NotLive)?
            .registration
            .subtitle_file
            .clone()
            .ok_or(StreamError::Invalid)
    }
}

impl<S: ProcessSpawner + 'static> HlsStreamSource<S> {
    async fn start_stream(&self, spec: TranscodeSpec) -> Result<TranscodeStarted, TranscodeError> {
        let opened = Instant::now();
        let session = spec.session.clone();
        let generation = spec.generation;
        let session_dir = self.session_dir(&session);
        let output_dir = session_dir.join(generation.dir_name());
        let variant_dir = output_dir.join(VARIANT);
        let (origin_us, plan) = plan_run(&spec);
        let limit = self.inner.prepare_limit;
        let preparing = self.prepare(&spec, &output_dir, &variant_dir, &plan, origin_us);
        let prepared = prepared_in_time(tokio::time::timeout(limit, preparing).await, &spec, limit);
        let Prepared { start_us, known_start, subtitles, subtitle_file, burn } =
            discard_on_failure(prepared, &session_dir, &output_dir)?;
        let producer =
            self.spawn_producer(&spec, &variant_dir, origin_us, start_us, &plan, burn.as_ref());
        let run = Run {
            container: spec.container,
            copy: spec.copy,
            segments: plan.segments.len(),
            opened,
            first_ready: false,
            producer,
        };
        let stale: Vec<Generation> = {
            let mut sessions = self.inner.sessions.write().unwrap();
            let entry = sessions.entry(session.clone()).or_insert_with(|| Session {
                generations: HashMap::new(),
                last_active: Timestamp::now(),
                start_us: None,
            });
            entry.start_us = known_start;
            let registration = entry
                .generations
                .get(&generation)
                .map(|existing| existing.registration.clone())
                .unwrap_or(StreamRegistration {
                    mode: DeliveryMode::Remux,
                    output_dir: output_dir.clone(),
                    direct_path: None,
                    bandwidth: 0,
                    subtitle: None,
                    subtitle_file: None,
                });
            entry.generations.insert(
                generation,
                Generation {
                    registration: StreamRegistration {
                        output_dir: output_dir.clone(),
                        ..registration
                    },
                    run: Some(run),
                },
            );
            entry.last_active = Timestamp::now();
            let newest =
                entry.generations.keys().copied().max().expect("a generation was just inserted");
            let superseded: Vec<StreamGeneration> =
                entry.generations.keys().copied().filter(|g| *g != newest).collect();
            superseded.into_iter().filter_map(|g| entry.generations.remove(&g)).collect()
        };
        for entry in stale {
            self.retire(entry).await;
        }
        Ok(TranscodeStarted {
            session,
            output_dir: output_dir.to_string_lossy().into_owned(),
            origin_ms: origin_us / 1_000,
            sequential: true,
            subtitles,
            subtitle_file,
        })
    }

    async fn prepare(
        &self,
        spec: &TranscodeSpec,
        output_dir: &Path,
        variant_dir: &Path,
        plan: &SegmentPlan,
        origin_us: u64,
    ) -> Result<Prepared, TranscodeError> {
        let make = variant_dir.to_path_buf();
        spawn_blocking(move || std::fs::create_dir_all(make))
            .await
            .expect("create_dir_all task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let playlist = media_playlist(plan, spec.container);
        let index_path = variant_dir.join(MEDIA_PLAYLIST);
        spawn_blocking(move || std::fs::write(index_path, playlist))
            .await
            .expect("write playlist task panicked")
            .map_err(|e| TranscodeError::Spawn(e.to_string()))?;
        let known_start = self.start_of(spec).await;
        let start_us = known_start.unwrap_or(0);
        let length_ms = plan.segments.iter().map(|segment| segment.duration_ms).sum();
        let (subtitles, subtitle_file) = self
            .write_soft_subtitle(output_dir, spec, length_ms, start_us, origin_us / 1_000)
            .await;
        let burn = self.burn_for(spec).await;
        Ok(Prepared { start_us, known_start, subtitles, subtitle_file, burn })
    }
}

impl<S: ProcessSpawner + 'static> TranscodeManager for HlsStreamSource<S> {
    async fn start(&self, spec: TranscodeSpec) -> Result<TranscodeStarted, TranscodeError> {
        let copy = spec.copy;
        let started = self.start_stream(spec).await;
        stream_metrics::stream_started(copy, started.is_ok());
        started
    }

    async fn prepare_subtitle_file(
        &self,
        spec: SubtitleFileSpec,
    ) -> Result<PathBuf, TranscodeError> {
        let limit = self.inner.prepare_limit;
        match tokio::time::timeout(limit, self.direct_subtitle_file(spec)).await {
            Ok(made) => made,
            Err(_) => Err(not_ready_within(limit)),
        }
    }

    async fn touch(&self, session: &SessionId) -> Result<(), TranscodeError> {
        let mut sessions = self.inner.sessions.write().unwrap();
        match sessions.get_mut(session) {
            Some(s) if s.generations.values().any(|g| g.run.is_some()) => {
                s.last_active = Timestamp::now();
                Ok(())
            }
            _ => Err(TranscodeError::NotFound),
        }
    }

    async fn stop(&self, session: &SessionId) -> Result<(), TranscodeError> {
        self.stop_producers(session).await;
        let removed = self.inner.sessions.write().unwrap().remove(session);
        removed.ok_or(TranscodeError::NotFound)?;
        remove_dir_off_lock(self.session_dir(session)).await;
        Ok(())
    }

    async fn reap_idle(&self) -> usize {
        self.reap_at(Timestamp::now()).await
    }
}

fn plan_run(spec: &TranscodeSpec) -> (u64, SegmentPlan) {
    let seek_ms = spec.seek_ms.unwrap_or(0);
    match spec.keyframes_us.as_deref().filter(|_| spec.copy) {
        Some(keyframes) => {
            let origin_us = copy_origin(keyframes, seek_ms.saturating_mul(1_000));
            (origin_us, plan_cuts(keyframes, origin_us, spec.duration_ms, TARGET_MS))
        }
        None => {
            let full = plan_fixed_segments(spec.duration_ms, TARGET_MS);
            let first = full.index_at(seek_ms);
            if first > 0 {
                return (full.start_of(first) * 1_000, full.from_index(first));
            }
            let origin_us = spec.keyframes_us.iter().flatten().copied().min().unwrap_or(0);
            (origin_us, grid_from(origin_us / 1_000, spec.duration_ms))
        }
    }
}

fn grid_from(origin_ms: u64, duration_ms: u64) -> SegmentPlan {
    let mut plan = plan_fixed_segments(duration_ms.saturating_sub(origin_ms), TARGET_MS);
    for segment in &mut plan.segments {
        segment.start_ms += origin_ms;
    }
    plan
}

fn copy_origin(keyframes_us: &[u64], seek_us: u64) -> u64 {
    let first = keyframes_us.iter().copied().min().unwrap_or(0);
    keyframes_us.iter().copied().filter(|&k| k <= seek_us).max().unwrap_or(first)
}

async fn prepared(entry: &PrepEntry, index: u32) -> Result<PathBuf, TranscodeError> {
    if has_content(&entry.path).await {
        return Ok(entry.path.clone());
    }
    #[rustfmt::skip]
    return Err(TranscodeError::Spawn(format!("embedded track [{index}] has not been extracted; \"Re-read files\" on the title prepares it")));
}

fn prepared_in_time(
    prepared: Result<Result<Prepared, TranscodeError>, tokio::time::error::Elapsed>,
    spec: &TranscodeSpec,
    limit: Duration,
) -> Result<Prepared, TranscodeError> {
    match prepared {
        Ok(prepared) => prepared,
        Err(_) => {
            let seconds = limit.as_secs_f64();
            #[rustfmt::skip]
            tracing::warn!("stream [{}/{}] gave up after [{seconds}s]: its file or the cache did not answer in time", spec.session.0, spec.generation.0);
            Err(not_ready_within(limit))
        }
    }
}

fn not_ready_within(limit: Duration) -> TranscodeError {
    TranscodeError::Spawn(format!("not ready within [{}s]", limit.as_secs_f64()))
}

fn discard_on_failure(
    prepared: Result<Prepared, TranscodeError>,
    session_dir: &Path,
    output_dir: &Path,
) -> Result<Prepared, TranscodeError> {
    if prepared.is_ok() {
        return prepared;
    }
    let (session_dir, output_dir) = (session_dir.to_path_buf(), output_dir.to_path_buf());
    spawn_blocking(move || {
        let _ = std::fs::remove_dir_all(&output_dir);
        let _ = std::fs::remove_dir(&session_dir);
    });
    prepared
}

async fn remove_dir_off_lock(dir: PathBuf) {
    let _ = spawn_blocking(move || std::fs::remove_dir_all(&dir)).await;
}

struct SegmentWait {
    stream: String,
    file: String,
    started: Instant,
    settled: bool,
}

impl SegmentWait {
    fn started(session: &SessionId, generation: StreamGeneration, file: &str) -> Self {
        Self {
            stream: format!("{}/{}", session.0, generation.0),
            file: file.to_owned(),
            started: Instant::now(),
            settled: false,
        }
    }

    fn settled(&mut self) {
        self.settled = true;
    }
}

impl Drop for SegmentWait {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        stream_metrics::segment_abandoned();
        let seconds = self.started.elapsed().as_secs_f64();
        #[rustfmt::skip]
        tracing::warn!("[{}] of stream [{}] was abandoned after [{seconds:.1}s]: the client stopped waiting before the run wrote it", self.file, self.stream);
    }
}

pub(crate) async fn remove_file_off_lock(path: PathBuf) {
    let _ = spawn_blocking(move || std::fs::remove_file(&path)).await;
}

pub(crate) async fn rename_off_lock(from: PathBuf, to: PathBuf) -> bool {
    spawn_blocking(move || std::fs::rename(&from, &to).is_ok()).await.unwrap_or(false)
}

pub(crate) async fn has_content(path: &Path) -> bool {
    let path = path.to_path_buf();
    spawn_blocking(move || {
        std::fs::metadata(&path).map(|m| m.is_file() && m.len() > 0).unwrap_or(false)
    })
    .await
    .unwrap_or(false)
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
    file.strip_prefix("seg_")?.strip_suffix(&suffix)?.parse().ok()
}

fn is_segment_request(file: &str) -> bool {
    file.ends_with(".ts") || file.ends_with(".m4s") || file == INIT_SEGMENT
}

fn segment_matches(file: &str, container: SegmentContainer) -> bool {
    match container {
        SegmentContainer::Fmp4 => file == INIT_SEGMENT || file.ends_with(".m4s"),
        SegmentContainer::MpegTs => file.ends_with(".ts"),
    }
}

fn is_expired(now: Timestamp, last_active: Timestamp, timeout: SignedDuration) -> bool {
    now.duration_since(last_active) > timeout
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use domain::catalog::VersionId;
    use domain::process::CommandOutput;
    use domain::session::{SoftSubtitleSource, SubtitleRendition};
    use domain::user::UserId;

    const CUE: &[u8] = b"WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello\n";
    const GEN: StreamGeneration = StreamGeneration(1);
    const VAAPI: &str = "/dev/dri/renderD128";
    const SHORT_LIMIT: Duration = Duration::from_secs(1);

    struct DropFlag(Arc<AtomicUsize>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Clone, Default)]
    struct Fake {
        segments: usize,
        fail: bool,
        failing_runs: usize,
        unspawnable: bool,
        hang: bool,
        empty_init: bool,
        extract_fails: bool,
        slow_extract: bool,
        extract_hang: bool,
        film_start: Option<&'static str>,
        probe_unspawnable: bool,
        probe_hang: bool,
        runs: Arc<Mutex<Vec<Vec<String>>>>,
        extracts: Arc<AtomicUsize>,
        probes: Arc<AtomicUsize>,
        dropped: Arc<AtomicUsize>,
    }

    impl Fake {
        fn runs(&self) -> Vec<Vec<String>> {
            self.runs.lock().unwrap().clone()
        }

        fn extracts(&self) -> usize {
            self.extracts.load(Ordering::SeqCst)
        }

        fn probes(&self) -> usize {
            self.probes.load(Ordering::SeqCst)
        }

        fn dropped(&self) -> usize {
            self.dropped.load(Ordering::SeqCst)
        }

        async fn extract(&self, args: &[String]) -> std::io::Result<bool> {
            self.extracts.fetch_add(1, Ordering::SeqCst);
            if self.extract_hang {
                let _flag = DropFlag(Arc::clone(&self.dropped));
                std::future::pending::<()>().await;
            }
            if self.slow_extract {
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
            if self.extract_fails {
                return Ok(false);
            }
            std::fs::write(args.last().expect("an output path"), CUE).ok();
            Ok(true)
        }
    }

    impl ProcessSpawner for Fake {
        async fn run_captured(
            &self,
            program: &str,
            args: &[String],
        ) -> std::io::Result<CommandOutput> {
            if program != "ffprobe" {
                let success = self.run(program, args).await?;
                return Ok(CommandOutput { success, ..CommandOutput::default() });
            }
            self.probes.fetch_add(1, Ordering::SeqCst);
            if self.probe_unspawnable {
                return Err(std::io::Error::other("no such binary"));
            }
            if self.probe_hang {
                let _flag = DropFlag(Arc::clone(&self.dropped));
                std::future::pending::<()>().await;
            }
            Ok(match self.film_start {
                Some(start) => CommandOutput {
                    success: true,
                    stdout: start.to_owned(),
                    ..CommandOutput::default()
                },
                None => CommandOutput {
                    success: false,
                    stderr: "Invalid data found when processing input".to_owned(),
                    status: "exit code 1".to_owned(),
                    ..CommandOutput::default()
                },
            })
        }

        async fn run(&self, _program: &str, args: &[String]) -> std::io::Result<bool> {
            if args.iter().any(|a| a == "webvtt") {
                return self.extract(args).await;
            }
            let attempt = {
                let mut runs = self.runs.lock().unwrap();
                runs.push(args.to_vec());
                runs.len()
            };
            if self.unspawnable {
                return Err(std::io::Error::other("no such binary"));
            }
            if attempt <= self.failing_runs {
                return Ok(false);
            }
            let playlist = args.last().expect("a run is given a playlist path");
            let dir = Path::new(playlist).parent().expect("the playlist sits in its variant");
            let container = match args.iter().any(|a| a == "fmp4") {
                true => SegmentContainer::Fmp4,
                false => SegmentContainer::MpegTs,
            };
            if container == SegmentContainer::Fmp4 {
                let init: &[u8] = if self.empty_init { b"" } else { b"INIT" };
                std::fs::write(dir.join(INIT_SEGMENT), init).ok();
            }
            for index in 0..self.segments {
                std::fs::write(dir.join(segment_file_name(index, container)), b"SEGMENT").ok();
            }
            if self.hang {
                let _flag = DropFlag(Arc::clone(&self.dropped));
                std::future::pending::<()>().await;
            }
            Ok(!self.fail)
        }
    }

    fn engine(fake: Fake) -> (tempfile::TempDir, HlsStreamSource<Fake>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), prep_root(&dir), fake);
        (dir, src)
    }

    fn vaapi_engine(fake: Fake) -> (tempfile::TempDir, HlsStreamSource<Fake>) {
        let (dir, src) = engine(fake);
        (dir, src.with_encoder(VideoEncoder::VaapiEncode { device: VAAPI.to_owned() }))
    }

    fn gpu_engine(fake: Fake) -> (tempfile::TempDir, HlsStreamSource<Fake>) {
        let (dir, src) = engine(fake);
        (dir, src.with_encoder(VideoEncoder::Vaapi { device: VAAPI.to_owned(), tone_map: false }))
    }

    fn prep_root(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("_prep")
    }

    fn sid(session: &str) -> SessionId {
        SessionId(session.to_owned())
    }

    fn claims(session: &str) -> StreamClaims {
        claims_at(session, GEN)
    }

    fn claims_at(session: &str, generation: StreamGeneration) -> StreamClaims {
        StreamClaims {
            session: sid(session),
            generation,
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
            subtitle_file: None,
        }
    }

    fn direct_registration() -> StreamRegistration {
        StreamRegistration {
            mode: DeliveryMode::Direct,
            direct_path: Some(PathBuf::from("/media/movie.mkv")),
            ..transcode_registration(PathBuf::new())
        }
    }

    fn spec(id: &str, duration_ms: u64) -> TranscodeSpec {
        spec_at(id, GEN, duration_ms)
    }

    fn spec_at(id: &str, generation: StreamGeneration, duration_ms: u64) -> TranscodeSpec {
        TranscodeSpec {
            session: sid(id),
            generation,
            version: VersionId("ver-1".to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            duration_ms,
            keyframes_us: Some(Vec::new()),
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

    fn copy_spec(
        id: &str,
        container: SegmentContainer,
        duration_ms: u64,
        keyframes_ms: &[u64],
    ) -> TranscodeSpec {
        TranscodeSpec {
            copy: true,
            container,
            keyframes_us: Some(keyframes_ms.iter().map(|ms| ms * 1_000).collect()),
            ..spec(id, duration_ms)
        }
    }

    fn fmp4_spec(id: &str, duration_ms: u64) -> TranscodeSpec {
        copy_spec(id, SegmentContainer::Fmp4, duration_ms, &[0, 4_000, 8_000])
    }

    fn playlist_of(started: &TranscodeStarted) -> String {
        std::fs::read_to_string(
            PathBuf::from(&started.output_dir).join(VARIANT).join(MEDIA_PLAYLIST),
        )
        .expect("the playlist is written at start")
    }

    fn extinfs(playlist: &str) -> Vec<&str> {
        playlist.lines().filter_map(|line| line.strip_prefix("#EXTINF:")).collect()
    }

    async fn started_runs(fake: &Fake, count: usize) -> Vec<Vec<String>> {
        for _ in 0..50 {
            if fake.runs().len() >= count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        fake.runs()
    }

    fn pair_after(args: &[String], flag: &str) -> Option<String> {
        args.iter().position(|a| a == flag).map(|i| args[i + 1].clone())
    }

    async fn until_dropped(fake: &Fake) -> usize {
        for _ in 0..50 {
            if fake.dropped() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        fake.dropped()
    }

    async fn gone_soon(path: &Path) -> bool {
        tokio::time::sleep(Duration::from_millis(500)).await;
        !path.exists()
    }

    fn live_generations(src: &HlsStreamSource<Fake>, session: &str) -> Vec<StreamGeneration> {
        let sessions = src.inner.sessions.read().unwrap();
        let mut live: Vec<StreamGeneration> = sessions
            .get(&sid(session))
            .map(|s| s.generations.keys().copied().collect())
            .unwrap_or_default();
        live.sort();
        live
    }

    #[tokio::test]
    async fn master_playlist_lists_single_variant() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from("/cache/s1")));
        let master = src.master_playlist(&claims("s1")).unwrap();
        assert!(master.starts_with("#EXTM3U"));
        assert!(master.contains("#EXT-X-STREAM-INF:BANDWIDTH=4000000"));
        assert!(master.contains("v0/index.m3u8"));
        assert!(!master.contains("SUBTITLES"));
    }

    #[tokio::test]
    async fn master_playlist_includes_subtitle_rendition() {
        let (_dir, src) = engine(Fake::default());
        src.register(
            sid("s1"),
            GEN,
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
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, direct_registration());
        assert!(matches!(src.master_playlist(&claims("s1")), Err(StreamError::Invalid)));
    }

    #[tokio::test]
    async fn master_playlist_unknown_session_not_live() {
        let (_dir, src) = engine(Fake::default());
        assert!(matches!(src.master_playlist(&claims("nope")), Err(StreamError::NotLive)));
    }

    #[tokio::test]
    async fn remove_deregisters_session() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from("/cache/s1")));
        src.remove(&sid("s1"));
        assert!(matches!(src.master_playlist(&claims("s1")), Err(StreamError::NotLive)));
    }

    #[tokio::test]
    async fn direct_file_returns_path() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, direct_registration());
        assert_eq!(src.direct_file(&claims("s1")).unwrap(), PathBuf::from("/media/movie.mkv"));
    }

    #[tokio::test]
    async fn direct_file_without_path_is_invalid() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from("/cache/s1")));
        assert!(matches!(src.direct_file(&claims("s1")), Err(StreamError::Invalid)));
    }

    #[tokio::test]
    async fn direct_file_unknown_session_not_live() {
        let (_dir, src) = engine(Fake::default());
        assert!(matches!(src.direct_file(&claims("nope")), Err(StreamError::NotLive)));
    }

    #[tokio::test]
    async fn start_writes_vod_playlist_and_serves_it() {
        let (_dir, src) = engine(Fake::default());
        let started = src.start(spec("s1", 15_000)).await.expect("start");
        assert!(!started.subtitles);
        assert!(started.sequential, "every streamed generation comes from one run");
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from(&started.output_dir)));
        let index = PathBuf::from(&started.output_dir).join(VARIANT).join(MEDIA_PLAYLIST);
        let text = std::fs::read_to_string(&index).unwrap();
        assert!(text.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(text.contains("#EXT-X-ENDLIST"));
        assert!(text.contains("seg_00000.ts"));
        let path = src.media_path(&claims("s1"), VARIANT, MEDIA_PLAYLIST).await.unwrap();
        assert_eq!(path, index.canonicalize().unwrap());
    }

    #[tokio::test]
    async fn a_transcode_is_cut_on_the_fixed_grid_whatever_the_keyframes() {
        let (_dir, src) = engine(Fake::default());
        let spec = TranscodeSpec {
            keyframes_us: Some(vec![0, 5_000_000, 9_000_000]),
            ..spec("s1", 15_000)
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(extinfs(&playlist_of(&started)), ["4.000,", "4.000,", "7.000,"]);
        assert_eq!(started.origin_ms, 0);
    }

    #[tokio::test]
    async fn a_copy_is_cut_where_the_muxer_cuts_its_keyframes() {
        let (_dir, src) = engine(Fake::default());
        let spec =
            copy_spec("s1", SegmentContainer::MpegTs, 25_000, &[0, 9_000, 10_000, 11_000, 20_000]);
        let started = src.start(spec).await.expect("start");
        assert_eq!(extinfs(&playlist_of(&started)), ["9.000,", "1.000,", "10.000,", "5.000,"]);
        assert!(started.sequential);
    }

    #[tokio::test]
    async fn a_copy_without_keyframes_is_cut_on_the_fixed_grid() {
        let (_dir, src) = engine(Fake::default());
        let spec = TranscodeSpec {
            keyframes_us: None,
            ..copy_spec("s1", SegmentContainer::MpegTs, 10_000, &[])
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(extinfs(&playlist_of(&started)), ["4.000,", "6.000,"]);
    }

    #[tokio::test]
    async fn a_transcode_resumed_part_way_starts_on_the_grid_boundary_before_the_seek() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let spec = TranscodeSpec {
            seek_ms: Some(9_000),
            keyframes_us: Some(vec![0, 6_000_000, 12_000_000]),
            ..spec("s1", 16_000)
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 8_000);
        let playlist = playlist_of(&started);
        assert_eq!(extinfs(&playlist), ["4.000,", "4.000,"]);
        assert!(playlist.contains("#EXT-X-MEDIA-SEQUENCE:0\n"));
        assert!(playlist.contains("seg_00000.ts") && !playlist.contains("seg_00002.ts"));
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("5.999000"));
        assert_eq!(pair_after(&runs[0], "-vf").as_deref(), Some("trim=start=8.000000"));
    }

    #[tokio::test]
    async fn a_transcode_from_the_start_begins_at_the_first_picture() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let spec = TranscodeSpec {
            keyframes_us: Some(vec![343_222, 4_023_222, 8_023_222]),
            ..spec("s1", 12_000)
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 343);
        assert_eq!(extinfs(&playlist_of(&started)), ["4.000,", "7.657,"]);
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("0.342222"));
        assert_eq!(pair_after(&runs[0], "-vf").as_deref(), Some("trim=start=0.343222"));
        assert_eq!(
            pair_after(&runs[0], "-af").as_deref(),
            Some("atrim=start=0.343222,aformat=channel_layouts=mono|stereo|3.0|4.0|5.0|5.1|7.1")
        );
    }

    #[tokio::test]
    async fn every_run_is_timed_from_where_its_file_starts() {
        let fake = Fake { film_start: Some("9.976780\n"), ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        let spec = TranscodeSpec {
            seek_ms: Some(9_000),
            ..copy_spec("s1", SegmentContainer::MpegTs, 30_000, &[23, 2_023, 4_023, 6_023, 8_023])
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 8_023);
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("17.998780"));
        assert_eq!(pair_after(&runs[0], "-output_ts_offset").as_deref(), Some("-8.976780"));
    }

    #[tokio::test]
    async fn a_file_whose_start_cannot_be_read_is_timed_from_zero() {
        for fake in [Fake::default(), Fake { probe_unspawnable: true, ..Fake::default() }] {
            let (_dir, src) = engine(fake.clone());
            src.start(spec("s1", 8_000)).await.expect("start");
            let runs = started_runs(&fake, 1).await;
            assert_eq!(pair_after(&runs[0], "-output_ts_offset").as_deref(), Some("1.000000"));
        }
    }

    #[tokio::test]
    async fn a_later_generation_is_timed_from_the_start_the_first_one_read() {
        let fake = Fake { film_start: Some("1.400000\n"), ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.start(spec_at("s1", StreamGeneration(2), 8_000)).await.expect("renegotiate");
        let runs = started_runs(&fake, 2).await;
        assert_eq!(fake.probes(), 1, "the second generation probed the file again");
        assert_eq!(pair_after(&runs[0], "-output_ts_offset").as_deref(), Some("-0.400000"));
        assert_eq!(pair_after(&runs[1], "-output_ts_offset").as_deref(), Some("-0.400000"));
    }

    #[tokio::test]
    async fn a_start_that_could_not_be_read_is_read_again_by_the_next_generation() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.start(spec_at("s1", StreamGeneration(2), 8_000)).await.expect("renegotiate");
        assert_eq!(fake.probes(), 2, "a failed read was kept for the session");
    }

    #[tokio::test]
    async fn a_session_that_went_direct_reads_its_start_again() {
        let fake = Fake { film_start: Some("1.400000\n"), ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.stop(&sid("s1")).await.expect("stop");
        src.register(sid("s1"), StreamGeneration(2), direct_registration());
        src.start(spec_at("s1", StreamGeneration(3), 8_000)).await.expect("renegotiate");
        assert_eq!(fake.probes(), 2);
    }

    #[tokio::test]
    async fn a_copy_resumed_part_way_starts_on_the_keyframe_before_the_seek() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let keyframes = [0, 4_000, 9_000, 13_000];
        let spec = TranscodeSpec {
            seek_ms: Some(9_500),
            ..copy_spec("s1", SegmentContainer::MpegTs, 16_000, &keyframes)
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 9_000);
        assert_eq!(extinfs(&playlist_of(&started)), ["4.000,", "3.000,"]);
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("8.999000"));
        assert_eq!(pair_after(&runs[0], "-copypriorss").as_deref(), Some("0"));
    }

    #[tokio::test]
    async fn a_copy_resumed_on_a_keyframe_between_milliseconds_seeks_to_its_microsecond() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let spec = TranscodeSpec {
            seek_ms: Some(9_000),
            keyframes_us: Some(vec![0, 4_004_000, 8_008_333]),
            ..copy_spec("s1", SegmentContainer::MpegTs, 16_000, &[])
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 8_008);
        assert_eq!(extinfs(&playlist_of(&started)), ["7.992,"]);
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("8.007333"));
    }

    #[tokio::test]
    async fn a_copy_resumed_inside_its_first_gop_starts_at_the_first_keyframe() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let spec = TranscodeSpec {
            seek_ms: Some(2_000),
            ..copy_spec("s1", SegmentContainer::MpegTs, 12_000, &[21, 4_021, 8_021])
        };
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 21);
        assert_eq!(extinfs(&playlist_of(&started)), ["4.000,", "4.000,", "3.979,"]);
        let runs = started_runs(&fake, 1).await;
        assert_eq!(pair_after(&runs[0], "-ss").as_deref(), Some("0.020000"));
        assert_eq!(pair_after(&runs[0], "-copypriorss").as_deref(), Some("0"));
    }

    #[tokio::test]
    async fn a_copy_whose_first_keyframe_opens_the_file_neither_seeks_nor_drops() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let spec = copy_spec("s1", SegmentContainer::MpegTs, 12_000, &[0, 4_000, 8_000]);
        let started = src.start(spec).await.expect("start");
        assert_eq!(started.origin_ms, 0);
        let runs = started_runs(&fake, 1).await;
        assert!(!runs[0].iter().any(|a| a == "-ss" || a == "-copypriorss"));
    }

    #[tokio::test]
    async fn start_create_dir_failure_maps_to_error() {
        let file = tempfile::NamedTempFile::new().expect("tempfile");
        let src = HlsStreamSource::with_parts(file.path(), file.path(), Fake::default());
        let err = src.start(spec("s1", 1_000)).await.unwrap_err();
        assert!(matches!(err, TranscodeError::Spawn(_)));
    }

    #[tokio::test]
    async fn a_probe_that_never_answers_fails_the_start_within_the_limit() {
        let fake = Fake { probe_hang: true, ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        let src = src.with_prepare_limit(SHORT_LIMIT);
        let err = src.start(spec("s1", 8_000)).await.unwrap_err();
        assert!(matches!(&err, TranscodeError::Spawn(detail) if detail == "not ready within [1s]"));
        assert!(live_generations(&src, "s1").is_empty());
        assert!(fake.runs().is_empty());
        assert_eq!(fake.dropped(), 1, "the probe was left running");
        assert!(gone_soon(&dir.path().join("s1")).await, "the start left its cache behind");
    }

    #[tokio::test]
    async fn an_extraction_that_never_answers_fails_the_start_within_the_limit() {
        let fake = Fake { extract_hang: true, ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        let src = src.with_prepare_limit(SHORT_LIMIT);
        let err = src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.unwrap_err();
        assert!(matches!(err, TranscodeError::Spawn(_)));
        assert!(live_generations(&src, "s1").is_empty());
        assert!(fake.runs().is_empty());
        assert_eq!(fake.dropped(), 1, "the extraction was left running");
        assert!(gone_soon(&dir.path().join("s1")).await, "the start left its cache behind");
    }

    #[tokio::test]
    async fn a_renegotiation_that_runs_out_of_time_leaves_the_earlier_generation_playing() {
        let fake = Fake { extract_hang: true, hang: true, ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        let src = src.with_prepare_limit(SHORT_LIMIT);
        src.start(spec("s1", 8_000)).await.expect("start");
        let late = with_subtitle(spec_at("s1", StreamGeneration(2), 8_000), soft(0));
        assert!(src.start(late).await.is_err());
        assert_eq!(live_generations(&src, "s1"), vec![GEN]);
        assert!(src.media_path(&claims("s1"), VARIANT, MEDIA_PLAYLIST).await.is_ok());
        assert_eq!(started_runs(&fake, 1).await.len(), 1);
        assert_eq!(fake.dropped(), 1, "the earlier run was stopped or the extraction left");
        let session = dir.path().join("s1");
        assert!(gone_soon(&session.join(StreamGeneration(2).dir_name())).await);
        assert!(session.join(GEN.dir_name()).join(VARIANT).join(MEDIA_PLAYLIST).is_file());
    }

    #[tokio::test]
    async fn renegotiating_replaces_the_prior_generation() {
        let (_dir, src) = engine(Fake::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.start(spec_at("s1", StreamGeneration(2), 8_000)).await.expect("renegotiate");
        let sessions = src.inner.sessions.read().unwrap();
        assert_eq!(sessions.len(), 1);
        let session = sessions.get(&sid("s1")).unwrap();
        assert_eq!(session.generations.len(), 1);
        assert!(session.generations.get(&StreamGeneration(2)).unwrap().run.is_some());
    }

    fn extracted_files(dir: &tempfile::TempDir) -> Vec<PathBuf> {
        let owned = prep_root(dir).join("ver-1");
        let Ok(entries) = std::fs::read_dir(owned) else {
            return Vec::new();
        };
        let mut found: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        found.sort();
        found
    }

    fn soft(offset_ms: i64) -> SoftSubtitle {
        SoftSubtitle {
            source: SoftSubtitleSource::File("/media/movie.en.srt".to_owned()),
            offset_ms,
            delivery: SubtitleDelivery::HlsVtt,
        }
    }

    fn embedded(offset_ms: i64) -> SoftSubtitle {
        SoftSubtitle {
            source: SoftSubtitleSource::Embedded(3),
            offset_ms,
            delivery: SubtitleDelivery::HlsVtt,
        }
    }

    fn as_file(subtitle: SoftSubtitle) -> SoftSubtitle {
        SoftSubtitle { delivery: SubtitleDelivery::External, ..subtitle }
    }

    fn direct_file_spec(session: &str, subtitle: SoftSubtitle) -> SubtitleFileSpec {
        SubtitleFileSpec {
            session: sid(session),
            generation: GEN,
            version: VersionId("ver-1".to_owned()),
            input_path: "/media/movie.mkv".to_owned(),
            subtitle,
        }
    }

    fn with_subtitle(spec: TranscodeSpec, subtitle: SoftSubtitle) -> TranscodeSpec {
        TranscodeSpec { soft_subtitle: Some(subtitle), ..spec }
    }

    fn cues(output_dir: &str) -> String {
        std::fs::read_to_string(PathBuf::from(output_dir).join("subs").join("subs.vtt"))
            .expect("subs.vtt")
    }

    async fn prepare_track(dir: &tempfile::TempDir) {
        let entry = FsPlaybackPrepStore::new(prep_root(dir))
            .entry(&VersionId("ver-1".to_owned()), "/media/movie.mkv", &embedded(0).source)
            .await
            .expect("entry");
        std::fs::create_dir_all(&entry.dir).expect("dir");
        std::fs::write(&entry.path, CUE).expect("write");
    }

    #[tokio::test]
    async fn start_writes_subtitle_rendition() {
        let (_dir, src) = engine(Fake::default());
        let started = src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.expect("start");
        assert!(started.subtitles);
        let subs = PathBuf::from(&started.output_dir).join("subs");
        assert!(subs.join("subs.vtt").exists());
        let playlist = std::fs::read_to_string(subs.join(MEDIA_PLAYLIST)).unwrap();
        assert!(playlist.contains("subs.vtt"));
        assert!(playlist.contains("#EXT-X-ENDLIST"));
    }

    #[tokio::test]
    async fn the_cues_sit_on_the_stream_clock_and_say_so_in_their_header() {
        let (_dir, src) = engine(Fake::default());
        let started = src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.expect("start");
        assert_eq!(
            cues(&started.output_dir),
            "WEBVTT\nX-TIMESTAMP-MAP=MPEGTS:90000,LOCAL:00:00:01.000\n\n00:00:02.000 --> 00:00:03.000\nHello\n",
            "a cue at 1 s of the film plays at 2 s of a stream that carries a 1 s offset"
        );
    }

    #[tokio::test]
    async fn a_rendition_lasts_as_long_as_its_generation() {
        let (_dir, src) = engine(Fake::default());
        let resumed = TranscodeSpec { seek_ms: Some(9_000), ..spec("s1", 10_000) };
        let started = src.start(with_subtitle(resumed, soft(0))).await.expect("start");
        let playlist = std::fs::read_to_string(
            PathBuf::from(&started.output_dir).join("subs").join(MEDIA_PLAYLIST),
        )
        .expect("playlist");
        assert!(playlist.contains("#EXTINF:6.000,\nsubs.vtt\n"), "{playlist}");
    }

    #[tokio::test]
    async fn start_subtitle_failure_is_best_effort() {
        let (dir, src) = engine(Fake { extract_fails: true, ..Fake::default() });
        let started = src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.expect("start");
        assert!(!started.subtitles);
        assert!(!PathBuf::from(&started.output_dir).join("subs").join(MEDIA_PLAYLIST).exists());
        assert!(extracted_files(&dir).is_empty(), "a failed extraction must cache nothing");
    }

    #[tokio::test]
    async fn an_embedded_track_is_served_from_its_preparation_without_reading_the_file() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        prepare_track(&dir).await;
        let started =
            src.start(with_subtitle(spec("s1", 10_000), embedded(2_000))).await.expect("start");
        assert!(started.subtitles);
        assert_eq!(fake.extracts(), 0);
        assert!(cues(&started.output_dir).contains("00:00:04.000"));
    }

    #[tokio::test]
    async fn an_embedded_track_moves_from_the_file_clock_and_a_sidecar_from_the_film_clock() {
        let fake = Fake { film_start: Some("1.400000"), ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        prepare_track(&dir).await;
        let copied =
            src.start(with_subtitle(spec("s1", 10_000), embedded(0))).await.expect("start");
        assert!(cues(&copied.output_dir).contains("00:00:00.600 --> 00:00:01.600"));
        let sidecar = with_subtitle(spec("s2", 10_000), soft(0));
        let started = src.start(sidecar).await.expect("start");
        assert!(cues(&started.output_dir).contains("00:00:02.000 --> 00:00:03.000"));
    }

    #[tokio::test]
    async fn an_embedded_track_not_prepared_yet_starts_without_subtitles_or_a_read() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        let started =
            src.start(with_subtitle(spec("s1", 10_000), embedded(0))).await.expect("start");
        assert!(!started.subtitles, "a rendition that was not written is not advertised");
        assert_eq!(fake.extracts(), 0);
        assert!(!PathBuf::from(&started.output_dir).join("subs").join(MEDIA_PLAYLIST).exists());
        assert!(extracted_files(&dir).is_empty());
    }

    #[tokio::test]
    async fn a_second_generation_reads_the_extraction_the_first_one_paid_for() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        let one = src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.expect("start");
        assert!(cues(&one.output_dir).contains("00:00:02.000"));
        let second = with_subtitle(spec_at("s1", StreamGeneration(2), 10_000), soft(2_000));
        let two = src.start(second).await.expect("renegotiate");

        assert_eq!(fake.extracts(), 1, "the second generation re-extracted");
        assert_eq!(extracted_files(&dir).len(), 1);
        assert!(cues(&two.output_dir).contains("00:00:04.000"), "the offset was not applied");
        let cached = std::fs::read_to_string(&extracted_files(&dir)[0]).expect("cached");
        assert!(cached.contains("00:00:01.000"), "the cache must hold unshifted cues");
    }

    #[tokio::test]
    async fn renegotiating_mid_extraction_waits_for_it_instead_of_starting_a_second_one() {
        let fake = Fake { slow_extract: true, ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        let first = with_subtitle(spec("s1", 10_000), soft(0));
        let second = with_subtitle(spec_at("s1", StreamGeneration(2), 10_000), soft(2_000));

        let (one, two) = tokio::join!(src.start(first), src.start(second));
        one.expect("start");
        two.expect("renegotiate");

        assert_eq!(fake.extracts(), 1, "both generations extracted");
        assert_eq!(extracted_files(&dir).len(), 1);
        assert!(src.inner.extractions.lock().unwrap().is_empty(), "the lock outlived the work");
    }

    #[tokio::test]
    async fn a_stream_for_a_player_writes_a_plain_subtitle_file_and_no_rendition() {
        let (_dir, src) = engine(Fake::default());
        let started =
            src.start(with_subtitle(spec("s1", 10_000), as_file(soft(0)))).await.expect("start");
        let file = PathBuf::from(&started.output_dir).join(SUBTITLE_FILE);
        assert!(started.subtitles);
        assert_eq!(started.subtitle_file.as_deref(), Some(file.as_path()));
        assert_eq!(
            std::fs::read_to_string(&file).expect("subtitle file"),
            "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello\n",
            "a player times a separate file on its own clock, which starts where the stream does"
        );
        assert!(!PathBuf::from(&started.output_dir).join("subs").exists());
    }

    #[tokio::test]
    async fn a_resumed_stream_moves_its_subtitle_file_back_by_the_stream_start() {
        let (_dir, src) = engine(Fake::default());
        let resumed = TranscodeSpec { seek_ms: Some(4_000), ..spec("s1", 10_000) };
        let started = src.start(with_subtitle(resumed, as_file(soft(6_000)))).await.expect("start");
        assert_eq!(started.origin_ms, 4_000);
        let file = started.subtitle_file.expect("a subtitle file");
        assert!(
            std::fs::read_to_string(file).expect("file").contains("00:00:03.000 --> 00:00:04.000")
        );
    }

    #[tokio::test]
    async fn a_failed_subtitle_file_leaves_the_stream_without_one() {
        let (_dir, src) = engine(Fake { extract_fails: true, ..Fake::default() });
        let started =
            src.start(with_subtitle(spec("s1", 10_000), as_file(soft(0)))).await.expect("start");
        assert!(!started.subtitles);
        assert_eq!(started.subtitle_file, None);
    }

    #[tokio::test]
    async fn direct_play_gets_its_subtitle_file_in_its_generation_on_the_film_clock() {
        let fake = Fake { film_start: Some("1.400000"), ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        prepare_track(&dir).await;

        let sidecar = src.prepare_subtitle_file(direct_file_spec("s1", as_file(soft(500)))).await;
        let sidecar = sidecar.expect("sidecar file");
        assert_eq!(sidecar, src.session_dir(&sid("s1")).join(GEN.dir_name()).join(SUBTITLE_FILE));
        assert!(
            std::fs::read_to_string(&sidecar).unwrap().contains("00:00:01.500 --> 00:00:02.500")
        );

        let copied =
            src.prepare_subtitle_file(direct_file_spec("s2", as_file(embedded(2_000)))).await;
        let copied = std::fs::read_to_string(copied.expect("embedded file")).unwrap();
        assert!(copied.contains("00:00:01.600 --> 00:00:02.600"), "{copied}");
        assert!(!copied.contains("X-TIMESTAMP-MAP"));
    }

    #[tokio::test]
    async fn a_direct_subtitle_file_that_cannot_be_made_is_an_error() {
        let (_dir, src) = engine(Fake { extract_fails: true, ..Fake::default() });
        let file = src.prepare_subtitle_file(direct_file_spec("s1", as_file(soft(0)))).await;
        assert!(file.is_err());
        let (_dir, src) = engine(Fake::default());
        let unprepared =
            src.prepare_subtitle_file(direct_file_spec("s1", as_file(embedded(0)))).await;
        assert!(unprepared.is_err(), "an embedded track that was never prepared has no file");
    }

    #[tokio::test]
    async fn a_direct_subtitle_file_whose_source_never_answers_gives_up_within_the_limit() {
        let fake = Fake { extract_hang: true, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        let src = src.with_prepare_limit(SHORT_LIMIT);
        let sidecar = src.prepare_subtitle_file(direct_file_spec("s1", as_file(soft(0)))).await;
        assert!(
            matches!(&sidecar, Err(TranscodeError::Spawn(detail)) if detail == "not ready within [1s]")
        );
        assert_eq!(fake.dropped(), 1, "the extraction was left running");

        let fake = Fake { probe_hang: true, ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        prepare_track(&dir).await;
        let src = src.with_prepare_limit(SHORT_LIMIT);
        let embedded =
            src.prepare_subtitle_file(direct_file_spec("s2", as_file(embedded(0)))).await;
        assert!(matches!(embedded, Err(TranscodeError::Spawn(_))));
        assert_eq!(fake.dropped(), 1, "the probe was left running");
    }

    #[test]
    fn the_subtitle_file_is_served_only_for_a_registration_that_has_one() {
        let (_dir, src) = engine(Fake::default());
        assert!(matches!(src.subtitle_file(&claims("s1")), Err(StreamError::NotLive)));
        src.register(sid("s1"), GEN, direct_registration());
        assert!(matches!(src.subtitle_file(&claims("s1")), Err(StreamError::Invalid)));
        let file = PathBuf::from("/cache/s1/1/subtitles.vtt");
        src.register(
            sid("s1"),
            GEN,
            StreamRegistration { subtitle_file: Some(file.clone()), ..direct_registration() },
        );
        assert_eq!(src.subtitle_file(&claims("s1")).unwrap(), file);
    }

    #[tokio::test]
    async fn a_second_session_on_the_same_version_reads_the_same_extraction() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        for session in ["s1", "s2"] {
            src.start(with_subtitle(spec(session, 10_000), soft(0))).await.expect("start");
        }
        assert_eq!(fake.extracts(), 1);
        assert_eq!(extracted_files(&dir).len(), 1);
    }

    #[tokio::test]
    async fn an_extraction_that_cannot_be_kept_is_reported_rather_than_served() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        src.start(with_subtitle(spec("s1", 10_000), soft(0))).await.expect("start");
        let cached = extracted_files(&dir).remove(0);
        std::fs::remove_file(&cached).expect("remove");
        std::fs::create_dir(&cached).expect("dir");
        std::fs::write(cached.join("occupied"), b"x").expect("write");

        let started = src.start(with_subtitle(spec("s2", 10_000), soft(0))).await.expect("start");

        assert_eq!(fake.extracts(), 2);
        assert!(!PathBuf::from(&started.output_dir).join("subs").join(MEDIA_PLAYLIST).exists());
    }

    async fn prepare_copy(dir: &tempfile::TempDir, index: u32) -> String {
        let entry = FsPlaybackPrepStore::new(prep_root(dir))
            .output(
                &VersionId("ver-1".to_owned()),
                "/media/movie.mkv",
                &embedded_prefix(index),
                "mks",
            )
            .await
            .expect("entry");
        std::fs::create_dir_all(&entry.dir).expect("dir");
        std::fs::write(&entry.path, b"MKS").expect("write");
        entry.path.to_string_lossy().into_owned()
    }

    fn burning(burn: BurnSubtitle) -> TranscodeSpec {
        TranscodeSpec { burn_subtitle: Some(burn), ..spec("s1", 12_000) }
    }

    fn image_burn() -> BurnSubtitle {
        BurnSubtitle::EmbeddedImage { index: 3, width: 1920, height: 800 }
    }

    fn input_files(args: &[String]) -> Vec<&str> {
        args.iter()
            .enumerate()
            .filter(|(_, a)| *a == "-i")
            .map(|(i, _)| args[i + 1].as_str())
            .collect()
    }

    #[tokio::test]
    async fn an_embedded_image_burn_overlays_its_prepared_copy_on_the_file_clock() {
        let fake = Fake { film_start: Some("1.400000\n"), ..Fake::default() };
        let (dir, src) = engine(fake.clone());
        let copy = prepare_copy(&dir, 3).await;
        src.start(burning(image_burn())).await.expect("start");
        let runs = started_runs(&fake, 1).await;
        assert_eq!(input_files(&runs[0]), vec!["/media/movie.mkv", copy.as_str()]);
        assert!(!runs[0].iter().any(|a| a == "-itsoffset"));
        let graph = pair_after(&runs[0], "-filter_complex").expect("an overlay");
        assert!(graph.contains("[1:s:0]scale=1920:800[sub]"), "{graph}");
        let copy_at = runs[0].iter().position(|a| a == copy.as_str()).unwrap();
        assert_eq!(runs[0][copy_at - 2], "1.400000", "the copy is read from the film's start");
    }

    #[tokio::test]
    async fn a_hardware_run_that_falls_back_keeps_its_burn() {
        let fake = Fake { failing_runs: 1, ..Fake::default() };
        let (dir, src) = vaapi_engine(fake.clone());
        prepare_copy(&dir, 3).await;
        src.start(burning(image_burn())).await.expect("start");
        let runs = started_runs(&fake, 2).await;
        assert_eq!(runs.len(), 2);
        assert!(runs.iter().all(|args| args.iter().any(|a| a == "-filter_complex")));
    }

    #[tokio::test]
    async fn each_gpu_fallback_keeps_its_burn() {
        let fake = Fake { failing_runs: 2, ..Fake::default() };
        let (dir, src) = gpu_engine(fake.clone());
        prepare_copy(&dir, 3).await;
        src.start(burning(image_burn())).await.expect("start");
        let runs = started_runs(&fake, 3).await;
        assert_eq!(runs.len(), 3);
        assert!(runs.iter().all(|args| args.iter().any(|a| a == "-filter_complex")));
        assert_eq!(runs.iter().filter(|args| args.iter().any(|a| a == "-hwaccel")).count(), 1);
    }

    #[tokio::test]
    async fn an_embedded_text_burn_draws_its_prepared_copy_instead_of_the_film() {
        let fake = Fake::default();
        let (dir, src) = engine(fake.clone());
        let copy = prepare_copy(&dir, 3).await;
        src.start(burning(BurnSubtitle::EmbeddedText(3))).await.expect("start");
        let runs = started_runs(&fake, 1).await;
        assert_eq!(input_files(&runs[0]), vec!["/media/movie.mkv"]);
        assert_eq!(pair_after(&runs[0], "-vf"), Some(format!("subtitles=filename='{copy}'")));
    }

    #[tokio::test]
    async fn a_sidecar_burn_draws_the_sidecar_file() {
        let fake = Fake::default();
        let (_dir, src) = engine(fake.clone());
        let burn = BurnSubtitle::File("/media/movie.en.srt".to_owned());
        src.start(burning(burn)).await.expect("start");
        let runs = started_runs(&fake, 1).await;
        assert_eq!(
            pair_after(&runs[0], "-vf").as_deref(),
            Some("subtitles=filename='/media/movie.en.srt'")
        );
    }

    #[tokio::test]
    async fn a_burn_that_cannot_be_prepared_leaves_the_run_without_it() {
        for burn in [image_burn(), BurnSubtitle::EmbeddedText(3)] {
            let fake = Fake::default();
            let (_dir, src) = engine(fake.clone());
            src.start(burning(burn)).await.expect("start");
            let runs = started_runs(&fake, 1).await;
            assert_eq!(input_files(&runs[0]), vec!["/media/movie.mkv"]);
            assert!(!runs[0].iter().any(|a| a == "-vf" || a == "-filter_complex"), "{:?}", runs[0]);
        }
    }

    #[tokio::test]
    async fn a_segment_is_served_once_the_run_has_written_it() {
        let fake = Fake { segments: 3, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from(&started.output_dir)));
        let path = src.media_path(&claims("s1"), VARIANT, "seg_00001.ts").await.unwrap();
        assert!(path.is_file());
        src.media_path(&claims("s1"), VARIANT, "seg_00001.ts").await.unwrap();
        assert_eq!(fake.runs().len(), 1, "one run writes every segment");
    }

    #[tokio::test]
    async fn an_fmp4_session_serves_a_playlist_that_maps_the_init_segment() {
        let (_dir, src) = engine(Fake { segments: 3, ..Fake::default() });
        let started = src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        let playlist = playlist_of(&started);
        assert!(playlist.contains("#EXT-X-VERSION:7"));
        assert!(playlist.contains("#EXT-X-MAP:URI=\"init.mp4\""));
        assert!(playlist.contains("seg_00000.m4s"));
        let segment = src.media_path(&claims("s1"), VARIANT, "seg_00000.m4s").await.unwrap();
        assert!(segment.is_file());
        let init = src.media_path(&claims("s1"), VARIANT, INIT_SEGMENT).await.unwrap();
        assert!(init.is_file());
    }

    #[tokio::test]
    async fn a_segment_outside_the_playlist_is_refused_without_waiting() {
        let (_dir, src) = engine(Fake { hang: true, ..Fake::default() });
        src.start(spec("s1", 8_000)).await.expect("start");
        let verdict = tokio::time::timeout(
            Duration::from_millis(500),
            src.media_path(&claims("s1"), VARIANT, "seg_00002.ts"),
        )
        .await
        .expect("a segment the playlist does not list is refused at once");
        assert!(matches!(verdict, Err(StreamError::Invalid)));
    }

    #[tokio::test]
    async fn segment_bad_name_is_invalid() {
        let (_dir, src) = engine(Fake::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        for file in ["seg_x.ts", "seg_.ts", "foo.ts"] {
            assert!(matches!(
                src.media_path(&claims("s1"), VARIANT, file).await,
                Err(StreamError::Invalid)
            ));
        }
    }

    #[tokio::test]
    async fn ensure_segment_rejects_wrong_variant_and_unknown_session() {
        let (_dir, src) = engine(Fake::default());
        assert!(matches!(
            src.ensure_segment(&sid("s1"), GEN, "subs", "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
        assert!(matches!(
            src.ensure_segment(&sid("nope"), GEN, VARIANT, "seg_00000.ts").await,
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn a_segment_of_a_generation_without_a_run_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(VARIANT)).unwrap();
        let (_cache, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(dir.path().to_path_buf()));
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn a_segment_is_invalid_when_the_run_fails_or_cannot_start() {
        for fake in
            [Fake { fail: true, ..Fake::default() }, Fake { unspawnable: true, ..Fake::default() }]
        {
            let (_dir, src) = engine(fake);
            src.start(spec("s1", 8_000)).await.expect("start");
            assert!(
                matches!(
                    src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await,
                    Err(StreamError::Invalid)
                ),
                "a failed run must fail the request at once, not wait out the clock"
            );
        }
    }

    #[tokio::test]
    async fn a_segment_the_run_finished_without_writing_is_invalid() {
        let (_dir, src) = engine(Fake { segments: 1, ..Fake::default() });
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "seg_00002.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn an_init_segment_the_run_has_only_created_is_never_served() {
        let (_dir, src) = engine(Fake { empty_init: true, segments: 1, ..Fake::default() });
        let src = src.with_produced_wait(Duration::from_millis(250));
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, INIT_SEGMENT).await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn a_segment_gives_up_when_the_run_never_reaches_it() {
        let (_dir, src) = engine(Fake { hang: true, ..Fake::default() });
        let src = src.with_produced_wait(Duration::from_millis(250));
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn a_segment_is_not_live_once_its_session_is_gone() {
        let (_dir, src) = engine(Fake { hang: true, ..Fake::default() });
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.remove(&sid("s1"));
        let path = PathBuf::from(&started.output_dir).join(VARIANT).join("seg_00000.ts");
        assert!(matches!(
            src.await_produced(&sid("s1"), GEN, &path).await,
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn an_fmp4_generation_refuses_a_mpegts_segment_without_waiting_for_it() {
        let (_dir, src) = engine(Fake { hang: true, ..Fake::default() });
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        let verdict = tokio::time::timeout(
            Duration::from_millis(500),
            src.media_path(&claims("s1"), VARIANT, "seg_00000.ts"),
        )
        .await
        .expect("the mismatch is answered without waiting for the run");
        assert!(matches!(verdict, Err(StreamError::Invalid)));
    }

    #[tokio::test]
    async fn a_mpegts_generation_refuses_an_fmp4_segment() {
        let (_dir, src) = engine(Fake::default());
        src.start(spec("s1", 12_000)).await.expect("start");
        for file in ["seg_00000.m4s", INIT_SEGMENT] {
            assert!(matches!(
                src.media_path(&claims("s1"), VARIANT, file).await,
                Err(StreamError::Invalid)
            ));
        }
    }

    #[test]
    fn a_segment_matches_only_the_container_that_produces_it() {
        assert!(segment_matches("seg_00000.ts", SegmentContainer::MpegTs));
        assert!(!segment_matches("seg_00000.m4s", SegmentContainer::MpegTs));
        assert!(!segment_matches(INIT_SEGMENT, SegmentContainer::MpegTs));
        assert!(segment_matches("seg_00000.m4s", SegmentContainer::Fmp4));
        assert!(segment_matches(INIT_SEGMENT, SegmentContainer::Fmp4));
        assert!(!segment_matches("seg_00000.ts", SegmentContainer::Fmp4));
    }

    #[tokio::test]
    async fn the_media_playlist_resolves_without_waiting_for_the_run() {
        let (_dir, src) = engine(Fake { hang: true, ..Fake::default() });
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        let verdict = tokio::time::timeout(
            Duration::from_millis(500),
            src.media_path(&claims("s1"), VARIANT, MEDIA_PLAYLIST),
        )
        .await
        .expect("the playlist is written before the run starts");
        assert!(
            verdict
                .unwrap()
                .starts_with(PathBuf::from(&started.output_dir).canonicalize().unwrap())
        );
    }

    #[test]
    fn an_unsettled_segment_wait_reports_itself_on_drop() {
        drop(SegmentWait::started(&sid("s1"), GEN, "seg_00003.ts"));
        let mut settled = SegmentWait::started(&sid("s1"), GEN, "seg_00004.ts");
        settled.settled();
        drop(settled);
    }

    #[tokio::test]
    async fn a_hardware_run_that_fails_before_its_first_segment_is_rerun_in_software() {
        let fake = Fake { segments: 3, failing_runs: 1, ..Fake::default() };
        let (_dir, src) = vaapi_engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await.is_ok());
        let runs = fake.runs();
        assert_eq!(runs.len(), 2);
        assert_eq!(pair_after(&runs[0], "-c:v").as_deref(), Some("h264_vaapi"));
        assert_eq!(pair_after(&runs[1], "-c:v").as_deref(), Some("libx264"));
    }

    #[tokio::test]
    async fn a_gpu_run_that_fails_falls_back_to_the_gpu_encoder_then_to_software() {
        let fake = Fake { segments: 3, failing_runs: 2, ..Fake::default() };
        let (_dir, src) = gpu_engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await.is_ok());
        let runs = fake.runs();
        assert_eq!(runs.len(), 3);
        assert!(runs[0].iter().any(|a| a == "-hwaccel"));
        assert!(!runs[1].iter().any(|a| a == "-hwaccel"));
        let codecs: Vec<String> = runs.iter().filter_map(|run| pair_after(run, "-c:v")).collect();
        assert_eq!(codecs, ["h264_vaapi", "h264_vaapi", "libx264"]);
    }

    #[tokio::test]
    async fn a_gpu_run_that_works_needs_no_other() {
        let fake = Fake { segments: 3, ..Fake::default() };
        let (_dir, src) = gpu_engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await.is_ok());
        assert_eq!(fake.runs().len(), 1);
    }

    #[tokio::test]
    async fn a_hardware_run_that_fails_after_writing_is_not_rerun() {
        let fake = Fake { segments: 1, fail: true, ..Fake::default() };
        let (_dir, src) = vaapi_engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        assert!(src.media_path(&claims("s1"), VARIANT, "seg_00000.ts").await.is_ok());
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "seg_00001.ts").await,
            Err(StreamError::Invalid)
        ));
        assert_eq!(fake.runs().len(), 1, "a software rerun would restart what was already played");
    }

    #[tokio::test]
    async fn the_configured_read_rate_paces_a_copy_and_not_a_transcode() {
        let fake = Fake::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let src = HlsStreamSource::with_parts(dir.path(), prep_root(&dir), fake.clone())
            .with_read_rate(3.5);
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        src.start(spec("s2", 12_000)).await.expect("start");
        let runs = started_runs(&fake, 2).await;
        let copy = runs.iter().find(|args| args.iter().any(|a| a == "copy")).expect("a copy run");
        assert_eq!(pair_after(copy, "-readrate").as_deref(), Some("3.5"));
        let encode =
            runs.iter().find(|args| args.iter().any(|a| a == "libx264")).expect("an encode");
        assert!(!encode.iter().any(|a| a == "-readrate"));
    }

    #[tokio::test]
    async fn stopping_kills_the_run_and_waits_for_it() {
        let fake = Fake { hang: true, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        started_runs(&fake, 1).await;
        assert_eq!(fake.dropped(), 0, "the run is going");

        src.stop(&sid("s1")).await.expect("stop");

        assert_eq!(
            fake.dropped(),
            1,
            "dropping the run future is what kills ffmpeg; one that outlives its session keeps \
             writing to a deleted directory"
        );
    }

    #[tokio::test]
    async fn reaping_an_idle_session_kills_the_run() {
        let fake = Fake { hang: true, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        let src = src.with_idle_timeout(SignedDuration::from_secs(5));
        src.start(fmp4_spec("s1", 12_000)).await.expect("start");
        started_runs(&fake, 1).await;

        let reaped = src.reap_at(Timestamp::now() + SignedDuration::from_secs(6)).await;

        assert_eq!(reaped, 1);
        assert_eq!(fake.dropped(), 1);
    }

    #[tokio::test]
    async fn a_newer_generation_waits_for_the_old_run_to_die_before_returning() {
        let fake = Fake { hang: true, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        started_runs(&fake, 1).await;

        src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("renegotiate");

        assert_eq!(fake.dropped(), 1);
    }

    #[tokio::test]
    async fn halting_a_run_twice_is_harmless() {
        let mut producer = Producer { task: Some(tokio::spawn(async {})), state: Arc::default() };
        producer.halt().await;
        producer.halt().await;
        assert!(producer.task.is_none());
    }

    #[tokio::test]
    async fn a_direct_generation_gives_way_to_a_streamed_one() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, direct_registration());
        src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("renegotiate");
        assert_eq!(live_generations(&src, "s1"), vec![StreamGeneration(2)]);
    }

    #[tokio::test]
    async fn a_removed_session_drops_its_run() {
        let fake = Fake { hang: true, ..Fake::default() };
        let (_dir, src) = engine(fake.clone());
        src.start(spec("s1", 12_000)).await.expect("start");
        started_runs(&fake, 1).await;
        src.remove(&sid("s1"));
        assert_eq!(until_dropped(&fake).await, 1);
    }

    #[tokio::test]
    async fn a_newer_generation_supersedes_the_previous_one_and_takes_its_directory_with_it() {
        let (_dir, src) = engine(Fake::default());
        let first = src.start(spec("s1", 12_000)).await.expect("start");
        let second =
            src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("renegotiate");
        assert_ne!(first.output_dir, second.output_dir);
        assert!(!PathBuf::from(&first.output_dir).exists());
        assert!(PathBuf::from(&second.output_dir).exists());
        assert_eq!(live_generations(&src, "s1"), vec![StreamGeneration(2)]);
    }

    #[tokio::test]
    async fn a_superseded_generation_is_no_longer_served() {
        let (_dir, src) = engine(Fake::default());
        let started = src.start(spec("s1", 12_000)).await.expect("start");
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from(&started.output_dir)));
        assert!(src.master_playlist(&claims_at("s1", GEN)).is_ok());

        let next =
            src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("renegotiate");
        src.register(
            sid("s1"),
            StreamGeneration(2),
            transcode_registration(PathBuf::from(&next.output_dir)),
        );

        assert!(matches!(src.master_playlist(&claims_at("s1", GEN)), Err(StreamError::NotLive)));
        assert!(src.master_playlist(&claims_at("s1", StreamGeneration(2))).is_ok());
    }

    #[tokio::test]
    async fn a_generation_that_lands_after_a_newer_one_is_dropped_on_sight() {
        let (_dir, src) = engine(Fake::default());
        let newer = src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("start");
        let older = src.start(spec("s1", 12_000)).await.expect("late start");
        assert_eq!(live_generations(&src, "s1"), vec![StreamGeneration(2)]);
        assert!(!PathBuf::from(&older.output_dir).exists());
        assert!(PathBuf::from(&newer.output_dir).exists());
    }

    #[tokio::test]
    async fn stopping_removes_the_whole_session_tree() {
        let (dir, src) = engine(Fake::default());
        src.start(spec("s1", 12_000)).await.expect("start");
        src.start(spec_at("s1", StreamGeneration(2), 12_000)).await.expect("renegotiate");
        src.stop(&sid("s1")).await.expect("stop");
        assert!(!dir.path().join("s1").exists());
    }

    #[tokio::test]
    async fn media_path_falls_back_to_join_for_absent_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        std::fs::create_dir_all(base.join(VARIANT)).unwrap();
        let (_cache, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(base.clone()));
        let canonical_base = base.canonicalize().unwrap();
        let absent_variant = src.media_path(&claims("s1"), "subs", MEDIA_PLAYLIST).await.unwrap();
        assert_eq!(absent_variant, canonical_base.join("subs").join(MEDIA_PLAYLIST));
        let absent_file = src.media_path(&claims("s1"), VARIANT, "other.m3u8").await.unwrap();
        assert!(absent_file.ends_with("v0/other.m3u8"));
    }

    #[tokio::test]
    async fn media_path_missing_base_is_not_live() {
        let (_dir, src) = engine(Fake::default());
        src.register(
            sid("s1"),
            GEN,
            transcode_registration(PathBuf::from("/no/such/shadowmask/dir")),
        );
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, MEDIA_PLAYLIST).await,
            Err(StreamError::NotLive)
        ));
    }

    #[test]
    fn is_expired_respects_boundary() {
        let base = Timestamp::UNIX_EPOCH;
        let timeout = SignedDuration::from_secs(30);
        assert!(!is_expired(base + SignedDuration::from_secs(30), base, timeout));
        assert!(is_expired(base + SignedDuration::from_secs(31), base, timeout));
    }

    #[tokio::test]
    async fn media_path_unknown_session_not_live() {
        let (_dir, src) = engine(Fake::default());
        assert!(matches!(
            src.media_path(&claims("nope"), VARIANT, MEDIA_PLAYLIST).await,
            Err(StreamError::NotLive)
        ));
    }

    #[tokio::test]
    async fn media_path_rejects_unsafe_segments() {
        let (_dir, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from("/cache/s1")));
        for (variant, file) in [("..", MEDIA_PLAYLIST), (VARIANT, ""), (VARIANT, "a\\b")] {
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
        std::os::unix::fs::symlink(outside.path(), base.join(VARIANT)).unwrap();
        let (_cache, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(base));
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "secret").await,
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
        std::fs::create_dir_all(base.join(VARIANT)).unwrap();
        std::os::unix::fs::symlink(&secret, base.join(VARIANT).join("leak.bin")).unwrap();
        let (_cache, src) = engine(Fake::default());
        src.register(sid("s1"), GEN, transcode_registration(base));
        assert!(matches!(
            src.media_path(&claims("s1"), VARIANT, "leak.bin").await,
            Err(StreamError::Invalid)
        ));
    }

    #[tokio::test]
    async fn touch_updates_started_and_rejects_others() {
        let (_dir, src) = engine(Fake::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        src.touch(&sid("s1")).await.expect("touch");
        assert!(matches!(src.touch(&sid("nope")).await, Err(TranscodeError::NotFound)));
        src.register(sid("direct"), GEN, direct_registration());
        assert!(matches!(src.touch(&sid("direct")).await, Err(TranscodeError::NotFound)));
    }

    #[tokio::test]
    async fn stop_evicts_cache_and_unknown_errors() {
        let (_dir, src) = engine(Fake::default());
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.stop(&sid("s1")).await.expect("stop");
        assert!(!PathBuf::from(&started.output_dir).exists());
        assert!(matches!(src.stop(&sid("nope")).await, Err(TranscodeError::NotFound)));
    }

    #[tokio::test]
    async fn reap_evicts_idle_streamed_sessions_only() {
        let (_dir, src) = engine(Fake::default());
        let src = src.with_idle_timeout(SignedDuration::from_secs(5));
        let started = src.start(spec("s1", 8_000)).await.expect("start");
        src.register(sid("direct"), GEN, direct_registration());
        let now = Timestamp::now();
        assert_eq!(src.reap_at(now + SignedDuration::from_secs(3)).await, 0);
        assert_eq!(src.reap_at(now + SignedDuration::from_secs(6)).await, 1);
        assert!(!PathBuf::from(&started.output_dir).exists());
        assert!(src.inner.sessions.read().unwrap().contains_key(&sid("direct")));
    }

    #[tokio::test]
    async fn reap_idle_uses_wall_clock() {
        let (_dir, src) = engine(Fake::default());
        src.start(spec("s1", 8_000)).await.expect("start");
        assert_eq!(src.reap_idle().await, 0);
    }

    #[tokio::test]
    async fn clone_shares_state() {
        let (_dir, src) = engine(Fake::default());
        let cloned = src.clone();
        src.register(sid("s1"), GEN, transcode_registration(PathBuf::from("/cache/s1")));
        assert!(cloned.master_playlist(&claims("s1")).is_ok());
    }

    fn recorded<F, Fut>(work: F) -> String
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
        let handle = recorder.handle();
        metrics::with_local_recorder(&recorder, || {
            let runtime =
                tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(work());
        });
        handle.render()
    }

    async fn serve(src: &HlsStreamSource<Fake>, spec: TranscodeSpec, files: &[&str]) {
        src.start(spec).await.expect("start");
        for file in files {
            let _ = src.media_path(&claims("s1"), VARIANT, file).await;
        }
    }

    fn assert_lines(rendered: &str, lines: &[&str]) {
        for line in lines {
            assert!(rendered.contains(line), "missing [{line}] in:\n{rendered}");
        }
    }

    #[test]
    fn a_run_counts_the_segments_it_wrote_with_its_encoder_and_media_time() {
        let rendered = recorded(|| async {
            let (_dir, src) = engine(Fake { segments: 3, ..Fake::default() });
            serve(&src, spec("s1", 12_000), &["seg_00000.ts", "seg_00001.ts"]).await;
        });
        assert_lines(
            &rendered,
            &[
                "stream_starts_total{delivery=\"transcode\",outcome=\"ok\"} 1",
                "transcode_segments_total{encoder=\"software\",outcome=\"ok\"} 3",
                "transcode_media_milliseconds_total{encoder=\"software\"} 12000",
                "transcode_segment_duration_milliseconds_count{encoder=\"software\"} 3",
                "stream_first_segment_milliseconds_count{delivery=\"transcode\"} 1",
            ],
        );
        assert!(!rendered.contains("transcode_fallbacks_total"));
    }

    #[test]
    fn a_hardware_failure_rerun_in_software_is_a_fallback() {
        let rendered = recorded(|| async {
            let (_dir, src) =
                vaapi_engine(Fake { segments: 3, failing_runs: 1, ..Fake::default() });
            serve(&src, spec("s1", 12_000), &["seg_00000.ts"]).await;
        });
        assert_lines(
            &rendered,
            &[
                "transcode_segments_total{encoder=\"vaapi-encode\",outcome=\"failed\"} 1",
                "transcode_segments_total{encoder=\"software\",outcome=\"ok\"} 3",
                "transcode_fallbacks_total 1",
            ],
        );
    }

    #[test]
    fn each_gpu_fallback_counts_under_the_path_that_failed() {
        let rendered = recorded(|| async {
            let (_dir, src) = gpu_engine(Fake { segments: 3, failing_runs: 2, ..Fake::default() });
            serve(&src, spec("s1", 12_000), &["seg_00000.ts"]).await;
        });
        assert_lines(
            &rendered,
            &[
                "transcode_segments_total{encoder=\"vaapi\",outcome=\"failed\"} 1",
                "transcode_segments_total{encoder=\"vaapi-encode\",outcome=\"failed\"} 1",
                "transcode_segments_total{encoder=\"software\",outcome=\"ok\"} 3",
                "transcode_fallbacks_total 2",
            ],
        );
    }

    #[test]
    fn a_failed_copy_on_a_hardware_host_is_not_rerun_in_software() {
        let fake = Fake { failing_runs: 1, ..Fake::default() };
        let ran = fake.clone();
        let rendered = recorded(|| async {
            let (_dir, src) = gpu_engine(fake);
            let copy = copy_spec("s1", SegmentContainer::MpegTs, 12_000, &[0, 4_000, 8_000]);
            serve(&src, copy, &["seg_00000.ts"]).await;
        });
        assert_eq!(ran.runs().len(), 1);
        assert_lines(
            &rendered,
            &["transcode_segments_total{encoder=\"remux\",outcome=\"failed\"} 1"],
        );
        assert!(!rendered.contains("transcode_fallbacks_total"), "{rendered}");
    }

    #[test]
    fn a_copied_run_is_counted_as_remux() {
        let rendered = recorded(|| async {
            let (_dir, src) = engine(Fake { segments: 3, ..Fake::default() });
            let copy = copy_spec("s1", SegmentContainer::MpegTs, 12_000, &[0, 4_000, 8_000]);
            serve(&src, copy, &["seg_00000.ts"]).await;
        });
        assert_lines(
            &rendered,
            &[
                "stream_starts_total{delivery=\"remux\",outcome=\"ok\"} 1",
                "transcode_segments_total{encoder=\"remux\",outcome=\"ok\"} 3",
                "stream_first_segment_milliseconds_count{delivery=\"remux\"} 1",
            ],
        );
    }

    #[test]
    fn a_run_that_cannot_start_or_fails_is_counted_as_failed() {
        let rendered = recorded(|| async {
            let (_dir, unspawnable) = engine(Fake { unspawnable: true, ..Fake::default() });
            serve(&unspawnable, spec("s1", 12_000), &["seg_00000.ts"]).await;
            let (_dir, failing) = engine(Fake { fail: true, ..Fake::default() });
            serve(&failing, spec("s1", 12_000), &["seg_00000.ts"]).await;
        });
        assert_lines(
            &rendered,
            &["transcode_segments_total{encoder=\"software\",outcome=\"failed\"} 2"],
        );
        assert!(
            !rendered.contains("transcode_segments_total{encoder=\"software\",outcome=\"ok\"}")
        );
        assert!(!rendered.contains("stream_first_segment_milliseconds"), "{rendered}");
    }

    #[test]
    fn a_run_stopped_part_way_counts_what_it_wrote() {
        let rendered = recorded(|| async {
            let (_dir, src) = engine(Fake { segments: 1, hang: true, ..Fake::default() });
            serve(&src, spec("s1", 12_000), &["seg_00000.ts"]).await;
            src.stop(&sid("s1")).await.expect("stop");
        });
        assert_lines(
            &rendered,
            &[
                "transcode_segments_total{encoder=\"software\",outcome=\"ok\"} 1",
                "transcode_media_milliseconds_total{encoder=\"software\"} 4000",
            ],
        );
    }

    #[test]
    fn an_abandoned_segment_is_counted() {
        let rendered = recorded(|| async {
            drop(SegmentWait::started(&sid("s1"), GEN, "seg_00003.ts"));
            let mut settled = SegmentWait::started(&sid("s1"), GEN, "seg_00004.ts");
            settled.settled();
        });
        assert!(rendered.contains("transcode_segments_abandoned_total 1"), "{rendered}");
    }

    #[test]
    fn a_start_that_fails_is_counted() {
        let rendered = recorded(|| async {
            let (dir, src) = engine(Fake::default());
            std::fs::write(dir.path().join("s1"), b"in the way").unwrap();
            assert!(src.start(spec("s1", 12_000)).await.is_err());
        });
        assert_lines(
            &rendered,
            &["stream_starts_total{delivery=\"transcode\",outcome=\"failed\"} 1"],
        );
    }

    #[test]
    fn the_run_counts_its_first_media_segment_once_and_not_the_init() {
        let init_only = recorded(|| async {
            let (_dir, src) = engine(Fake { segments: 2, ..Fake::default() });
            src.start(fmp4_spec("s1", 12_000)).await.expect("start");
            src.media_path(&claims("s1"), VARIANT, INIT_SEGMENT).await.expect("init");
        });
        let segments = recorded(|| async {
            let (_dir, src) = engine(Fake { segments: 2, ..Fake::default() });
            src.start(fmp4_spec("s1", 12_000)).await.expect("start");
            for file in [INIT_SEGMENT, "seg_00000.m4s", "seg_00001.m4s"] {
                src.media_path(&claims("s1"), VARIANT, file).await.expect("served");
            }
        });
        assert!(!init_only.contains("stream_first_segment_milliseconds"), "{init_only}");
        assert_lines(&segments, &["stream_first_segment_milliseconds_count{delivery=\"remux\"} 1"]);
    }
}
