mod cells;

use std::path::PathBuf;
use std::sync::Arc;

use jiff::{SignedDuration, Timestamp};
use tracing::{info, warn};
use uuid::Uuid;

use domain::catalog::{VersionDetail, VersionId};
use domain::common::{Page, PageRequest, paginate};
use domain::error::SessionError;
use domain::media::VideoTrack;
use domain::negotiation::{
    AvailableSubtitles, CONVERTED_AUDIO_CODEC, NegotiationInput, NegotiationReason,
    effective_max_height, negotiate, resolve_audio, resolve_subtitle,
};
use domain::playback::{
    PlaybackProgress, SubtitleOverride, SubtitleTrackRef, UserSubtitleOffset, is_complete,
    is_started,
};
use domain::profile::{CapabilityProfile, Container, ProfileRegistry};
use domain::repository::{
    PreferencesRepository, ProgressRepository, SessionRegistry, UserRepository, VersionCatalog,
};
use domain::service::SessionService;
use domain::session::{
    BurnSubtitle, ClientCapabilities, DeliveryMode, DeliveryPreference, HeartbeatAck,
    PlaybackSession, PlaybackState, Renegotiated, SegmentContainer, SelectedTracks, SessionId,
    SessionStartInput, SessionStarted, SessionUpdate, SoftSubtitle, SoftSubtitleSource,
    StreamClaims, StreamGeneration, StreamRegistration, StreamRegistry, StreamTokens,
    SubtitleChange, SubtitleDelivery, SubtitleFileSpec, SubtitleRendition, SubtitleRequest,
    SubtitleSelection, TranscodeManager, TranscodeSpec, segment_container_for,
};
use domain::user::{Principal, UserId};

use crate::acl;
use cells::{Live, SessionCells};

const HEARTBEAT_INTERVAL_S: u32 = 10;
const MISSED_HEARTBEATS_BEFORE_REAP: i64 = 6;
const DEFAULT_BANDWIDTH: u64 = 4_000_000;
const TOKEN_TTL_SECS: i64 = 129_600;

#[derive(Clone, Copy, Default)]
struct Timeline {
    origin_ms: u64,
    sequential: bool,
}

#[derive(Clone)]
struct LaunchContext {
    user: UserId,
    version: VersionId,
    capabilities: ClientCapabilities,
    requested_audio: Option<u32>,
    requested_subtitle: Option<SubtitleSelection>,
    remembered_audio: Option<u32>,
    remembered_subtitle: Option<SubtitleOverride>,
    bitrate_cap: Option<u64>,
    target_height: Option<u32>,
    force_burn: bool,
    downmix_stereo: bool,
    delivery: DeliveryPreference,
}

struct Launched {
    mode: DeliveryMode,
    container: Option<SegmentContainer>,
    selected: SelectedTracks,
    timeline: Timeline,
    subtitle_file: bool,
}

impl Launched {
    fn answered(&self, token: &str) -> SelectedTracks {
        SelectedTracks {
            subtitle_url: self.subtitle_file.then(|| subtitle_path(token)),
            ..self.selected.clone()
        }
    }
}

impl LaunchContext {
    fn has_override(&self) -> bool {
        self.remembered_audio.is_some() || self.remembered_subtitle.is_some()
    }
}

struct Inner<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> {
    catalog: Vc,
    profiles: Pr,
    transcode: Tm,
    tokens: Tk,
    streams: Sr,
    sessions: Reg,
    users: U,
    progress: Pg,
    preferences: Pf,
    cells: SessionCells,
}

pub struct DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> {
    #[allow(clippy::type_complexity)]
    inner: Arc<Inner<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>>,
    max_transcode_height: Option<u32>,
    token_ttl_secs: i64,
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> Clone
    for DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            max_transcode_height: self.max_transcode_height,
            token_ttl_secs: self.token_ttl_secs,
        }
    }
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        catalog: Vc,
        profiles: Pr,
        transcode: Tm,
        tokens: Tk,
        streams: Sr,
        sessions: Reg,
        users: U,
        progress: Pg,
        preferences: Pf,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                catalog,
                profiles,
                transcode,
                tokens,
                streams,
                sessions,
                users,
                progress,
                preferences,
                cells: SessionCells::default(),
            }),
            max_transcode_height: None,
            token_ttl_secs: TOKEN_TTL_SECS,
        }
    }

    pub fn with_max_transcode_height(mut self, height: Option<u32>) -> Self {
        self.max_transcode_height = height;
        self
    }

    pub fn with_token_ttl_secs(mut self, secs: i64) -> Self {
        self.token_ttl_secs = secs;
        self
    }
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
where
    Tm: TranscodeManager,
    Sr: StreamRegistry,
    Reg: SessionRegistry,
{
    async fn release(&self, session: &SessionId) {
        let _ = self.inner.transcode.stop(session).await;
        self.inner.streams.remove(session);
    }

    async fn clear(&self, session: &SessionId) -> Result<(), SessionError> {
        self.release(session).await;
        self.inner.sessions.remove(session).await?;
        Ok(())
    }

    async fn teardown(&self, session: &SessionId, live: &mut Live) -> Result<(), SessionError> {
        live.ended = true;
        self.clear(session).await
    }

    async fn end_any(&self, session: &SessionId) -> Result<(), SessionError> {
        match self.inner.cells.end(session, async |live| self.teardown(session, live).await).await {
            Err(SessionError::NotFound) => self.clear(session).await,
            ended => ended,
        }
    }

    pub async fn reap_idle(&self) -> usize {
        let reaped = self.inner.transcode.reap_idle().await;
        let cutoff = Timestamp::now()
            - SignedDuration::from_secs(
                i64::from(HEARTBEAT_INTERVAL_S) * MISSED_HEARTBEATS_BEFORE_REAP,
            );
        match self.inner.sessions.list_idle(cutoff).await {
            Ok(idle) => {
                let mut dropped = 0;
                for session in &idle {
                    dropped += usize::from(self.reap(session, cutoff).await);
                }
                if dropped > 0 {
                    info!("Reaped [{dropped}] playback sessions that stopped heartbeating");
                }
            }
            Err(err) => warn!("Could not reap idle playback sessions: [{err}]"),
        }
        reaped
    }

    async fn reap(&self, session: &SessionId, cutoff: Timestamp) -> bool {
        let reaped = self
            .inner
            .cells
            .try_run(session, async |live| {
                let idle = self
                    .inner
                    .sessions
                    .get(session)
                    .await?
                    .is_none_or(|playback| playback.last_heartbeat_at < cutoff);
                if idle {
                    self.teardown(session, live).await?;
                }
                Ok(idle)
            })
            .await;
        let reaped = match reaped {
            None => Ok(false),
            Some(Err(SessionError::NotFound)) => self.clear(session).await.map(|()| true),
            Some(reaped) => reaped,
        };
        #[rustfmt::skip]
        let reaped = reaped.inspect_err(|err| warn!("Could not reap session [{}]: [{err}]", session.0));
        reaped.unwrap_or(false)
    }
}

fn manifest_path(mode: DeliveryMode, token: &str) -> String {
    match mode {
        DeliveryMode::Direct => format!("/stream/{token}/file"),
        _ => format!("/stream/{token}/master.m3u8"),
    }
}

fn subtitle_path(token: &str) -> String {
    format!("/stream/{token}/subtitles.vtt")
}

fn state_label(state: PlaybackState) -> &'static str {
    match state {
        PlaybackState::Playing => "playing",
        PlaybackState::Paused => "paused",
    }
}

fn combine_caps(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}

fn bandwidth_of(detail: &VersionDetail) -> u64 {
    detail.video.first().and_then(|v| v.bitrate).unwrap_or(DEFAULT_BANDWIDTH)
}

fn fitted_height(video: &VideoTrack, max_width: u32, max_height: u32) -> Option<u32> {
    let by_width = match video.width > max_width {
        true => {
            let scaled = u64::from(video.height) * u64::from(max_width) / u64::from(video.width);
            u32::try_from(scaled).unwrap_or(video.height) & !1
        }
        false => video.height,
    };
    let fitted = by_width.min(max_height);
    (fitted < video.height).then_some(fitted)
}

fn subtitle_rendition(
    selected: &SelectedTracks,
    detail: &VersionDetail,
) -> Option<SubtitleRendition> {
    if selected.subtitle_delivery != Some(SubtitleDelivery::HlsVtt) {
        return None;
    }
    let language = match selected.subtitle_track.as_ref()? {
        SubtitleTrackRef::Embedded(idx) => detail
            .subtitles
            .iter()
            .find(|s| s.index == *idx)
            .and_then(|s| s.language.as_ref())
            .map(|l| l.0.clone()),
        SubtitleTrackRef::File(id) => detail
            .subtitle_files
            .iter()
            .find(|s| &s.id == id)
            .and_then(|s| s.language.as_ref())
            .map(|l| l.0.clone()),
    };
    let language = language.unwrap_or_else(|| "und".to_owned());
    Some(SubtitleRendition { name: language.clone(), language })
}

fn with_prepared_subtitles(mut detail: VersionDetail) -> VersionDetail {
    detail.subtitles.retain(|track| detail.extracted_subtitles.contains(&track.index));
    detail
}

fn burn_subtitle_for(selected: &SelectedTracks, detail: &VersionDetail) -> Option<BurnSubtitle> {
    if selected.subtitle_delivery != Some(SubtitleDelivery::Burned) {
        return None;
    }
    match selected.subtitle_track.as_ref()? {
        SubtitleTrackRef::Embedded(index) => {
            let track = detail.subtitles.iter().find(|s| s.index == *index)?;
            if !track.format.is_image() {
                return Some(BurnSubtitle::EmbeddedText(*index));
            }
            let video = detail.video.first()?;
            Some(BurnSubtitle::EmbeddedImage {
                index: *index,
                width: video.width,
                height: video.height,
            })
        }
        SubtitleTrackRef::File(id) => detail
            .subtitle_files
            .iter()
            .find(|f| &f.id == id)
            .map(|f| BurnSubtitle::File(f.path.clone())),
    }
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
where
    Vc: VersionCatalog + Send + Sync,
    Pr: ProfileRegistry + Send + Sync,
    Tm: TranscodeManager + Send + Sync,
    Tk: StreamTokens + Send + Sync,
    Sr: StreamRegistry + Send + Sync,
    Reg: SessionRegistry + Send + Sync,
    U: UserRepository + Send + Sync,
    Pg: ProgressRepository + Send + Sync,
    Pf: PreferencesRepository + Send + Sync,
{
    async fn owned(
        &self,
        caller: &Principal,
        session: &SessionId,
    ) -> Result<PlaybackSession, SessionError> {
        let playback = self.inner.sessions.get(session).await?.ok_or(SessionError::NotFound)?;
        if playback.user != caller.user {
            warn!(
                "User [{}] tried to control session [{}], which belongs to user [{}]",
                caller.user.0, session.0, playback.user.0
            );
            return Err(SessionError::NotFound);
        }
        Ok(playback)
    }

    async fn remember_tracks(
        &self,
        ctx: &LaunchContext,
        position_ms: u64,
    ) -> Result<(), SessionError> {
        if !ctx.has_override() {
            return Ok(());
        }
        self.inner
            .progress
            .upsert(PlaybackProgress {
                user: ctx.user.clone(),
                version: ctx.version.clone(),
                position_ms,
                audio_track: ctx.remembered_audio,
                subtitle: ctx.remembered_subtitle.clone(),
                updated_at: Timestamp::now(),
            })
            .await?;
        Ok(())
    }

    async fn soft_subtitle_for(
        &self,
        selected: &SelectedTracks,
        detail: &VersionDetail,
        ctx: &LaunchContext,
    ) -> Option<SoftSubtitle> {
        let delivery = selected
            .subtitle_delivery
            .filter(|d| matches!(d, SubtitleDelivery::HlsVtt | SubtitleDelivery::External))?;
        let track = selected.subtitle_track.as_ref()?;
        let source = match track {
            SubtitleTrackRef::Embedded(idx) => SoftSubtitleSource::Embedded(*idx),
            SubtitleTrackRef::File(id) => {
                let path = detail.subtitle_files.iter().find(|f| &f.id == id)?.path.clone();
                SoftSubtitleSource::File(path)
            }
        };
        Some(SoftSubtitle { source, offset_ms: self.resolve_offset(ctx, track).await, delivery })
    }

    async fn direct_subtitle_file(
        &self,
        session_id: &SessionId,
        generation: StreamGeneration,
        detail: &VersionDetail,
        subtitle: SoftSubtitle,
    ) -> Option<PathBuf> {
        let spec = SubtitleFileSpec {
            session: session_id.clone(),
            generation,
            version: detail.version.id.clone(),
            input_path: detail.version.path.clone(),
            subtitle,
        };
        match self.inner.transcode.prepare_subtitle_file(spec).await {
            Ok(path) => Some(path),
            Err(err) => {
                #[rustfmt::skip]
                warn!("session [{}] generation [{}] plays without its subtitle file: [{err}]", session_id.0, generation.0);
                None
            }
        }
    }

    async fn resolve_offset(&self, ctx: &LaunchContext, track: &SubtitleTrackRef) -> i64 {
        if let Some(offset) = ctx.requested_subtitle.as_ref().and_then(|s| s.offset_ms) {
            return offset;
        }
        self.inner
            .preferences
            .get_subtitle_offset(&ctx.user, &ctx.version, track)
            .await
            .ok()
            .flatten()
            .map(|o| o.offset_ms)
            .unwrap_or(0)
    }

    fn create_token(
        &self,
        session: &SessionId,
        generation: StreamGeneration,
        user: &UserId,
        version: &VersionId,
    ) -> Result<String, SessionError> {
        let expires_at = Timestamp::from_second(Timestamp::now().as_second() + self.token_ttl_secs)
            .unwrap_or(Timestamp::MAX);
        let claims = StreamClaims {
            session: session.clone(),
            generation,
            user: user.clone(),
            version: version.clone(),
            expires_at,
            nonce: Uuid::new_v4().to_string(),
        };
        self.inner
            .tokens
            .create(&claims)
            .map(|token| token.0)
            .map_err(|_| SessionError::NegotiationFailed)
    }

    fn profile_for(&self, capabilities: &ClientCapabilities) -> (CapabilityProfile, &'static str) {
        let named = self.inner.profiles.resolve(&capabilities.platform);
        let Some(reported) = capabilities.decoding.as_ref() else {
            return (named, "none");
        };
        match named.merged_with(reported) {
            Ok(merged) => (merged, "accepted"),
            Err(error) => {
                warn!(
                    "rejected the capabilities reported by platform [{}]: [{error}]",
                    capabilities.platform
                );
                (named, "rejected")
            }
        }
    }

    async fn launch(
        &self,
        session_id: &SessionId,
        generation: StreamGeneration,
        detail: &VersionDetail,
        ctx: &LaunchContext,
        position_ms: u64,
    ) -> Result<Launched, SessionError> {
        let Some(container) = Container::parse(&detail.version.container) else {
            #[rustfmt::skip]
            tracing::warn!("cannot play version [{}] at [{}]: unknown container [{}]", detail.version.id.0, detail.version.path, detail.version.container);
            return Err(SessionError::NegotiationFailed);
        };
        let (profile, report) = self.profile_for(&ctx.capabilities);
        let max_bitrate = combine_caps(ctx.capabilities.max_bitrate, ctx.bitrate_cap);
        let keyframes_us = self
            .inner
            .catalog
            .keyframes_us(&detail.version.id)
            .await?
            .filter(|keyframes| !keyframes.is_empty());
        let input = NegotiationInput {
            container,
            video: detail.video.clone(),
            audio: detail.audio.clone(),
            subtitles: detail.subtitles.clone(),
            requested_audio: ctx.requested_audio,
            requested_subtitle: ctx.requested_subtitle.clone(),
            max_bitrate,
            target_height: ctx.target_height,
            force_burn: ctx.force_burn,
            downmix_stereo: ctx.downmix_stereo,
            delivery: ctx.delivery,
            prepared: keyframes_us.is_some(),
            tracks: ctx.capabilities.tracks,
        };
        let outcome = negotiate(&input, &profile);
        let ceiling = match profile.max_frame_rate {
            Some(rate) => format!("{}x{}@{rate}", profile.max_width, profile.max_height),
            None => format!("{}x{}", profile.max_width, profile.max_height),
        };
        let reasons: Vec<&str> = outcome.reasons.iter().map(NegotiationReason::as_str).collect();
        let bandwidth = bandwidth_of(detail);
        let cap = effective_max_height(profile.max_height, ctx.target_height)
            .min(self.max_transcode_height.unwrap_or(u32::MAX));
        let scale_to = detail.video.first().and_then(|v| fitted_height(v, profile.max_width, cap));
        let height = match scale_to {
            Some(height) => height.to_string(),
            None => "source".to_owned(),
        };
        let remux = outcome.mode == DeliveryMode::Remux;
        let audio_codec = match outcome.convert_audio {
            true => Some(CONVERTED_AUDIO_CODEC),
            false => outcome
                .selected
                .audio_track
                .and_then(|idx| detail.audio.iter().find(|a| a.index == idx))
                .map(|a| a.codec.as_str()),
        };
        let segments = (outcome.mode != DeliveryMode::Direct).then(|| {
            segment_container_for(
                remux,
                detail.video.first().map(|v| v.codec.as_str()),
                audio_codec,
            )
        });
        #[rustfmt::skip]
        info!("negotiated session [{}] generation [{}]: platform [{}] report [{report}] ceiling [{ceiling}] height [{height}] mode [{:?}] container [{}] reasons [{}]", session_id.0, generation.0, ctx.capabilities.platform, outcome.mode, segments.map_or("none", SegmentContainer::as_str), reasons.join(", "));

        let mut origin_ms = 0;
        let mut sequential = false;
        let subtitle_file;
        if outcome.mode == DeliveryMode::Direct {
            let _ = self.inner.transcode.stop(session_id).await;
            subtitle_file = match self.soft_subtitle_for(&outcome.selected, detail, ctx).await {
                Some(subtitle) => {
                    self.direct_subtitle_file(session_id, generation, detail, subtitle).await
                }
                None => None,
            };
            self.inner.streams.register(
                session_id.clone(),
                generation,
                StreamRegistration {
                    mode: DeliveryMode::Direct,
                    output_dir: PathBuf::new(),
                    direct_path: Some(PathBuf::from(&detail.version.path)),
                    bandwidth,
                    subtitle: None,
                    subtitle_file: subtitle_file.clone(),
                },
            );
        } else {
            let burn_subtitle = burn_subtitle_for(&outcome.selected, detail);
            let soft_subtitle = self.soft_subtitle_for(&outcome.selected, detail, ctx).await;
            let started = self
                .inner
                .transcode
                .start(TranscodeSpec {
                    session: session_id.clone(),
                    generation,
                    version: detail.version.id.clone(),
                    input_path: detail.version.path.clone(),
                    duration_ms: detail.version.duration_ms,
                    keyframes_us,
                    copy: remux,
                    container: segments.unwrap_or(SegmentContainer::MpegTs),
                    seek_ms: (position_ms > 0).then_some(position_ms),
                    audio_track: outcome.selected.audio_track,
                    max_height: if remux { None } else { scale_to },
                    max_bitrate,
                    burn_subtitle,
                    soft_subtitle,
                    convert_audio: outcome.convert_audio,
                    audio_channels: outcome.audio_channels,
                    source_hdr: detail.video.first().and_then(|v| v.hdr),
                })
                .await
                .map_err(|_| SessionError::NegotiationFailed)?;
            origin_ms = started.origin_ms;
            sequential = started.sequential;
            subtitle_file = started.subtitle_file.clone();
            self.inner.streams.register(
                session_id.clone(),
                generation,
                StreamRegistration {
                    mode: outcome.mode,
                    output_dir: PathBuf::from(started.output_dir),
                    direct_path: None,
                    bandwidth,
                    subtitle: subtitle_rendition(&outcome.selected, detail)
                        .filter(|_| started.subtitles),
                    subtitle_file: started.subtitle_file,
                },
            );
        }
        Ok(Launched {
            mode: outcome.mode,
            container: segments,
            selected: outcome.selected,
            timeline: Timeline { origin_ms, sequential },
            subtitle_file: subtitle_file.is_some(),
        })
    }
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> SessionService
    for DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
where
    Vc: VersionCatalog + Send + Sync,
    Pr: ProfileRegistry + Send + Sync,
    Tm: TranscodeManager + Send + Sync,
    Tk: StreamTokens + Send + Sync,
    Sr: StreamRegistry + Send + Sync,
    Reg: SessionRegistry + Send + Sync,
    U: UserRepository + Send + Sync,
    Pg: ProgressRepository + Send + Sync,
    Pf: PreferencesRepository + Send + Sync,
{
    async fn start(
        &self,
        caller: &Principal,
        request: SessionStartInput,
    ) -> Result<SessionStarted, SessionError> {
        let detail = self
            .inner
            .catalog
            .version_detail(&request.version)
            .await?
            .map(with_prepared_subtitles)
            .ok_or(SessionError::VersionNotFound)?;
        if !detail.version.available {
            return Err(SessionError::VersionNotFound);
        }
        let viewer =
            acl::viewer(&self.inner.users, &caller.user).await?.limited_to(caller.age_limit);
        let rating = self.inner.catalog.title_rating(&detail.version.title).await?;
        if !viewer.sees_library(&detail.version.library) || !viewer.permits(rating.as_ref()) {
            return Err(SessionError::VersionNotFound);
        }
        let user = self.inner.users.get(&caller.user).await?;

        if let Some(limit) = user.as_ref().and_then(|u| u.concurrent_stream_limit) {
            let active = self.inner.sessions.list_for_user(&caller.user).await?;
            if active.len() >= limit as usize {
                return Err(SessionError::ConcurrentLimit { active });
            }
        }

        let started_title = detail.version.title.clone();
        for row in self.inner.progress.list_in_progress(&caller.user).await? {
            if row.version == request.version {
                continue;
            }
            let same_title = self
                .inner
                .catalog
                .get_version(&row.version)
                .await?
                .is_some_and(|other| other.title == started_title);
            if same_title {
                self.inner.progress.delete(&caller.user, &row.version).await?;
            }
        }

        let stored = self.inner.progress.get(&caller.user, &request.version).await?;
        let audio = resolve_audio(
            &request.audio,
            stored.as_ref().and_then(|row| row.audio_track),
            &detail.audio,
            user.as_ref().map_or(&[], |u| &u.preferred_audio),
        );
        let subtitle = resolve_subtitle(
            &request.subtitle,
            stored.as_ref().and_then(|row| row.subtitle.as_ref()),
            &AvailableSubtitles { embedded: &detail.subtitles, files: &detail.subtitle_files },
            user.as_ref().map_or(&[], |u| &u.preferred_subtitle),
        );

        let session_id = SessionId(Uuid::new_v4().to_string());
        let now = Timestamp::now();
        let generation = StreamGeneration(1);
        let ctx = LaunchContext {
            user: caller.user.clone(),
            version: request.version.clone(),
            capabilities: request.capabilities.clone(),
            requested_audio: audio.selected,
            requested_subtitle: subtitle.selected,
            remembered_audio: audio.remembered,
            remembered_subtitle: subtitle.remembered,
            bitrate_cap: user.as_ref().and_then(|u| u.bitrate_cap),
            target_height: request.target_height,
            force_burn: request.force_burn,
            downmix_stereo: request.downmix_stereo,
            delivery: request.delivery,
        };
        let live = Live { ctx, generation: generation.0, ended: false };
        let opened = self
            .inner
            .cells
            .create(&session_id, live, async |live| {
                let token =
                    self.create_token(&session_id, generation, &caller.user, &request.version)?;
                let launched = self
                    .launch(&session_id, generation, &detail, &live.ctx, request.start_position_ms)
                    .await?;
                self.remember_tracks(&live.ctx, request.start_position_ms).await?;
                self.inner
                    .sessions
                    .insert(PlaybackSession {
                        id: session_id.clone(),
                        user: caller.user.clone(),
                        device: None,
                        version: request.version.clone(),
                        mode: launched.mode,
                        position_ms: request.start_position_ms,
                        state: PlaybackState::Playing,
                        selected: launched.selected.clone(),
                        started_at: now,
                        last_heartbeat_at: now,
                        completed: false,
                    })
                    .await?;
                Ok((token, launched))
            })
            .await;
        let (token, launched) = match opened {
            Ok(opened) => opened,
            Err(err) => {
                self.release(&session_id).await;
                return Err(err);
            }
        };
        let manifest_url = manifest_path(launched.mode, &token);

        Ok(SessionStarted {
            session_id,
            mode: launched.mode,
            container: launched.container,
            manifest_url,
            origin_ms: launched.timeline.origin_ms,
            sequential: launched.timeline.sequential,
            selected: launched.answered(&token),
            heartbeat_interval_s: HEARTBEAT_INTERVAL_S,
            markers: detail.markers,
            trickplay: detail.trickplay,
        })
    }

    async fn heartbeat(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
        state: PlaybackState,
    ) -> Result<HeartbeatAck, SessionError> {
        self.inner
            .cells
            .run(session, async |live| {
                let mut playback = self.owned(caller, session).await?;
                let owner = playback.user.clone();
                let now = Timestamp::now();
                if playback.state != state {
                    #[rustfmt::skip]
                    tracing::debug!("session [{}] for version [{}] went from [{}] to [{}] at [{}] ms", session.0, playback.version.0, state_label(playback.state), state_label(state), position_ms);
                }
                playback.position_ms = position_ms;
                playback.state = state;
                playback.last_heartbeat_at = now;
                let version = playback.version.clone();
                let remembered = Some(&live.ctx).filter(|ctx| ctx.has_override());

                let played = self.inner.catalog.get_version(&version).await?;
                let duration_ms = played.as_ref().map_or(0, |played| played.duration_ms);
                let completed_title = played.and_then(|played| {
                    is_complete(position_ms, duration_ms).then_some(played.title)
                });
                let already_counted = playback.completed;
                playback.completed |= completed_title.is_some();

                self.inner.sessions.insert(playback).await?;
                let _ = self.inner.transcode.touch(session).await;

                if let Some(title) = completed_title {
                    if !already_counted {
                        self.inner.progress.record_view(&owner, &title, now).await?;
                        self.inner.preferences.remove_watchlist(&owner, title.id()).await?;
                    }
                    self.inner.progress.delete(&owner, &version).await?;
                } else if is_started(position_ms, duration_ms)
                    || remembered.is_some()
                    || self.inner.progress.get(&owner, &version).await?.is_some()
                {
                    self.inner
                        .progress
                        .upsert(PlaybackProgress {
                            user: owner.clone(),
                            version: version.clone(),
                            position_ms,
                            audio_track: remembered.and_then(|ctx| ctx.remembered_audio),
                            subtitle: remembered.and_then(|ctx| ctx.remembered_subtitle.clone()),
                            updated_at: now,
                        })
                        .await?;
                }

                Ok(HeartbeatAck { heartbeat_interval_s: HEARTBEAT_INTERVAL_S })
            })
            .await
    }

    async fn seek(
        &self,
        caller: &Principal,
        session: &SessionId,
        position_ms: u64,
    ) -> Result<Renegotiated, SessionError> {
        self.inner
            .cells
            .run_abortable(session, async |live| {
                let mut playback = self.owned(caller, session).await?;
                let detail = self
                    .inner
                    .catalog
                    .version_detail(&playback.version)
                    .await?
                    .map(with_prepared_subtitles)
                    .ok_or(SessionError::VersionNotFound)?;

                live.generation += 1;
                let generation = StreamGeneration(live.generation);
                let token =
                    self.create_token(session, generation, &playback.user, &playback.version)?;
                let launched =
                    self.launch(session, generation, &detail, &live.ctx, position_ms).await?;
                let manifest_url = manifest_path(launched.mode, &token);

                playback.position_ms = position_ms;
                playback.last_heartbeat_at = Timestamp::now();
                playback.mode = launched.mode;
                playback.selected = launched.selected.clone();
                self.inner.sessions.insert(playback).await?;

                Ok(Renegotiated {
                    session_id: session.clone(),
                    mode: launched.mode,
                    container: launched.container,
                    manifest_url,
                    origin_ms: launched.timeline.origin_ms,
                    sequential: launched.timeline.sequential,
                    selected: launched.answered(&token),
                })
            })
            .await
    }

    async fn update(
        &self,
        caller: &Principal,
        session: &SessionId,
        update: SessionUpdate,
    ) -> Result<Renegotiated, SessionError> {
        self.inner
            .cells
            .run_abortable(session, async |live| {
                let mut playback = self.owned(caller, session).await?;
                let mut ctx = live.ctx.clone();

                let detail = self
                    .inner
                    .catalog
                    .version_detail(&playback.version)
                    .await?
                    .map(with_prepared_subtitles)
                    .ok_or(SessionError::VersionNotFound)?;
                let available = AvailableSubtitles {
                    embedded: &detail.subtitles,
                    files: &detail.subtitle_files,
                };

                if let Some(audio) = update
                    .audio_track
                    .filter(|index| detail.audio.iter().any(|track| track.index == *index))
                {
                    ctx.requested_audio = Some(audio);
                    ctx.remembered_audio = Some(audio);
                }
                ctx.target_height = update.target_height;
                ctx.force_burn = update.force_burn;
                ctx.downmix_stereo = update.downmix_stereo;
                ctx.delivery = update.delivery;
                match update.subtitle {
                    SubtitleChange::Keep => {}
                    SubtitleChange::Disable => {
                        ctx.requested_subtitle = None;
                        ctx.remembered_subtitle = Some(SubtitleOverride::Off);
                    }
                    SubtitleChange::Set(selection) => {
                        let request = SubtitleRequest::Track(selection);
                        let resolved = resolve_subtitle(&request, None, &available, &[]);
                        if let Some(selection) = resolved.selected {
                            if let Some(offset_ms) = selection.offset_ms {
                                self.inner
                                    .preferences
                                    .set_subtitle_offset(UserSubtitleOffset {
                                        user: playback.user.clone(),
                                        version: playback.version.clone(),
                                        subtitle: selection.track.clone(),
                                        offset_ms,
                                    })
                                    .await?;
                            }
                            ctx.remembered_subtitle = resolved.remembered;
                            ctx.requested_subtitle = Some(selection);
                        }
                    }
                }

                live.generation += 1;
                let generation = StreamGeneration(live.generation);
                let token =
                    self.create_token(session, generation, &playback.user, &playback.version)?;
                let launched =
                    self.launch(session, generation, &detail, &ctx, update.position_ms).await?;
                let manifest_url = manifest_path(launched.mode, &token);

                playback.position_ms = update.position_ms;
                playback.mode = launched.mode;
                playback.selected = launched.selected.clone();
                playback.last_heartbeat_at = Timestamp::now();
                let position_ms = playback.position_ms;
                live.ctx = ctx;
                self.inner.sessions.insert(playback).await?;
                self.remember_tracks(&live.ctx, position_ms).await?;

                Ok(Renegotiated {
                    session_id: session.clone(),
                    mode: launched.mode,
                    container: launched.container,
                    manifest_url,
                    origin_ms: launched.timeline.origin_ms,
                    sequential: launched.timeline.sequential,
                    selected: launched.answered(&token),
                })
            })
            .await
    }

    async fn end(&self, caller: &Principal, session: &SessionId) -> Result<(), SessionError> {
        self.owned(caller, session).await?;
        self.inner
            .cells
            .end(session, async |live| {
                self.owned(caller, session).await?;
                self.teardown(session, live).await
            })
            .await
    }

    async fn end_all_for_user(&self, user: &UserId) -> Result<(), SessionError> {
        for session in self.inner.sessions.list_for_user(user).await? {
            self.end_any(&session.id).await?;
        }
        Ok(())
    }

    async fn active_sessions(
        &self,
        caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<PlaybackSession>, SessionError> {
        if acl::is_admin(caller) {
            return Ok(self.inner.sessions.list_all(page).await?);
        }
        let mine = self.inner.sessions.list_for_user(&caller.user).await?;
        Ok(paginate(&mine, page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use domain::catalog::{MovieId, TitleId, Version};
    use domain::common::{LanguageCode, Quality};
    use domain::library::LibraryId;
    use domain::media::{
        AudioTrack, DetectedMarkers, EmbeddedSubtitleTrack, SubtitleFile, SubtitleFileId,
        SubtitleFormat, SubtitleSource, VideoTrack,
    };
    use domain::metadata::ContentRating;
    use domain::playback::WatchlistItem;
    use domain::profile::{AudioCodecCap, CapabilityProfile, ClientDecoding, VideoCodecCap};
    use domain::session::{AudioRequest, SubtitleRequest, TrackHandling};
    use domain::user::{Role, User};

    use futures::FutureExt;
    use mocks::{
        Hold, MockPreferencesRepo, MockProfileRegistry, MockProgressRepo, MockSessionRegistry,
        MockStreamRegistry, MockStreamTokens, MockTranscodeManager, MockUserRepo,
        MockVersionCatalog,
    };

    type Service = DefaultSessionService<
        MockVersionCatalog,
        MockProfileRegistry,
        MockTranscodeManager,
        MockStreamTokens,
        MockStreamRegistry,
        MockSessionRegistry,
        MockUserRepo,
        MockProgressRepo,
        MockPreferencesRepo,
    >;

    struct Harness {
        service: Service,
        catalog: MockVersionCatalog,
        transcode: MockTranscodeManager,
        streams: MockStreamRegistry,
        sessions: MockSessionRegistry,
        users: MockUserRepo,
        progress: MockProgressRepo,
        tokens: MockStreamTokens,
        preferences: MockPreferencesRepo,
    }

    impl Harness {
        fn new() -> Self {
            Self::bounded(None)
        }

        fn bounded(max_transcode_height: Option<u32>) -> Self {
            let catalog = MockVersionCatalog::new();
            let transcode = MockTranscodeManager::new();
            let streams = MockStreamRegistry::new();
            let sessions = MockSessionRegistry::new();
            let users = MockUserRepo::new();
            users.grant(&UserId("u1".to_owned()), &[LibraryId("lib1".to_owned())]);
            let progress = MockProgressRepo::new();
            let tokens = MockStreamTokens::new();
            let preferences = MockPreferencesRepo::new();
            let service = DefaultSessionService::new(
                catalog.clone(),
                MockProfileRegistry::new(profile()),
                transcode.clone(),
                tokens.clone(),
                streams.clone(),
                sessions.clone(),
                users.clone(),
                progress.clone(),
                preferences.clone(),
            )
            .with_max_transcode_height(max_transcode_height);
            Self {
                service,
                catalog,
                transcode,
                streams,
                sessions,
                users,
                progress,
                tokens,
                preferences,
            }
        }
    }

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            containers: vec![Container::Mp4, Container::Hls],
            video: vec![VideoCodecCap {
                codec: "h264".to_owned(),
                max_level: None,
                max_bit_depth: 8,
                smooth: true,
            }],
            audio: vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 2 }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
            max_frame_rate: None,
        }
    }

    fn video(codec: &str, bitrate: Option<u64>) -> VideoTrack {
        VideoTrack {
            index: 0,
            codec: codec.to_owned(),
            width: 1920,
            height: 1080,
            bit_depth: 8,
            hdr: None,
            frame_rate: 24.0,
            bitrate,
        }
    }

    fn audio(index: u32) -> AudioTrack {
        AudioTrack {
            index,
            codec: "aac".to_owned(),
            channels: 2,
            language: None,
            bitrate: None,
            title: None,
        }
    }

    fn audio_in(index: u32, language: Option<&str>) -> AudioTrack {
        AudioTrack { language: language.map(|l| LanguageCode(l.to_owned())), ..audio(index) }
    }

    fn subtitle(
        index: u32,
        language: Option<&str>,
        format: SubtitleFormat,
    ) -> EmbeddedSubtitleTrack {
        EmbeddedSubtitleTrack {
            index,
            language: language.map(|l| LanguageCode(l.to_owned())),
            format,
            forced: false,
            default: false,
            title: None,
        }
    }

    fn detail_with(
        container: &str,
        duration_ms: u64,
        video: Vec<VideoTrack>,
        audio: Vec<AudioTrack>,
        subtitles: Vec<EmbeddedSubtitleTrack>,
    ) -> VersionDetail {
        let extracted_subtitles = subtitles.iter().map(|track| track.index).collect();
        VersionDetail {
            version: Version {
                id: VersionId("v1".to_owned()),
                title: TitleId::Movie(MovieId("m1".to_owned())),
                library: LibraryId("lib1".to_owned()),
                quality: Quality::Fhd,
                container: container.to_owned(),
                path: "/media/m1".to_owned(),
                size_bytes: 1,
                duration_ms,
                available: true,
                added_at: Timestamp::UNIX_EPOCH,
                updated_at: Timestamp::UNIX_EPOCH,
            },
            video,
            audio,
            subtitles,
            subtitle_files: Vec::new(),
            chapters: Vec::new(),
            markers: DetectedMarkers::default(),
            trickplay: Vec::new(),
            extracted_subtitles,
        }
    }

    fn direct_detail() -> VersionDetail {
        detail_with("mp4", 100_000, vec![video("h264", Some(5_000_000))], vec![audio(1)], vec![])
    }

    fn transcode_detail() -> VersionDetail {
        detail_with("mp4", 100_000, vec![video("vp9", Some(5_000_000))], vec![audio(1)], vec![])
    }

    fn make_user(limit: Option<u32>, bitrate_cap: Option<u64>) -> User {
        User {
            id: UserId("u1".to_owned()),
            username: "u1".to_owned(),
            password_hash: String::new(),
            role: Role::User,
            max_content_rating: None,
            preferred_audio: Vec::new(),
            preferred_subtitle: Vec::new(),
            concurrent_stream_limit: limit,
            bitrate_cap,
            active: true,
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn principal() -> Principal {
        Principal {
            user: UserId("u1".to_owned()),
            role: Role::User,
            account_admin: false,
            device: None,
            age_limit: None,
        }
    }

    fn caps(max_bitrate: Option<u64>) -> ClientCapabilities {
        ClientCapabilities {
            platform: "web".to_owned(),
            profile_version: 1,
            max_bitrate,
            decoding: None,
            tracks: TrackHandling::default(),
        }
    }

    fn start_request(start_position_ms: u64) -> SessionStartInput {
        SessionStartInput {
            version: VersionId("v1".to_owned()),
            start_position_ms,
            capabilities: caps(None),
            audio: AudioRequest::Unspecified,
            subtitle: SubtitleRequest::Unspecified,
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
            delivery: DeliveryPreference::Auto,
        }
    }

    fn bare_session(id: &str) -> PlaybackSession {
        PlaybackSession {
            id: SessionId(id.to_owned()),
            user: UserId("u1".to_owned()),
            device: None,
            version: VersionId("v1".to_owned()),
            mode: DeliveryMode::Direct,
            position_ms: 0,
            state: PlaybackState::Playing,
            selected: SelectedTracks {
                audio_track: None,
                audio_delivery: None,
                subtitle_track: None,
                subtitle_delivery: None,
                subtitle_url: None,
            },
            started_at: Timestamp::UNIX_EPOCH,
            last_heartbeat_at: Timestamp::UNIX_EPOCH,
            completed: false,
        }
    }

    fn page() -> PageRequest {
        PageRequest { offset: 0, limit: 10 }
    }

    fn token_of(url: &str) -> &str {
        url.split('/').nth(2).unwrap()
    }

    fn selected(
        delivery: Option<SubtitleDelivery>,
        track: Option<SubtitleTrackRef>,
    ) -> SelectedTracks {
        SelectedTracks {
            audio_track: None,
            audio_delivery: None,
            subtitle_track: track,
            subtitle_delivery: delivery,
            subtitle_url: None,
        }
    }

    #[test]
    fn combine_caps_takes_the_minimum() {
        assert_eq!(combine_caps(Some(5), Some(3)), Some(3));
        assert_eq!(combine_caps(Some(5), None), Some(5));
        assert_eq!(combine_caps(None, Some(7)), Some(7));
        assert_eq!(combine_caps(None, None), None);
    }

    #[test]
    fn manifest_path_varies_by_mode() {
        assert_eq!(manifest_path(DeliveryMode::Direct, "tok"), "/stream/tok/file");
        assert_eq!(manifest_path(DeliveryMode::Remux, "tok"), "/stream/tok/master.m3u8");
        assert_eq!(manifest_path(DeliveryMode::Transcode, "tok"), "/stream/tok/master.m3u8");
    }

    #[test]
    fn bandwidth_uses_first_video_bitrate_or_default() {
        let with_bitrate =
            detail_with("mp4", 1, vec![video("h264", Some(6_000_000))], vec![], vec![]);
        assert_eq!(bandwidth_of(&with_bitrate), 6_000_000);
        let no_bitrate = detail_with("mp4", 1, vec![video("h264", None)], vec![], vec![]);
        assert_eq!(bandwidth_of(&no_bitrate), DEFAULT_BANDWIDTH);
        let no_video = detail_with("mp4", 1, vec![], vec![], vec![]);
        assert_eq!(bandwidth_of(&no_video), DEFAULT_BANDWIDTH);
    }

    #[test]
    fn subtitle_rendition_none_unless_hls_vtt_with_track() {
        let detail = direct_detail();
        assert!(subtitle_rendition(&selected(None, None), &detail).is_none());
        assert!(
            subtitle_rendition(
                &selected(Some(SubtitleDelivery::Burned), Some(SubtitleTrackRef::Embedded(0))),
                &detail
            )
            .is_none()
        );
        assert!(
            subtitle_rendition(&selected(Some(SubtitleDelivery::HlsVtt), None), &detail).is_none()
        );
    }

    #[test]
    fn subtitle_rendition_uses_track_language() {
        let detail = detail_with(
            "mp4",
            1,
            vec![],
            vec![],
            vec![subtitle(2, Some("en"), SubtitleFormat::Srt)],
        );
        let rendition = subtitle_rendition(
            &selected(Some(SubtitleDelivery::HlsVtt), Some(SubtitleTrackRef::Embedded(2))),
            &detail,
        )
        .unwrap();
        assert_eq!(rendition.language, "en");
        assert_eq!(rendition.name, "en");
    }

    #[test]
    fn subtitle_rendition_defaults_to_und() {
        let detail =
            detail_with("mp4", 1, vec![], vec![], vec![subtitle(3, None, SubtitleFormat::Srt)]);
        let embedded_missing = subtitle_rendition(
            &selected(Some(SubtitleDelivery::HlsVtt), Some(SubtitleTrackRef::Embedded(9))),
            &detail,
        )
        .unwrap();
        assert_eq!(embedded_missing.language, "und");

        let embedded_no_lang = subtitle_rendition(
            &selected(Some(SubtitleDelivery::HlsVtt), Some(SubtitleTrackRef::Embedded(3))),
            &detail,
        )
        .unwrap();
        assert_eq!(embedded_no_lang.language, "und");

        let file_ref = subtitle_rendition(
            &selected(
                Some(SubtitleDelivery::HlsVtt),
                Some(SubtitleTrackRef::File(SubtitleFileId("x".to_owned()))),
            ),
            &detail,
        )
        .unwrap();
        assert_eq!(file_ref.language, "und");
    }

    #[tokio::test]
    async fn reap_idle_delegates_to_transcode_manager() {
        let harness = Harness::new();
        assert_eq!(harness.service.reap_idle().await, 0);
        assert_eq!(harness.transcode.reaped(), 1);
    }

    // The reaper runs on a timer with nobody waiting on its result, so a session store that
    // is down must not stop the transcodes from being reaped.
    #[tokio::test]
    async fn a_session_store_that_is_down_still_reaps_the_transcodes() {
        let harness = Harness::new();
        harness.sessions.set_fail();

        assert_eq!(harness.service.reap_idle().await, 0);

        assert_eq!(harness.transcode.reaped(), 1);
    }

    #[tokio::test]
    async fn reap_idle_drops_sessions_that_stopped_heartbeating() {
        let harness = Harness::new();
        let mut stale = bare_session("stale");
        stale.last_heartbeat_at = Timestamp::UNIX_EPOCH;
        let mut live = bare_session("live");
        live.last_heartbeat_at = Timestamp::now();
        harness.sessions.insert(stale).await.unwrap();
        harness.sessions.insert(live).await.unwrap();

        harness.service.reap_idle().await;

        let left = harness.sessions.list_all(page()).await.unwrap();
        assert_eq!(left.items.len(), 1);
        assert_eq!(left.items[0].id, SessionId("live".to_owned()));
    }

    #[tokio::test]
    async fn a_reaped_session_stops_counting_against_the_stream_limit() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.users.insert(make_user(Some(1), None));
        let mut abandoned = bare_session("abandoned");
        abandoned.last_heartbeat_at = Timestamp::UNIX_EPOCH;
        harness.sessions.insert(abandoned).await.unwrap();

        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::ConcurrentLimit { .. })
        ));

        harness.service.reap_idle().await;

        assert!(harness.service.start(&principal(), start_request(0)).await.is_ok());
    }

    #[tokio::test]
    async fn a_reaped_session_leaves_nothing_behind_in_memory() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let mut stale = harness.sessions.get(&session).await.unwrap().unwrap();
        stale.last_heartbeat_at = Timestamp::UNIX_EPOCH;
        harness.sessions.insert(stale).await.unwrap();

        harness.service.reap_idle().await;

        assert!(harness.sessions.get(&session).await.unwrap().is_none());
        assert!(!harness.service.inner.cells.holds(&session));
        assert!(harness.streams.registered_generation(&session).is_none());
        assert_eq!(harness.transcode.stopped(), vec![session]);
    }

    #[tokio::test]
    async fn a_heartbeat_racing_an_end_saves_its_position_without_bringing_the_session_back() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.catalog.hold_next_version_read();
        let caller = principal();

        let (beat, ended) = tokio::join!(
            harness.service.heartbeat(&caller, &session, 30_000, PlaybackState::Paused),
            while_held(hold, harness.service.end(&caller, &session))
        );

        assert!(beat.is_ok() && ended.is_ok());
        assert!(
            harness.sessions.get(&session).await.unwrap().is_none(),
            "a session ended while its heartbeat was in flight stays ended"
        );
        assert_nothing_left_of(&harness, &session);
        let saved = harness
            .progress
            .get(&UserId("u1".to_owned()), &VersionId("v1".to_owned()))
            .await
            .unwrap()
            .expect("the last position sent on the way out is kept");
        assert_eq!(saved.position_ms, 30_000);
    }

    #[tokio::test]
    async fn a_version_in_an_ungranted_library_cannot_be_played() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.users.grant(&UserId("u1".to_owned()), &[]);

        let refused = harness.service.start(&principal(), start_request(0)).await.unwrap_err();

        assert!(
            matches!(refused, SessionError::VersionNotFound),
            "a caller who cannot see the library must not learn the version exists"
        );
        assert!(
            harness.transcode.started().is_empty(),
            "nothing may be spawned before the access check"
        );
    }

    fn mpaa(code: &str) -> ContentRating {
        ContentRating { system: "mpaa".to_owned(), code: code.to_owned() }
    }

    fn movie_m1() -> TitleId {
        TitleId::Movie(MovieId("m1".to_owned()))
    }

    #[tokio::test]
    async fn a_title_above_the_account_cap_cannot_be_played() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.catalog.set_rating(movie_m1(), mpaa("r"));
        harness
            .users
            .insert(User { max_content_rating: Some(mpaa("pg-13")), ..make_user(None, None) });

        let refused = harness.service.start(&principal(), start_request(0)).await.unwrap_err();

        assert!(matches!(refused, SessionError::VersionNotFound));
        assert!(harness.transcode.started().is_empty());
        assert!(harness.sessions.list_for_user(&UserId("u1".to_owned())).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_device_limit_refuses_a_title_above_it_and_plays_one_within_it() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.catalog.set_rating(movie_m1(), mpaa("r"));
        let child = Principal { age_limit: Some(12), ..principal() };

        assert!(matches!(
            harness.service.start(&child, start_request(0)).await,
            Err(SessionError::VersionNotFound)
        ));

        harness.catalog.set_rating(movie_m1(), mpaa("g"));
        assert!(harness.service.start(&child, start_request(0)).await.is_ok());
    }

    #[tokio::test]
    async fn an_admin_cannot_play_out_of_an_ungranted_library_either() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.users.grant(&UserId("u1".to_owned()), &[]);
        let boss = Principal {
            user: UserId("u1".to_owned()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        };

        assert!(matches!(
            harness.service.start(&boss, start_request(0)).await,
            Err(SessionError::VersionNotFound)
        ));
    }

    #[tokio::test]
    async fn start_direct_play() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
        assert!(started.manifest_url.ends_with("/file"));
        assert_eq!(started.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        assert!(harness.sessions.get(&started.session_id).await.unwrap().is_some());
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.mode, DeliveryMode::Direct);
        assert!(registration.direct_path.is_some());
        assert!(harness.transcode.started().is_empty());
        let claims = harness.tokens.verify(token_of(&started.manifest_url)).unwrap();
        assert_eq!(claims.session, started.session_id);
        assert_eq!(claims.version, VersionId("v1".to_owned()));
    }

    #[tokio::test]
    async fn starting_a_version_drops_progress_on_sibling_versions() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let mut sibling = direct_detail();
        sibling.version.id = VersionId("v2".to_owned());
        harness.catalog.insert(sibling);
        harness
            .progress
            .upsert(PlaybackProgress {
                user: UserId("u1".to_owned()),
                version: VersionId("v1".to_owned()),
                position_ms: 5_000,
                audio_track: None,
                subtitle: None,
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let request = SessionStartInput {
            version: VersionId("v2".to_owned()),
            start_position_ms: 0,
            capabilities: caps(None),
            audio: AudioRequest::Unspecified,
            subtitle: SubtitleRequest::Unspecified,
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
            delivery: DeliveryPreference::Auto,
        };
        harness.service.start(&principal(), request).await.unwrap();

        assert!(
            harness
                .progress
                .get(&UserId("u1".to_owned()), &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn start_transcode() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert!(started.manifest_url.ends_with("/master.m3u8"));
        let specs = harness.transcode.started();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].seek_ms, None);
        assert_eq!(specs[0].max_height, None);
        assert!(specs[0].burn_subtitle.is_none());
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.mode, DeliveryMode::Transcode);
        assert_eq!(
            registration.output_dir,
            PathBuf::from(format!("/mock/cache/{}/1", started.session_id.0)),
            "the directory names the generation, which is what makes it written once"
        );
        assert!(registration.direct_path.is_none());
    }

    #[tokio::test]
    async fn start_transcode_passes_seek_position() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        harness.service.start(&principal(), start_request(5_000)).await.unwrap();
        assert_eq!(harness.transcode.started()[0].seek_ms, Some(5_000));
    }

    fn video_cap(codec: &str, smooth: bool) -> VideoCodecCap {
        VideoCodecCap { codec: codec.to_owned(), max_level: None, max_bit_depth: 8, smooth }
    }

    fn reporting(decoding: ClientDecoding) -> SessionStartInput {
        SessionStartInput {
            capabilities: ClientCapabilities { decoding: Some(decoding), ..caps(None) },
            ..start_request(0)
        }
    }

    #[tokio::test]
    async fn a_codec_the_profile_never_claimed_direct_plays_when_the_client_reports_it() {
        // The measurement has to be able to earn direct play, not only lose it,
        // or a profile is still the ceiling and nothing was gained.
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding {
                    video: Some(vec![video_cap("vp9", true)]),
                    ..ClientDecoding::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
        assert!(harness.transcode.started().is_empty());
    }

    #[tokio::test]
    async fn a_codec_the_client_can_only_software_decode_is_transcoded() {
        // The client says it can decode h264 but not smoothly, and the server,
        // not the client, is what turns that into a transcode.
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding {
                    video: Some(vec![video_cap("h264", false)]),
                    ..ClientDecoding::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
    }

    #[tokio::test]
    async fn a_report_that_does_not_validate_falls_back_to_the_named_profile() {
        // A client bug must cost the report, never the playback.
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding { video: Some(Vec::new()), ..ClientDecoding::default() }),
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
    }

    #[tokio::test]
    async fn a_reported_frame_rate_ceiling_transcodes_high_frame_rate_video() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding { max_frame_rate: Some(24), ..ClientDecoding::default() }),
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);

        let mut fast = direct_detail();
        fast.video[0].frame_rate = 60.0;
        harness.catalog.insert(fast);
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding { max_frame_rate: Some(24), ..ClientDecoding::default() }),
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
    }

    #[tokio::test]
    async fn start_remux_when_only_container_unsupported() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mkv",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Remux);
        assert!(started.manifest_url.ends_with("/master.m3u8"));
        let spec = harness.transcode.started().remove(0);
        assert!(spec.copy);
        assert_eq!(spec.keyframes_us, Some(mocks::PREPARED_KEYFRAMES.to_vec()));
    }

    #[tokio::test]
    async fn a_sound_the_player_lacks_is_converted_while_the_video_is_copied() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![AudioTrack { codec: "truehd".to_owned(), channels: 8, ..audio(1) }],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Remux);
        assert_eq!(started.container, Some(SegmentContainer::MpegTs));
        let spec = harness.transcode.started().remove(0);
        assert!(spec.copy);
        assert!(spec.convert_audio);
        assert_eq!(spec.audio_channels, Some(2));
        assert_eq!(spec.container, SegmentContainer::MpegTs);
    }

    #[tokio::test]
    async fn a_remux_that_keeps_its_sound_picks_the_container_for_it() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mkv",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![AudioTrack { codec: "truehd".to_owned(), ..audio(1) }],
            vec![],
        ));
        let started = harness
            .service
            .start(
                &principal(),
                SessionStartInput {
                    delivery: DeliveryPreference::NeverConvert,
                    ..start_request(0)
                },
            )
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Remux);
        assert_eq!(started.container, Some(SegmentContainer::MpegTs));
        assert!(!harness.transcode.started()[0].convert_audio);
    }

    fn remuxable() -> VersionDetail {
        detail_with("mkv", 100_000, vec![video("h264", Some(5_000_000))], vec![audio(1)], vec![])
    }

    #[tokio::test]
    async fn a_version_not_prepared_yet_is_transcoded_instead_of_remuxed() {
        for keyframes in [None, Some(Vec::new())] {
            let harness = Harness::new();
            harness.catalog.insert(remuxable());
            harness.catalog.set_keyframes(&VersionId("v1".to_owned()), keyframes);

            let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

            assert_eq!(started.mode, DeliveryMode::Transcode);
            assert_eq!(started.container, Some(SegmentContainer::MpegTs));
            let spec = harness.transcode.started().remove(0);
            assert!(!spec.copy);
            assert_eq!(spec.keyframes_us, None);
        }
    }

    #[tokio::test]
    async fn a_version_prepared_mid_session_remuxes_from_the_next_seek() {
        let harness = Harness::new();
        harness.catalog.insert(remuxable());
        let version = VersionId("v1".to_owned());
        harness.catalog.set_keyframes(&version, None);
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);

        harness.catalog.set_keyframes(&version, Some(vec![0, 6_000_000]));
        let sought = harness.service.seek(&principal(), &started.session_id, 30_000).await.unwrap();

        assert_eq!(sought.mode, DeliveryMode::Remux);
        assert_eq!(harness.transcode.started()[1].keyframes_us, Some(vec![0, 6_000_000]));
    }

    fn with_unextracted_track() -> VersionDetail {
        VersionDetail {
            extracted_subtitles: Vec::new(),
            ..detail_with(
                "mp4",
                100_000,
                vec![video("h264", Some(5_000_000))],
                vec![audio(1)],
                vec![subtitle(2, Some("en"), SubtitleFormat::Srt)],
            )
        }
    }

    fn embedded_track(index: u32) -> SubtitleSelection {
        SubtitleSelection { track: SubtitleTrackRef::Embedded(index), offset_ms: None }
    }

    #[tokio::test]
    async fn an_embedded_track_not_extracted_yet_is_not_played_at_the_start() {
        let harness = Harness::new();
        harness.catalog.insert(with_unextracted_track());
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(embedded_track(2));

        let started = harness.service.start(&principal(), request).await.unwrap();

        assert_eq!(started.mode, DeliveryMode::Direct);
        assert_eq!(started.selected.subtitle_track, None);
        assert!(harness.transcode.started().is_empty());
    }

    #[tokio::test]
    async fn an_embedded_track_not_extracted_yet_cannot_be_switched_to() {
        let harness = Harness::new();
        harness.catalog.insert(with_unextracted_track());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        let switched = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Set(embedded_track(2)),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();

        assert_eq!(switched.mode, DeliveryMode::Direct);
        assert_eq!(switched.selected.subtitle_track, None);
    }

    #[tokio::test]
    async fn start_burns_image_subtitle() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, None, SubtitleFormat::Pgs)],
        ));
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(
            harness.transcode.started()[0].burn_subtitle,
            Some(BurnSubtitle::EmbeddedImage { index: 2, width: 1920, height: 1080 })
        );
        assert!(harness.streams.registration(&started.session_id).unwrap().subtitle.is_none());
    }

    #[test]
    fn a_burn_needs_a_known_track_and_an_image_burn_a_video_to_size_it() {
        let burned = |index| {
            selected(Some(SubtitleDelivery::Burned), Some(SubtitleTrackRef::Embedded(index)))
        };
        let image = vec![subtitle(2, None, SubtitleFormat::VobSub)];
        let detail = detail_with("mkv", 1, vec![video("h264", None)], vec![], image.clone());
        assert_eq!(
            burn_subtitle_for(&burned(2), &detail),
            Some(BurnSubtitle::EmbeddedImage { index: 2, width: 1920, height: 1080 })
        );
        assert_eq!(burn_subtitle_for(&burned(5), &detail), None);
        let soft = selected(Some(SubtitleDelivery::HlsVtt), Some(SubtitleTrackRef::Embedded(2)));
        assert_eq!(burn_subtitle_for(&soft, &detail), None);
        let no_video = detail_with("mkv", 1, vec![], vec![], image);
        assert_eq!(burn_subtitle_for(&burned(2), &no_video), None);
    }

    fn sidecar() -> SubtitleFile {
        SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/m1.en.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }
    }

    async fn start_direct_with_sidecar(harness: &Harness) -> SessionStarted {
        let mut detail = detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        );
        detail.subtitle_files = vec![sidecar()];
        harness.catalog.insert(detail);
        let mut request = start_request(0);
        request.capabilities.tracks = TrackHandling { external: true, ..TrackHandling::default() };
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            offset_ms: None,
        });
        harness.service.start(&principal(), request).await.unwrap()
    }

    #[tokio::test]
    async fn a_subtitle_file_in_direct_play_is_prepared_and_its_address_answered() {
        let harness = Harness::new();

        let started = start_direct_with_sidecar(&harness).await;

        assert_eq!(started.mode, DeliveryMode::Direct);
        assert_eq!(started.selected.subtitle_delivery, Some(SubtitleDelivery::External));
        let url = started.selected.subtitle_url.clone().unwrap();
        assert!(url.starts_with("/stream/") && url.ends_with("/subtitles.vtt"), "{url}");
        let prepared = harness.transcode.subtitle_files();
        assert_eq!(prepared.len(), 1);
        assert_eq!(
            prepared[0].subtitle.source,
            SoftSubtitleSource::File("/media/m1.en.srt".into())
        );
        assert_eq!(prepared[0].subtitle.delivery, SubtitleDelivery::External);
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(
            registration.subtitle_file,
            Some(PathBuf::from(format!("/mock/cache/{}/1/subtitles.vtt", started.session_id.0)))
        );
    }

    #[tokio::test]
    async fn a_subtitle_file_that_cannot_be_prepared_leaves_direct_play_without_it() {
        let harness = Harness::new();
        harness.transcode.set_subtitles_missing();

        let started = start_direct_with_sidecar(&harness).await;

        assert_eq!(started.mode, DeliveryMode::Direct);
        assert_eq!(started.selected.subtitle_url, None);
        assert_eq!(harness.transcode.subtitle_files().len(), 1);
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.subtitle_file, None);
    }

    #[tokio::test]
    async fn start_burns_file_subtitle_from_sidecar_path() {
        let harness = Harness::new();
        let mut detail = detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        );
        detail.subtitle_files = vec![SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/m1.en.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }];
        harness.catalog.insert(detail);
        let mut request = start_request(0);
        request.force_burn = true;
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(
            harness.transcode.started()[0].burn_subtitle,
            Some(BurnSubtitle::File("/media/m1.en.srt".to_owned()))
        );
    }

    #[tokio::test]
    async fn start_registers_text_subtitle_rendition() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, Some("fr"), SubtitleFormat::Srt)],
        ));
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.subtitle.unwrap().language, "fr");
    }

    #[tokio::test]
    async fn a_subtitle_rendition_that_was_not_written_is_not_advertised() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, Some("fr"), SubtitleFormat::Srt)],
        ));
        harness.transcode.set_subtitles_missing();
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();

        assert!(harness.transcode.started()[0].soft_subtitle.is_some());
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert!(registration.subtitle.is_none());
    }

    #[tokio::test]
    async fn start_soft_subtitle_from_file_then_offset_persists() {
        let harness = Harness::new();
        let mut detail = detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        );
        detail.subtitle_files = vec![SubtitleFile {
            id: SubtitleFileId("sf1".into()),
            version: VersionId("v1".into()),
            language: Some(LanguageCode("en".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/m1.en.srt".into(),
            translated_from: None,
            label: None,
            pinned: false,
        }];
        harness.catalog.insert(detail);
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();

        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.subtitle.unwrap().language, "en");
        let spec = harness.transcode.started().last().cloned().unwrap();
        assert_eq!(
            spec.soft_subtitle,
            Some(SoftSubtitle {
                source: SoftSubtitleSource::File("/media/m1.en.srt".into()),
                offset_ms: 0,
                delivery: SubtitleDelivery::HlsVtt,
            })
        );

        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
                        offset_ms: Some(2_000),
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        let spec = harness.transcode.started().last().cloned().unwrap();
        assert_eq!(spec.soft_subtitle.unwrap().offset_ms, 2_000);
        let stored = harness
            .preferences
            .get_subtitle_offset(
                &principal().user,
                &VersionId("v1".into()),
                &SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.offset_ms, 2_000);
    }

    #[tokio::test]
    async fn start_rejects_unknown_container() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "rmvb",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::NegotiationFailed)
        ));
        assert!(harness.sessions.list_all(page()).await.unwrap().items.is_empty());
    }

    // The scanner ingests these, so a failure here is a title the user can see but never play.
    #[tokio::test]
    async fn start_remuxes_the_containers_no_client_can_play_directly() {
        for container in ["avi", "mov", "m4v", "wmv", "flv", "mpg", "mpeg", "m2ts"] {
            let harness = Harness::new();
            harness.catalog.insert(detail_with(
                container,
                100_000,
                vec![video("h264", Some(5_000_000))],
                vec![audio(1)],
                vec![],
            ));

            let started = harness
                .service
                .start(&principal(), start_request(0))
                .await
                .unwrap_or_else(|e| panic!("[{container}] should negotiate, got [{e:?}]"));

            assert_eq!(started.mode, DeliveryMode::Remux, "{container}");
        }
    }

    #[tokio::test]
    async fn start_transcode_failure_maps_to_negotiation_failed() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        harness.transcode.set_fail();
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::NegotiationFailed)
        ));
        assert!(harness.sessions.list_all(page()).await.unwrap().items.is_empty());
        assert!(harness.service.inner.cells.is_empty());
    }

    #[tokio::test]
    async fn start_token_failure_maps_to_negotiation_failed() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.tokens.set_fail();
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::NegotiationFailed)
        ));
        assert!(harness.sessions.list_all(page()).await.unwrap().items.is_empty());
        assert!(harness.transcode.started().is_empty());
        assert!(harness.service.inner.cells.is_empty());
    }

    #[tokio::test]
    async fn a_session_that_cannot_be_saved_leaves_nothing_running() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        harness.sessions.set_fail();

        assert!(harness.service.start(&principal(), start_request(0)).await.is_err());

        let session = harness.transcode.started()[0].session.clone();
        assert_eq!(harness.transcode.stopped(), vec![session.clone()]);
        assert!(harness.streams.registration(&session).is_none());
        assert!(harness.service.inner.cells.is_empty());
    }

    #[tokio::test]
    async fn start_version_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::VersionNotFound)
        ));
    }

    #[tokio::test]
    async fn start_rejects_unavailable_version() {
        let harness = Harness::new();
        let mut detail = direct_detail();
        detail.version.available = false;
        harness.catalog.insert(detail);
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::VersionNotFound)
        ));
    }

    #[tokio::test]
    async fn start_surfaces_user_lookup_error() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.users.set_fail();
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::Repository(_))
        ));
    }

    #[tokio::test]
    async fn start_enforces_concurrent_limit() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.users.insert(make_user(Some(1), None));
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        match harness.service.start(&principal(), start_request(0)).await {
            Err(SessionError::ConcurrentLimit { active }) => assert_eq!(active.len(), 1),
            other => panic!("expected ConcurrentLimit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn start_applies_user_bitrate_cap() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        harness.users.insert(make_user(None, Some(1_000_000)));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(harness.transcode.started()[0].max_bitrate, Some(1_000_000));
    }

    fn multilingual_detail() -> VersionDetail {
        detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio_in(0, Some("en")), audio_in(1, Some("ja")), audio_in(2, Some("fr"))],
            vec![
                subtitle(0, Some("en"), SubtitleFormat::Srt),
                subtitle(1, Some("fr"), SubtitleFormat::Srt),
            ],
        )
    }

    fn user_who_prefers(audio: &[&str], subtitle: &[&str]) -> User {
        let mut user = make_user(None, None);
        user.preferred_audio = audio.iter().map(|c| LanguageCode((*c).to_owned())).collect();
        user.preferred_subtitle = subtitle.iter().map(|c| LanguageCode((*c).to_owned())).collect();
        user
    }

    #[tokio::test]
    async fn start_honours_the_users_preferred_audio_and_subtitle_languages() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        harness.users.insert(user_who_prefers(&["ja"], &["fr"]));

        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        assert_eq!(started.selected.audio_track, Some(1));
        assert_eq!(started.selected.subtitle_track, Some(SubtitleTrackRef::Embedded(1)));
    }

    #[tokio::test]
    async fn an_explicit_request_beats_the_users_language_preferences() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        harness.users.insert(user_who_prefers(&["ja"], &["fr"]));
        let mut request = start_request(0);
        request.audio = AudioRequest::Track(2);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(0),
            offset_ms: None,
        });

        let started = harness.service.start(&principal(), request).await.unwrap();

        assert_eq!(started.selected.audio_track, Some(2));
        assert_eq!(started.selected.subtitle_track, Some(SubtitleTrackRef::Embedded(0)));
    }

    #[tokio::test]
    async fn a_language_with_no_matching_track_leaves_the_default_alone() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        harness.users.insert(user_who_prefers(&["de"], &["de"]));

        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        assert_eq!(started.selected.audio_track, Some(0));
        assert_eq!(started.selected.subtitle_track, None);
    }

    #[tokio::test]
    async fn start_forces_quality_rung_scales_and_transcodes() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let mut request = start_request(0);
        request.target_height = Some(720);
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(harness.transcode.started()[0].max_height, Some(720));
    }

    fn sized(width: u32, height: u32) -> VideoTrack {
        VideoTrack { width, height, ..video("h264", None) }
    }

    #[test]
    fn a_transcode_fits_the_picture_inside_both_limits() {
        assert_eq!(fitted_height(&sized(2582, 1080), 1920, 1080), Some(802));
        assert_eq!(fitted_height(&sized(2582, 1080), 2560, 1440), Some(1070));
        assert_eq!(fitted_height(&sized(1920, 700), 1280, 720), Some(466));
        assert_eq!(fitted_height(&sized(3840, 2160), 1920, 1080), Some(1080));
        assert_eq!(fitted_height(&sized(2582, 1080), 3840, 720), Some(720));
    }

    #[test]
    fn a_picture_inside_both_limits_is_not_scaled() {
        assert_eq!(fitted_height(&sized(1920, 1080), 1920, 1080), None);
        assert_eq!(fitted_height(&sized(720, 400), 1920, 1080), None);
        assert_eq!(fitted_height(&sized(0, 0), 1920, 1080), None);
    }

    #[tokio::test]
    async fn a_picture_wider_than_the_client_is_transcoded_to_its_width() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![sized(2582, 1080)],
            vec![audio(1)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(harness.transcode.started()[0].max_height, Some(802));
    }

    #[tokio::test]
    async fn a_started_session_reports_the_container_it_resolved() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(started.container, Some(SegmentContainer::MpegTs));
    }

    // Nothing is produced for a direct session, so there is no container to name.
    #[tokio::test]
    async fn a_direct_session_reports_no_container() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
        assert_eq!(started.container, None);
    }

    #[tokio::test]
    async fn always_convert_transcodes_a_version_that_would_direct_play() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let mut request = start_request(0);
        request.delivery = DeliveryPreference::AlwaysConvert;
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
    }

    // The preference has to outlive the request that set it, or the next track
    // change would quietly put the session back the way it was.
    #[tokio::test]
    async fn the_delivery_preference_survives_an_update() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let mut request = start_request(0);
        request.delivery = DeliveryPreference::AlwaysConvert;
        let started = harness.service.start(&principal(), request).await.unwrap();

        let again = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::AlwaysConvert,
                },
            )
            .await
            .unwrap();

        assert_eq!(again.mode, DeliveryMode::Transcode);
    }

    // The operator bound exists because a host without a hardware encoder cannot
    // produce 4K in realtime; it must never touch a session that is not re-encoding.
    #[tokio::test]
    async fn a_transcode_bound_lowers_the_output_height() {
        let harness = Harness::bounded(Some(720));
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(harness.transcode.started()[0].max_height, Some(720));
    }

    #[tokio::test]
    async fn a_transcode_bound_above_the_source_changes_nothing() {
        let harness = Harness::bounded(Some(2160));
        harness.catalog.insert(transcode_detail());
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(harness.transcode.started()[0].max_height, None);
    }

    #[tokio::test]
    async fn a_transcode_bound_never_shrinks_a_remux() {
        let harness = Harness::bounded(Some(480));
        harness.catalog.insert(detail_with(
            "mkv",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Remux);
        assert_eq!(harness.transcode.started()[0].max_height, None);
    }

    #[tokio::test]
    async fn update_forces_burn_in_of_selected_subtitle() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, Some("en"), SubtitleFormat::Srt)],
        ));
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));
        let renegotiated = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: true,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        assert_eq!(renegotiated.mode, DeliveryMode::Transcode);
        assert_eq!(renegotiated.selected.subtitle_delivery, Some(SubtitleDelivery::Burned));
        assert_eq!(
            harness.transcode.started().last().unwrap().burn_subtitle,
            Some(BurnSubtitle::EmbeddedText(2))
        );
    }

    #[tokio::test]
    async fn update_downmix_converts_the_sound_to_stereo() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![AudioTrack { channels: 6, ..audio(1) }],
            vec![],
        ));
        let started = harness
            .service
            .start(
                &principal(),
                reporting(ClientDecoding {
                    audio: Some(vec![AudioCodecCap { codec: "aac".to_owned(), max_channels: 8 }]),
                    ..ClientDecoding::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(harness.transcode.started().last().unwrap().audio_channels, None);
        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: true,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        assert_eq!(harness.transcode.started().last().unwrap().audio_channels, Some(2));
    }

    #[tokio::test]
    async fn heartbeat_records_completion_at_ninety_percent() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let ack = harness
            .service
            .heartbeat(&principal(), &started.session_id, 90_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert_eq!(ack.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        assert!(
            harness
                .progress
                .get(&principal().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .is_none()
        );
        let history = harness.progress.history(&principal().user, page()).await.unwrap();
        assert_eq!(history.total, 1);
        assert!(history.items[0].completed);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn a_heartbeat_that_changes_the_state_logs_the_change() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 5_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert!(!logs_contain("went from"));

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 6_000, PlaybackState::Paused)
            .await
            .unwrap();
        assert!(logs_contain(&format!(
            "session [{}] for version [v1] went from [playing] to [paused] at [6000] ms",
            started.session_id.0
        )));
    }

    #[tokio::test]
    async fn finishing_a_title_takes_it_off_the_watchlist() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let title = TitleId::Movie(MovieId("m1".to_owned()));
        harness
            .preferences
            .add_watchlist(WatchlistItem {
                user: principal().user.clone(),
                title: title.clone(),
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 90_000, PlaybackState::Playing)
            .await
            .unwrap();

        assert!(
            harness.preferences.list_watchlist(&principal().user).await.unwrap().is_empty(),
            "watching a title to the end is the clearest signal it can leave the queue"
        );
    }

    #[tokio::test]
    async fn a_partly_watched_title_stays_on_the_watchlist() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness
            .preferences
            .add_watchlist(WatchlistItem {
                user: principal().user.clone(),
                title: TitleId::Movie(MovieId("m1".to_owned())),
                added_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 30_000, PlaybackState::Playing)
            .await
            .unwrap();

        assert_eq!(harness.preferences.list_watchlist(&principal().user).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn every_heartbeat_past_the_threshold_counts_one_view() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        for position in [90_000, 93_000, 96_000, 99_000, 100_000] {
            harness
                .service
                .heartbeat(&principal(), &started.session_id, position, PlaybackState::Playing)
                .await
                .unwrap();
        }

        let history = harness.progress.history(&principal().user, page()).await.unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].play_count, 1);
        assert!(
            harness
                .progress
                .get(&principal().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .is_none(),
            "a finished title must not offer to resume at the end of itself"
        );
    }

    #[tokio::test]
    async fn a_second_session_on_the_same_title_counts_a_second_view() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        for _ in 0..2 {
            let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
            harness
                .service
                .heartbeat(&principal(), &started.session_id, 99_000, PlaybackState::Playing)
                .await
                .unwrap();
        }

        let history = harness.progress.history(&principal().user, page()).await.unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].play_count, 2);
    }

    #[tokio::test]
    async fn heartbeat_clears_a_resume_point_once_complete() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 42_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert_eq!(
            harness
                .progress
                .get(&principal().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .unwrap()
                .position_ms,
            42_000
        );

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 99_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert!(
            harness
                .progress
                .get(&principal().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn heartbeat_no_completion_below_threshold() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 89_000, PlaybackState::Paused)
            .await
            .unwrap();
        assert_eq!(harness.progress.history(&principal().user, page()).await.unwrap().total, 0);
    }

    fn feature_length() -> VersionDetail {
        detail_with("mp4", 7_200_000, vec![video("h264", Some(5_000_000))], vec![audio(1)], vec![])
    }

    async fn beat_at(harness: &Harness, position_ms: u64) {
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, position_ms, PlaybackState::Playing)
            .await
            .unwrap();
    }

    async fn stored_progress(harness: &Harness) -> Option<PlaybackProgress> {
        harness.progress.get(&principal().user, &VersionId("v1".to_owned())).await.unwrap()
    }

    #[tokio::test]
    async fn a_barely_started_title_is_not_worth_resuming() {
        let harness = Harness::new();
        harness.catalog.insert(feature_length());
        beat_at(&harness, 30_000).await;
        assert!(
            stored_progress(&harness).await.is_none(),
            "thirty seconds of a two hour film is below the resume floor"
        );
    }

    #[tokio::test]
    async fn passing_the_resume_floor_stores_progress() {
        let harness = Harness::new();
        harness.catalog.insert(feature_length());
        beat_at(&harness, 60_000).await;
        assert_eq!(stored_progress(&harness).await.map(|p| p.position_ms), Some(60_000));
    }

    #[tokio::test]
    async fn seeking_back_to_the_start_moves_the_bookmark_but_keeps_the_row() {
        let harness = Harness::new();
        harness.catalog.insert(feature_length());
        beat_at(&harness, 600_000).await;
        assert!(stored_progress(&harness).await.is_some());

        beat_at(&harness, 5_000).await;
        let row = stored_progress(&harness).await;
        assert_eq!(
            row.map(|row| row.position_ms),
            Some(5_000),
            "rewinding moves the bookmark rather than destroying the viewer's history"
        );
    }

    async fn track_choice(harness: &Harness, session: &SessionId, track: u32) {
        harness
            .service
            .update(
                &principal(),
                session,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: Some(track),
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::Embedded(1),
                        offset_ms: None,
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
    }

    // A choice made in the first minute must survive, which is why it forces a row
    // of its own rather than waiting for the started threshold.
    #[tokio::test]
    async fn changing_tracks_early_writes_the_override_immediately() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        track_choice(&harness, &started.session_id, 2).await;

        let row = stored_progress(&harness).await.expect("row forced");
        assert_eq!(row.audio_track, Some(2));
        assert_eq!(row.subtitle, Some(SubtitleOverride::Track(SubtitleTrackRef::Embedded(1))));
    }

    #[tokio::test]
    async fn a_stored_override_is_restored_on_the_next_session() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        harness.users.insert(user_who_prefers(&["en"], &["en"]));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        track_choice(&harness, &started.session_id, 2).await;
        harness.service.end(&principal(), &started.session_id).await.unwrap();

        let again = harness.service.start(&principal(), start_request(0)).await.unwrap();

        assert_eq!(again.selected.audio_track, Some(2));
        assert_eq!(again.selected.subtitle_track, Some(SubtitleTrackRef::Embedded(1)));
    }

    // The account preference is applied but never hardened, or it would stop
    // tracking the account setting from the next episode onward.
    #[tokio::test]
    async fn the_account_preference_never_becomes_an_override() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        harness.users.insert(user_who_prefers(&["ja"], &["fr"]));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.selected.audio_track, Some(1));

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 50_000, PlaybackState::Playing)
            .await
            .unwrap();

        let row = stored_progress(&harness).await.expect("started");
        assert_eq!(row.audio_track, None);
        assert_eq!(row.subtitle, None);
    }

    #[tokio::test]
    async fn finishing_a_version_drops_its_override() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        track_choice(&harness, &started.session_id, 2).await;
        assert!(stored_progress(&harness).await.is_some());

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 95_000, PlaybackState::Playing)
            .await
            .unwrap();

        assert!(stored_progress(&harness).await.is_none());
    }

    // A track the file does not have must leave the current choice alone rather than
    // selecting nothing, which is how an unknown index used to produce silence.
    #[tokio::test]
    async fn updating_to_a_subtitle_track_that_is_absent_keeps_the_current_one() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        let mut request = start_request(0);
        request.subtitle = SubtitleRequest::Track(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(1),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();

        let renegotiated = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::Embedded(9),
                        offset_ms: Some(250),
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();

        assert_eq!(renegotiated.selected.subtitle_track, Some(SubtitleTrackRef::Embedded(1)));
        assert!(
            harness
                .preferences
                .get_subtitle_offset(
                    &principal().user,
                    &VersionId("v1".to_owned()),
                    &SubtitleTrackRef::Embedded(9),
                )
                .await
                .unwrap()
                .is_none(),
            "an offset must not be stored against a track the file does not have"
        );
    }

    // Rewinding below the threshold must not throw the choice away with the bookmark.
    #[tokio::test]
    async fn an_override_survives_a_rewind_to_the_start() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        track_choice(&harness, &started.session_id, 2).await;

        harness
            .service
            .heartbeat(&principal(), &started.session_id, 0, PlaybackState::Paused)
            .await
            .unwrap();

        let row = stored_progress(&harness).await.expect("row kept");
        assert_eq!(row.position_ms, 0);
        assert_eq!(row.audio_track, Some(2));
    }

    #[tokio::test]
    async fn a_title_barely_sampled_leaves_no_bookmark() {
        let harness = Harness::new();
        harness.catalog.insert(feature_length());
        beat_at(&harness, 5_000).await;
        assert!(
            stored_progress(&harness).await.is_none(),
            "ten seconds of something new is not worth remembering"
        );
    }

    #[tokio::test]
    async fn heartbeat_zero_duration_never_completes() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            0,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 10_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert_eq!(harness.progress.history(&principal().user, page()).await.unwrap().total, 0);
    }

    #[tokio::test]
    async fn a_heartbeat_never_loads_the_full_version_detail() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let after_start = harness.catalog.detail_lookup_count();

        for _ in 0..5 {
            harness
                .service
                .heartbeat(&principal(), &started.session_id, 1_000, PlaybackState::Playing)
                .await
                .unwrap();
        }

        assert_eq!(
            harness.catalog.detail_lookup_count(),
            after_start,
            "a heartbeat needs duration and title, both columns on the version row, \
             so it must not pull tracks, chapters, markers and trickplay"
        );
    }

    #[tokio::test]
    async fn heartbeat_touches_active_transcode() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 1_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert_eq!(harness.transcode.touched(), vec![started.session_id]);
    }

    #[tokio::test]
    async fn heartbeat_tolerates_missing_version() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 95_000, PlaybackState::Playing)
            .await
            .unwrap();
        assert_eq!(harness.progress.history(&principal().user, page()).await.unwrap().total, 0);
    }

    #[tokio::test]
    async fn heartbeat_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness
                .service
                .heartbeat(&principal(), &SessionId("nope".to_owned()), 0, PlaybackState::Playing)
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_issues_fresh_url_and_token() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let first_url = started.manifest_url.clone();
        let renegotiated =
            harness.service.seek(&principal(), &started.session_id, 30_000).await.unwrap();
        assert_ne!(renegotiated.manifest_url, first_url);
        let claims = harness.tokens.verify(token_of(&renegotiated.manifest_url)).unwrap();
        assert_eq!(claims.session, started.session_id);
        assert_eq!(claims.version, VersionId("v1".to_owned()));
        assert_eq!(
            harness.sessions.get(&started.session_id).await.unwrap().unwrap().position_ms,
            30_000
        );
    }

    #[tokio::test]
    async fn an_update_starts_the_new_stream_where_the_viewer_is() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .service
            .heartbeat(&principal(), &started.session_id, 60_000, PlaybackState::Playing)
            .await
            .unwrap();

        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate { position_ms: 5_000, ..keep_everything() },
            )
            .await
            .unwrap();

        assert_eq!(harness.transcode.started().last().unwrap().seek_ms, Some(5_000));
        assert_eq!(
            harness.sessions.get(&started.session_id).await.unwrap().unwrap().position_ms,
            5_000
        );
    }

    #[tokio::test]
    async fn seek_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness.service.seek(&principal(), &SessionId("nope".to_owned()), 0).await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_missing_context_is_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.sessions.insert(bare_session("s1")).await.unwrap();
        assert!(matches!(
            harness.service.seek(&principal(), &SessionId("s1".to_owned()), 0).await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_version_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        assert!(matches!(
            harness.service.seek(&principal(), &started.session_id, 10_000).await,
            Err(SessionError::VersionNotFound)
        ));
    }

    #[tokio::test]
    async fn update_changes_audio_track() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1), audio(2)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let renegotiated = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: Some(2),
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        assert_eq!(renegotiated.selected.audio_track, Some(2));
    }

    #[tokio::test]
    async fn update_sets_then_disables_subtitle() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, Some("en"), SubtitleFormat::Srt)],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let set = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::Embedded(2),
                        offset_ms: None,
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        assert_eq!(set.selected.subtitle_delivery, Some(SubtitleDelivery::HlsVtt));
        let disabled = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    position_ms: 0,
                    audio_track: None,
                    subtitle: SubtitleChange::Disable,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                    delivery: DeliveryPreference::Auto,
                },
            )
            .await
            .unwrap();
        assert_eq!(disabled.selected.subtitle_delivery, None);
    }

    #[tokio::test]
    async fn update_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness
                .service
                .update(
                    &principal(),
                    &SessionId("nope".to_owned()),
                    SessionUpdate {
                        position_ms: 0,
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
                        delivery: DeliveryPreference::Auto,
                    },
                )
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn update_missing_context_is_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.sessions.insert(bare_session("s1")).await.unwrap();
        assert!(matches!(
            harness
                .service
                .update(
                    &principal(),
                    &SessionId("s1".to_owned()),
                    SessionUpdate {
                        position_ms: 0,
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
                        delivery: DeliveryPreference::Auto,
                    },
                )
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn update_version_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        assert!(matches!(
            harness
                .service
                .update(
                    &principal(),
                    &started.session_id,
                    SessionUpdate {
                        position_ms: 0,
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
                        delivery: DeliveryPreference::Auto,
                    },
                )
                .await,
            Err(SessionError::VersionNotFound)
        ));
    }

    #[tokio::test]
    async fn end_stops_transcode_and_removes_session() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.service.end(&principal(), &started.session_id).await.unwrap();
        assert_eq!(harness.transcode.stopped(), vec![started.session_id.clone()]);
        assert_eq!(harness.streams.removed(), vec![started.session_id.clone()]);
        assert!(harness.sessions.get(&started.session_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn end_all_for_user_tears_down_the_stream_not_just_the_session_row() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        harness.service.end_all_for_user(&UserId("u1".to_owned())).await.unwrap();

        assert_eq!(
            harness.streams.removed(),
            vec![started.session_id.clone()],
            "the stream registration is what makes playback stop"
        );
        assert_eq!(harness.transcode.stopped(), vec![started.session_id.clone()]);
        assert!(harness.sessions.get(&started.session_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn end_all_for_user_clears_a_registry_entry_an_earlier_teardown_left_behind() {
        let harness = Harness::new();
        harness.sessions.insert(bare_session("left")).await.unwrap();

        harness.service.end_all_for_user(&UserId("u1".to_owned())).await.unwrap();

        assert!(harness.sessions.get(&SessionId("left".to_owned())).await.unwrap().is_none());
        assert_eq!(harness.streams.removed(), vec![SessionId("left".to_owned())]);
    }

    #[tokio::test]
    async fn end_all_for_a_user_with_nothing_playing_is_a_no_op() {
        let harness = Harness::new();

        harness.service.end_all_for_user(&UserId("nobody".to_owned())).await.unwrap();

        assert!(harness.streams.removed().is_empty());
    }

    #[tokio::test]
    async fn end_direct_session_ignores_missing_transcode() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.service.end(&principal(), &started.session_id).await.unwrap();
        assert!(harness.transcode.stopped().is_empty());
        assert_eq!(harness.streams.removed(), vec![started.session_id]);
    }

    #[tokio::test]
    async fn end_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness.service.end(&principal(), &SessionId("nope".to_owned())).await,
            Err(SessionError::NotFound)
        ));
    }

    fn intruder() -> Principal {
        Principal {
            user: UserId("u2".to_owned()),
            role: Role::User,
            account_admin: false,
            device: None,
            age_limit: None,
        }
    }

    fn admin() -> Principal {
        Principal {
            user: UserId("boss".to_owned()),
            role: Role::Admin,
            account_admin: true,
            device: None,
            age_limit: None,
        }
    }

    #[tokio::test]
    async fn a_heartbeat_from_another_user_moves_nothing() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        assert!(matches!(
            harness
                .service
                .heartbeat(&intruder(), &started.session_id, 42_000, PlaybackState::Playing)
                .await,
            Err(SessionError::NotFound)
        ));
        assert_eq!(
            harness.sessions.get(&started.session_id).await.unwrap().unwrap().position_ms,
            0,
            "a stranger's heartbeat must not move the owner's playhead"
        );
        assert!(
            harness
                .progress
                .get(&intruder().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .is_none(),
            "nor write a resume point onto the stranger's own account"
        );
    }

    #[tokio::test]
    async fn seeking_another_users_session_is_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert!(matches!(
            harness.service.seek(&intruder(), &started.session_id, 42_000).await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn updating_another_users_session_is_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert!(matches!(
            harness
                .service
                .update(
                    &intruder(),
                    &started.session_id,
                    SessionUpdate {
                        position_ms: 0,
                        audio_track: Some(1),
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
                        delivery: DeliveryPreference::Auto,
                    },
                )
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn ending_another_users_session_leaves_it_playing() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        assert!(matches!(
            harness.service.end(&intruder(), &started.session_id).await,
            Err(SessionError::NotFound)
        ));
        assert!(
            harness.sessions.get(&started.session_id).await.unwrap().is_some(),
            "the owner's session survives a stranger's delete"
        );
        assert!(harness.transcode.stopped().is_empty(), "and their transcode keeps running");
    }

    #[tokio::test]
    async fn a_non_admin_sees_only_their_own_active_sessions() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .sessions
            .insert(PlaybackSession { user: intruder().user, ..bare_session("theirs") })
            .await
            .unwrap();

        let listing = harness.service.active_sessions(&principal(), page()).await.unwrap();

        assert_eq!(listing.total, 1);
        assert!(listing.items.iter().all(|s| s.user == principal().user));
    }

    #[tokio::test]
    async fn an_admin_sees_every_active_session() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness
            .sessions
            .insert(PlaybackSession { user: intruder().user, ..bare_session("theirs") })
            .await
            .unwrap();

        let listing = harness.service.active_sessions(&admin(), page()).await.unwrap();

        assert_eq!(listing.total, 2);
    }

    #[tokio::test]
    async fn active_sessions_lists_all() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.service.start(&principal(), start_request(0)).await.unwrap();
        let listing = harness.service.active_sessions(&principal(), page()).await.unwrap();
        assert_eq!(listing.total, 2);
    }

    #[tokio::test]
    async fn clone_shares_inner_state() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let cloned = harness.service.clone();
        let listing = cloned.active_sessions(&principal(), page()).await.unwrap();
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].id, started.session_id);
    }

    fn keep_everything() -> SessionUpdate {
        SessionUpdate {
            position_ms: 0,
            audio_track: None,
            subtitle: SubtitleChange::Keep,
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
            delivery: DeliveryPreference::Auto,
        }
    }

    #[tokio::test]
    async fn every_renegotiation_launches_a_new_generation() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.service.update(&principal(), &started.session_id, keep_everything()).await.unwrap();
        harness.service.seek(&principal(), &started.session_id, 5_000).await.unwrap();

        let generations: Vec<u32> =
            harness.transcode.started().iter().map(|spec| spec.generation.0).collect();
        assert_eq!(
            generations,
            vec![1, 2, 3],
            "a generation directory is written once, so every launch has to claim a fresh one"
        );
        assert_eq!(
            harness.streams.registered_generation(&started.session_id).map(|g| g.0),
            Some(3),
            "the registration follows the newest launch"
        );
    }

    #[tokio::test]
    async fn the_token_names_the_generation_it_was_minted_for() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let renegotiated = harness
            .service
            .update(&principal(), &started.session_id, keep_everything())
            .await
            .unwrap();

        let first = token_claims(&harness, &started.manifest_url);
        let second = token_claims(&harness, &renegotiated.manifest_url);
        assert_eq!(first.session, second.session);
        assert_eq!(first.generation, StreamGeneration(1));
        assert_eq!(
            second.generation,
            StreamGeneration(2),
            "the client is handed a URL naming the artifact it just negotiated, which is what \
             stops it reading a playlist another launch wrote"
        );
    }

    #[tokio::test]
    async fn a_direct_session_is_registered_under_a_generation_too() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
        assert_eq!(
            harness.streams.registered_generation(&started.session_id).map(|g| g.0),
            Some(1),
            "direct play spawns no transcode, but its token still has to resolve to something"
        );
    }

    #[tokio::test]
    async fn ending_a_session_forgets_its_state() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        harness.service.end(&principal(), &started.session_id).await.unwrap();
        assert!(!harness.service.inner.cells.holds(&started.session_id));
    }

    fn assert_nothing_left_of(harness: &Harness, session: &SessionId) {
        assert!(!harness.service.inner.cells.holds(session));
        assert!(harness.streams.registered_generation(session).is_none());
        assert_eq!(
            harness.transcode.stopped(),
            vec![session.clone()],
            "the end stops the stream the operation before it left running"
        );
    }

    #[tokio::test]
    async fn a_session_ended_while_an_update_launches_is_not_brought_back() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1), audio(2)],
            vec![],
        ));
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.transcode.hold_next_start();
        let change = SessionUpdate { audio_track: Some(2), ..keep_everything() };
        let caller = principal();

        let (updated, ended) = tokio::join!(
            harness.service.update(&caller, &session, change),
            while_held(hold, harness.service.end(&caller, &session))
        );

        assert!(
            matches!(updated, Err(SessionError::NotFound)),
            "the end cancels the update in flight"
        );
        assert!(ended.is_ok());
        assert!(harness.sessions.get(&session).await.unwrap().is_none());
        assert_nothing_left_of(&harness, &session);
        assert_eq!(
            stored_progress(&harness).await.and_then(|row| row.audio_track),
            None,
            "a track whose launch was cancelled is not remembered"
        );
    }

    #[tokio::test]
    async fn a_session_ended_while_a_seek_launches_is_not_brought_back() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.transcode.hold_next_start();
        let caller = principal();

        let (sought, ended) = tokio::join!(
            harness.service.seek(&caller, &session, 5_000),
            while_held(hold, harness.service.end(&caller, &session))
        );

        assert!(
            matches!(sought, Err(SessionError::NotFound)),
            "the end cancels the seek in flight"
        );
        assert!(ended.is_ok());
        assert!(harness.sessions.get(&session).await.unwrap().is_none());
        assert_nothing_left_of(&harness, &session);
    }

    #[tokio::test]
    async fn an_end_cancels_a_seek_whose_launch_never_finishes() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.transcode.hold_next_start();
        let caller = principal();

        let (sought, ended) = tokio::join!(harness.service.seek(&caller, &session, 5_000), async {
            hold.reached().await;
            harness.service.end(&caller, &session).await
        });

        assert!(matches!(sought, Err(SessionError::NotFound)));
        assert!(ended.is_ok(), "the end did not wait for the launch");
        assert!(harness.sessions.get(&session).await.unwrap().is_none());
        assert_nothing_left_of(&harness, &session);
    }

    #[tokio::test]
    async fn removing_a_user_cancels_an_update_whose_launch_never_finishes() {
        let harness = Harness::new();
        harness.catalog.insert(two_track_transcode());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.transcode.hold_next_start();
        let caller = principal();
        let change = SessionUpdate { audio_track: Some(2), ..keep_everything() };

        let (updated, ended) =
            tokio::join!(harness.service.update(&caller, &session, change), async {
                hold.reached().await;
                harness.service.end_all_for_user(&caller.user).await
            });

        assert!(matches!(updated, Err(SessionError::NotFound)));
        assert!(ended.is_ok(), "the end did not wait for the launch");
        assert!(harness.sessions.get(&session).await.unwrap().is_none());
        assert_nothing_left_of(&harness, &session);
    }

    #[tokio::test]
    async fn an_end_from_another_user_leaves_the_owners_launch_running() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        let hold = harness.transcode.hold_next_start();
        let (caller, stranger) = (principal(), intruder());

        let (sought, refused) = tokio::join!(
            harness.service.seek(&caller, &session, 5_000),
            while_held(hold, harness.service.end(&stranger, &session))
        );

        assert!(sought.is_ok(), "the owner's seek was not cancelled");
        assert!(matches!(refused, Err(SessionError::NotFound)));
        assert!(harness.sessions.get(&session).await.unwrap().is_some());
        assert!(harness.service.inner.cells.holds(&session));
    }

    #[tokio::test]
    async fn an_update_writes_the_bookmark_at_the_position_it_was_sent() {
        let harness = Harness::new();
        harness.catalog.insert(multilingual_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();

        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate { position_ms: 60_000, audio_track: Some(2), ..keep_everything() },
            )
            .await
            .unwrap();

        let row = stored_progress(&harness).await.expect("an override forces a row");
        assert_eq!(row.position_ms, 60_000);
        assert_eq!(row.audio_track, Some(2));
    }

    #[tokio::test]
    async fn a_session_ended_before_its_relaunch_starts_nothing() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
        let session = started.session_id;
        harness.service.end(&principal(), &session).await.unwrap();

        let updated = harness.service.update(&principal(), &session, keep_everything()).await;
        let sought = harness.service.seek(&principal(), &session, 5_000).await;

        assert!(matches!(updated, Err(SessionError::NotFound)));
        assert!(matches!(sought, Err(SessionError::NotFound)));
        assert_eq!(harness.transcode.started().len(), 1, "only the first launch ran");
    }

    fn token_claims(harness: &Harness, manifest_url: &str) -> StreamClaims {
        let token = manifest_url
            .strip_prefix("/stream/")
            .and_then(|rest| rest.rsplit_once('/'))
            .expect("a manifest url carries its token")
            .0;
        harness.tokens.verify(token).expect("the token verifies")
    }

    #[derive(Clone, Copy, Debug)]
    enum Held {
        Seek,
        Update,
        Heartbeat,
    }

    #[derive(Clone, Copy, Debug)]
    enum Fired {
        Heartbeat,
        Seek,
        Update,
        End,
        EndAll,
        Reap,
    }

    fn two_track_transcode() -> VersionDetail {
        detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1), audio(2)],
            vec![],
        )
    }

    fn hold_for(harness: &Harness, held: Held) -> Hold {
        match held {
            Held::Seek | Held::Update => harness.transcode.hold_next_start(),
            Held::Heartbeat => harness.catalog.hold_next_version_read(),
        }
    }

    fn held_position(held: Held, beat_ms: u64) -> u64 {
        match held {
            Held::Seek => 5_000,
            Held::Update => 6_000,
            Held::Heartbeat => beat_ms,
        }
    }

    fn fired_position(fired: Fired, beat_ms: u64) -> Option<u64> {
        match fired {
            Fired::Heartbeat => Some(beat_ms + 1_000),
            Fired::Seek => Some(9_000),
            Fired::Update => Some(10_000),
            Fired::End | Fired::EndAll | Fired::Reap => None,
        }
    }

    async fn run_held(
        harness: &Harness,
        held: Held,
        session: &SessionId,
        beat_ms: u64,
    ) -> Result<(), SessionError> {
        let caller = principal();
        let at = held_position(held, beat_ms);
        match held {
            Held::Seek => harness.service.seek(&caller, session, at).await.map(drop),
            Held::Update => {
                let change =
                    SessionUpdate { position_ms: at, audio_track: Some(2), ..keep_everything() };
                harness.service.update(&caller, session, change).await.map(drop)
            }
            Held::Heartbeat => harness
                .service
                .heartbeat(&caller, session, at, PlaybackState::Paused)
                .await
                .map(drop),
        }
    }

    async fn run_fired(
        harness: &Harness,
        fired: Fired,
        session: &SessionId,
        beat_ms: u64,
    ) -> Result<(), SessionError> {
        let caller = principal();
        let at = fired_position(fired, beat_ms).unwrap_or(0);
        match fired {
            Fired::Heartbeat => harness
                .service
                .heartbeat(&caller, session, at, PlaybackState::Playing)
                .await
                .map(drop),
            Fired::Seek => harness.service.seek(&caller, session, at).await.map(drop),
            Fired::Update => {
                let change = SessionUpdate {
                    position_ms: at,
                    subtitle: SubtitleChange::Disable,
                    ..keep_everything()
                };
                harness.service.update(&caller, session, change).await.map(drop)
            }
            Fired::End => harness.service.end(&caller, session).await,
            Fired::EndAll => harness.service.end_all_for_user(&caller.user).await,
            Fired::Reap => {
                harness.service.reap_idle().await;
                Ok(())
            }
        }
    }

    async fn while_held<T>(hold: Hold, fire: impl Future<Output = T>) -> T {
        hold.reached().await;
        let (fired, ()) = tokio::join!(fire, async { hold.release() });
        fired
    }

    async fn race(
        harness: &Harness,
        held: Held,
        fired: Fired,
        session: &SessionId,
        beat_ms: u64,
    ) -> (Result<(), SessionError>, Result<(), SessionError>) {
        let hold = hold_for(harness, held);
        tokio::join!(
            run_held(harness, held, session, beat_ms),
            while_held(hold, run_fired(harness, fired, session, beat_ms))
        )
    }

    async fn look_idle(harness: &Harness, session: &SessionId) {
        let mut idle = harness.sessions.get(session).await.unwrap().unwrap();
        idle.last_heartbeat_at = Timestamp::UNIX_EPOCH;
        harness.sessions.insert(idle).await.unwrap();
    }

    #[tokio::test]
    async fn every_operation_fired_during_another_keeps_the_session_invariants() {
        let fired_all =
            [Fired::Heartbeat, Fired::Seek, Fired::Update, Fired::End, Fired::EndAll, Fired::Reap];
        for held in [Held::Seek, Held::Update, Held::Heartbeat] {
            for fired in fired_all {
                let label = format!("{fired:?} during {held:?}");
                let harness = Harness::new();
                harness.catalog.insert(two_track_transcode());
                let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
                let session = started.session_id;
                if matches!(fired, Fired::Reap) {
                    look_idle(&harness, &session).await;
                }

                let (held_result, fired_result) =
                    race(&harness, held, fired, &session, 7_000).await;

                let ended = matches!(fired, Fired::End | Fired::EndAll);
                let cancelled = ended && !matches!(held, Held::Heartbeat);
                match cancelled {
                    true => assert!(
                        matches!(held_result, Err(SessionError::NotFound)),
                        "{label}: the end cancels the held launch"
                    ),
                    false => assert!(held_result.is_ok(), "{label}: the held operation finishes"),
                }
                assert!(fired_result.is_ok(), "{label}: the fired operation finishes");
                assert_eq!(harness.sessions.started_count(), 1, "{label}: one start");
                let entry = harness.sessions.get(&session).await.unwrap();
                assert_eq!(entry.is_some(), !ended, "{label}: only an end ends the session");
                assert_eq!(
                    harness.service.inner.cells.holds(&session),
                    !ended,
                    "{label}: the cell and the registry agree"
                );
                if ended {
                    assert_nothing_left_of(&harness, &session);
                    let late = harness
                        .service
                        .heartbeat(&principal(), &session, 11_000, PlaybackState::Playing)
                        .await;
                    assert!(matches!(late, Err(SessionError::NotFound)), "{label}: ended stays");
                    assert!(harness.sessions.get(&session).await.unwrap().is_none(), "{label}");
                }
                if let Some(entry) = entry {
                    let last = fired_position(fired, 7_000).unwrap_or(held_position(held, 7_000));
                    assert_eq!(entry.position_ms, last, "{label}: the registry shows the last one");
                    let audio = match held {
                        Held::Update => Some(2),
                        Held::Seek | Held::Heartbeat => started.selected.audio_track,
                    };
                    assert_eq!(entry.selected.audio_track, audio, "{label}: the newest selection");
                }
                let chosen = (
                    (matches!(held, Held::Update) && !cancelled).then_some(2),
                    matches!(fired, Fired::Update).then_some(SubtitleOverride::Off),
                );
                match stored_progress(&harness).await {
                    Some(row) => assert_eq!(
                        (row.audio_track, row.subtitle),
                        chosen,
                        "{label}: the bookmark carries the newest choice"
                    ),
                    None => assert_eq!(chosen, (None, None), "{label}: a choice is saved"),
                }
            }
        }
    }

    #[tokio::test]
    async fn a_view_counts_once_whatever_runs_beside_the_heartbeat_that_completes_it() {
        let pairs = [
            (Held::Seek, Fired::Heartbeat),
            (Held::Update, Fired::Heartbeat),
            (Held::Heartbeat, Fired::Seek),
            (Held::Heartbeat, Fired::Update),
            (Held::Heartbeat, Fired::Heartbeat),
        ];
        for (held, fired) in pairs {
            let label = format!("{fired:?} during {held:?}");
            let harness = Harness::new();
            harness.catalog.insert(two_track_transcode());
            let started = harness.service.start(&principal(), start_request(0)).await.unwrap();
            let session = started.session_id;

            let (held_result, fired_result) = race(&harness, held, fired, &session, 95_000).await;
            harness
                .service
                .heartbeat(&principal(), &session, 99_000, PlaybackState::Playing)
                .await
                .unwrap();

            assert!(held_result.is_ok() && fired_result.is_ok(), "{label}");
            let history = harness.progress.history(&principal().user, page()).await.unwrap();
            assert_eq!(history.items[0].play_count, 1, "{label}: one session, one view");
        }
    }

    #[tokio::test]
    async fn a_track_chosen_at_the_start_is_saved_before_any_heartbeat() {
        let harness = Harness::new();
        harness.catalog.insert(two_track_transcode());
        let request = SessionStartInput { audio: AudioRequest::Track(2), ..start_request(42_000) };

        harness.service.start(&principal(), request).await.unwrap();

        let row = stored_progress(&harness).await.expect("a chosen track forces a row");
        assert_eq!(row.audio_track, Some(2));
        assert_eq!(row.position_ms, 42_000);
    }

    #[tokio::test]
    async fn an_operation_waiting_behind_an_end_finds_the_session_gone() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let session =
            harness.service.start(&principal(), start_request(0)).await.unwrap().session_id;
        let hold = harness.transcode.hold_next_stop();
        let caller = principal();

        let (ended, beat) = tokio::join!(harness.service.end(&caller, &session), async {
            hold.reached().await;
            harness.sessions.set_fail();
            let beat = harness.service.heartbeat(&caller, &session, 8_000, PlaybackState::Playing);
            let (beat, ()) = tokio::join!(beat, async { hold.release() });
            beat
        });

        assert!(ended.is_err(), "the registry is down, so the end cannot remove its entry");
        assert!(
            matches!(beat, Err(SessionError::NotFound)),
            "the session ended, whatever the registry still holds"
        );
        assert_nothing_left_of(&harness, &session);
    }

    async fn idle_since(harness: &Harness, session: &SessionId, started_s: i64) {
        let mut idle = harness.sessions.get(session).await.unwrap().unwrap();
        idle.started_at = Timestamp::from_second(started_s).unwrap();
        idle.last_heartbeat_at = Timestamp::UNIX_EPOCH;
        harness.sessions.insert(idle).await.unwrap();
    }

    #[tokio::test]
    async fn a_session_that_heartbeats_while_the_reaper_ends_another_is_kept() {
        let harness = Harness::new();
        harness.catalog.insert(two_track_transcode());
        let first = harness.service.start(&principal(), start_request(0)).await.unwrap().session_id;
        let second =
            harness.service.start(&principal(), start_request(0)).await.unwrap().session_id;
        idle_since(&harness, &first, 1).await;
        idle_since(&harness, &second, 2).await;
        let hold = harness.transcode.hold_next_stop();
        let caller = principal();

        let (_, beat) = tokio::join!(
            harness.service.reap_idle(),
            while_held(
                hold,
                harness.service.heartbeat(&caller, &second, 8_000, PlaybackState::Playing)
            )
        );

        assert!(beat.is_ok());
        assert!(harness.sessions.get(&first).await.unwrap().is_none());
        assert!(
            harness.sessions.get(&second).await.unwrap().is_some(),
            "the reaper reads the heartbeat again once it holds the session"
        );
        assert!(harness.service.inner.cells.holds(&second));
    }

    #[tokio::test]
    async fn the_reaper_passes_over_a_busy_session_without_waiting_for_it() {
        let harness = Harness::new();
        harness.catalog.insert(two_track_transcode());
        let session =
            harness.service.start(&principal(), start_request(0)).await.unwrap().session_id;
        look_idle(&harness, &session).await;
        let hold = harness.transcode.hold_next_start();
        let caller = principal();

        let (sought, passed) =
            tokio::join!(harness.service.seek(&caller, &session, 5_000), async {
                hold.reached().await;
                let passed = harness.service.reap_idle().now_or_never().is_some();
                hold.release();
                passed
            });

        assert!(sought.is_ok());
        assert!(passed, "the reaper finishes while the session is still busy");
        assert!(harness.sessions.get(&session).await.unwrap().is_some());
    }
}
