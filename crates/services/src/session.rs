use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use jiff::Timestamp;
use uuid::Uuid;

use domain::catalog::{VersionDetail, VersionId};
use domain::common::{Page, PageRequest};
use domain::error::SessionError;
use domain::negotiation::{NegotiationInput, effective_max_height, negotiate};
use domain::playback::{PlaybackProgress, SubtitleTrackRef, UserSubtitleOffset, WatchHistory};
use domain::profile::{Container, ProfileRegistry};
use domain::repository::{
    PreferencesRepository, ProgressRepository, SessionRegistry, UserRepository, VersionCatalog,
};
use domain::service::SessionService;
use domain::session::{
    ClientCapabilities, DeliveryMode, HeartbeatAck, PlaybackSession, PlaybackState, Renegotiated,
    SelectedTracks, SessionId, SessionStartInput, SessionStarted, SessionUpdate, SoftSubtitle,
    SoftSubtitleSource, StreamClaims, StreamRegistration, StreamRegistry, StreamTokens,
    SubtitleChange, SubtitleDelivery, SubtitleRendition, SubtitleSelection, TranscodeManager,
    TranscodeSpec,
};
use domain::user::{Principal, UserId};

const HEARTBEAT_INTERVAL_S: u32 = 10;
const DEFAULT_BANDWIDTH: u64 = 4_000_000;
const TOKEN_TTL_SECS: i64 = 3600;

#[derive(Clone)]
struct LaunchContext {
    user: UserId,
    version: VersionId,
    capabilities: ClientCapabilities,
    requested_audio: Option<u32>,
    requested_subtitle: Option<SubtitleSelection>,
    bitrate_cap: Option<u64>,
    target_height: Option<u32>,
    force_burn: bool,
    downmix_stereo: bool,
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
    contexts: RwLock<HashMap<SessionId, LaunchContext>>,
}

pub struct DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> {
    #[allow(clippy::type_complexity)]
    inner: Arc<Inner<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>>,
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> Clone
    for DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
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
                contexts: RwLock::new(HashMap::new()),
            }),
        }
    }
}

impl<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf> DefaultSessionService<Vc, Pr, Tm, Tk, Sr, Reg, U, Pg, Pf>
where
    Tm: TranscodeManager,
{
    pub async fn reap_idle(&self) -> usize {
        self.inner.transcode.reap_idle().await
    }
}

fn manifest_path(mode: DeliveryMode, token: &str) -> String {
    match mode {
        DeliveryMode::Direct => format!("/stream/{token}/file"),
        _ => format!("/stream/{token}/master.m3u8"),
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
    detail
        .video
        .first()
        .and_then(|v| v.bitrate)
        .unwrap_or(DEFAULT_BANDWIDTH)
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
    Some(SubtitleRendition {
        name: language.clone(),
        language,
    })
}

fn burn_path_for(selected: &SelectedTracks, detail: &VersionDetail) -> Option<String> {
    if selected.subtitle_delivery != Some(SubtitleDelivery::Burned) {
        return None;
    }
    match selected.subtitle_track.as_ref()? {
        SubtitleTrackRef::Embedded(_) => Some(detail.version.path.clone()),
        SubtitleTrackRef::File(id) => detail
            .subtitle_files
            .iter()
            .find(|f| &f.id == id)
            .map(|f| f.path.clone()),
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
    async fn soft_subtitle_for(
        &self,
        selected: &SelectedTracks,
        detail: &VersionDetail,
        ctx: &LaunchContext,
    ) -> Option<SoftSubtitle> {
        if selected.subtitle_delivery != Some(SubtitleDelivery::HlsVtt) {
            return None;
        }
        let track = selected.subtitle_track.as_ref()?;
        let source = match track {
            SubtitleTrackRef::Embedded(idx) => SoftSubtitleSource::Embedded(*idx),
            SubtitleTrackRef::File(id) => {
                let path = detail
                    .subtitle_files
                    .iter()
                    .find(|f| &f.id == id)?
                    .path
                    .clone();
                SoftSubtitleSource::File(path)
            }
        };
        Some(SoftSubtitle {
            source,
            offset_ms: self.resolve_offset(ctx, track).await,
            duration_ms: detail.version.duration_ms,
        })
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
        user: &UserId,
        version: &VersionId,
    ) -> Result<String, SessionError> {
        let expires_at = Timestamp::from_second(Timestamp::now().as_second() + TOKEN_TTL_SECS)
            .unwrap_or(Timestamp::MAX);
        let claims = StreamClaims {
            session: session.clone(),
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

    async fn launch(
        &self,
        session_id: &SessionId,
        detail: &VersionDetail,
        ctx: &LaunchContext,
        position_ms: u64,
    ) -> Result<(DeliveryMode, SelectedTracks), SessionError> {
        let container =
            Container::parse(&detail.version.container).ok_or(SessionError::NegotiationFailed)?;
        let profile = self.inner.profiles.resolve(&ctx.capabilities.platform);
        let max_bitrate = combine_caps(ctx.capabilities.max_bitrate, ctx.bitrate_cap);
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
        };
        let outcome = negotiate(&input, &profile);
        let bandwidth = bandwidth_of(detail);
        let cap = effective_max_height(profile.max_height, ctx.target_height);
        let scale_to = detail
            .video
            .first()
            .map(|v| v.height)
            .filter(|height| *height > cap)
            .map(|_| cap);

        if outcome.mode == DeliveryMode::Direct {
            let _ = self.inner.transcode.stop(session_id).await;
            self.inner.streams.register(
                session_id.clone(),
                StreamRegistration {
                    mode: DeliveryMode::Direct,
                    output_dir: PathBuf::new(),
                    direct_path: Some(PathBuf::from(&detail.version.path)),
                    bandwidth,
                    subtitle: None,
                },
            );
        } else {
            let burn_subtitle_path = burn_path_for(&outcome.selected, detail);
            let soft_subtitle = self.soft_subtitle_for(&outcome.selected, detail, ctx).await;
            let remux = outcome.mode == DeliveryMode::Remux;
            let started = self
                .inner
                .transcode
                .start(TranscodeSpec {
                    session: session_id.clone(),
                    input_path: detail.version.path.clone(),
                    duration_ms: detail.version.duration_ms,
                    copy: remux,
                    seek_ms: (position_ms > 0).then_some(position_ms),
                    audio_track: outcome.selected.audio_track,
                    max_height: if remux { None } else { scale_to },
                    max_bitrate,
                    burn_subtitle_path,
                    soft_subtitle,
                    downmix_stereo: ctx.downmix_stereo,
                    source_hdr: detail.video.first().and_then(|v| v.hdr),
                })
                .await
                .map_err(|_| SessionError::NegotiationFailed)?;
            self.inner.streams.register(
                session_id.clone(),
                StreamRegistration {
                    mode: outcome.mode,
                    output_dir: PathBuf::from(started.output_dir),
                    direct_path: None,
                    bandwidth,
                    subtitle: subtitle_rendition(&outcome.selected, detail),
                },
            );
        }
        Ok((outcome.mode, outcome.selected))
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
            .ok_or(SessionError::VersionNotFound)?;
        if !detail.version.available {
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
                .version_detail(&row.version)
                .await?
                .is_some_and(|other| other.version.title == started_title);
            if same_title {
                self.inner
                    .progress
                    .delete(&caller.user, &row.version)
                    .await?;
            }
        }

        let session_id = SessionId(Uuid::new_v4().to_string());
        let now = Timestamp::now();
        let token = self.create_token(&session_id, &caller.user, &request.version)?;
        let ctx = LaunchContext {
            user: caller.user.clone(),
            version: request.version.clone(),
            capabilities: request.capabilities.clone(),
            requested_audio: request.audio_track,
            requested_subtitle: request.subtitle.clone(),
            bitrate_cap: user.as_ref().and_then(|u| u.bitrate_cap),
            target_height: request.target_height,
            force_burn: request.force_burn,
            downmix_stereo: request.downmix_stereo,
        };
        let (mode, selected) = self
            .launch(&session_id, &detail, &ctx, request.start_position_ms)
            .await?;
        let manifest_url = manifest_path(mode, &token);

        self.inner
            .contexts
            .write()
            .unwrap()
            .insert(session_id.clone(), ctx);
        self.inner
            .sessions
            .insert(PlaybackSession {
                id: session_id.clone(),
                user: caller.user.clone(),
                device: None,
                version: request.version,
                mode,
                position_ms: request.start_position_ms,
                state: PlaybackState::Playing,
                selected: selected.clone(),
                started_at: now,
                last_heartbeat_at: now,
            })
            .await?;

        Ok(SessionStarted {
            session_id,
            mode,
            manifest_url,
            selected,
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
        let mut playback = self
            .inner
            .sessions
            .get(session)
            .await?
            .ok_or(SessionError::NotFound)?;
        let now = Timestamp::now();
        playback.position_ms = position_ms;
        playback.state = state;
        playback.last_heartbeat_at = now;
        let version = playback.version.clone();
        self.inner.sessions.insert(playback).await?;
        let _ = self.inner.transcode.touch(session).await;

        self.inner
            .progress
            .upsert(PlaybackProgress {
                user: caller.user.clone(),
                version: version.clone(),
                position_ms,
                updated_at: now,
            })
            .await?;

        if let Some(detail) = self.inner.catalog.version_detail(&version).await? {
            let duration = detail.version.duration_ms;
            if duration > 0 && (position_ms as u128) * 10 >= (duration as u128) * 9 {
                self.inner
                    .progress
                    .record_history(WatchHistory {
                        user: caller.user.clone(),
                        title: detail.version.title,
                        watched: true,
                        play_count: 1,
                        last_watched_at: Some(now),
                        completed: true,
                    })
                    .await?;
            }
        }

        Ok(HeartbeatAck {
            heartbeat_interval_s: HEARTBEAT_INTERVAL_S,
        })
    }

    async fn seek(
        &self,
        _caller: &Principal,
        session: &SessionId,
        position_ms: u64,
    ) -> Result<Renegotiated, SessionError> {
        let mut playback = self
            .inner
            .sessions
            .get(session)
            .await?
            .ok_or(SessionError::NotFound)?;
        let ctx = self
            .inner
            .contexts
            .read()
            .unwrap()
            .get(session)
            .cloned()
            .ok_or(SessionError::NotFound)?;
        let detail = self
            .inner
            .catalog
            .version_detail(&playback.version)
            .await?
            .ok_or(SessionError::VersionNotFound)?;

        let token = self.create_token(session, &playback.user, &playback.version)?;
        let (mode, selected) = self.launch(session, &detail, &ctx, position_ms).await?;
        let manifest_url = manifest_path(mode, &token);

        playback.position_ms = position_ms;
        playback.last_heartbeat_at = Timestamp::now();
        playback.mode = mode;
        playback.selected = selected.clone();
        self.inner.sessions.insert(playback).await?;

        Ok(Renegotiated {
            session_id: session.clone(),
            mode,
            manifest_url,
            selected,
        })
    }

    async fn update(
        &self,
        _caller: &Principal,
        session: &SessionId,
        update: SessionUpdate,
    ) -> Result<Renegotiated, SessionError> {
        let mut playback = self
            .inner
            .sessions
            .get(session)
            .await?
            .ok_or(SessionError::NotFound)?;
        let mut ctx = self
            .inner
            .contexts
            .read()
            .unwrap()
            .get(session)
            .cloned()
            .ok_or(SessionError::NotFound)?;

        if let Some(audio) = update.audio_track {
            ctx.requested_audio = Some(audio);
        }
        ctx.target_height = update.target_height;
        ctx.force_burn = update.force_burn;
        ctx.downmix_stereo = update.downmix_stereo;
        match update.subtitle {
            SubtitleChange::Keep => {}
            SubtitleChange::Disable => ctx.requested_subtitle = None,
            SubtitleChange::Set(selection) => {
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
                ctx.requested_subtitle = Some(selection);
            }
        }

        let detail = self
            .inner
            .catalog
            .version_detail(&playback.version)
            .await?
            .ok_or(SessionError::VersionNotFound)?;

        let token = self.create_token(session, &playback.user, &playback.version)?;
        let (mode, selected) = self
            .launch(session, &detail, &ctx, playback.position_ms)
            .await?;
        let manifest_url = manifest_path(mode, &token);

        playback.mode = mode;
        playback.selected = selected.clone();
        playback.last_heartbeat_at = Timestamp::now();
        self.inner
            .contexts
            .write()
            .unwrap()
            .insert(session.clone(), ctx);
        self.inner.sessions.insert(playback).await?;

        Ok(Renegotiated {
            session_id: session.clone(),
            mode,
            manifest_url,
            selected,
        })
    }

    async fn end(&self, _caller: &Principal, session: &SessionId) -> Result<(), SessionError> {
        if self.inner.sessions.get(session).await?.is_none() {
            return Err(SessionError::NotFound);
        }
        let _ = self.inner.transcode.stop(session).await;
        self.inner.streams.remove(session);
        self.inner.sessions.remove(session).await?;
        self.inner.contexts.write().unwrap().remove(session);
        Ok(())
    }

    async fn active_sessions(
        &self,
        _caller: &Principal,
        page: PageRequest,
    ) -> Result<Page<PlaybackSession>, SessionError> {
        Ok(self.inner.sessions.list_all(page).await?)
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
    use domain::profile::{AudioCodecCap, CapabilityProfile, VideoCodecCap};
    use domain::user::{Role, User};

    use crate::mock::{
        MockPreferencesRepo, MockProfileRegistry, MockProgressRepo, MockSessionRegistry,
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
            let catalog = MockVersionCatalog::new();
            let transcode = MockTranscodeManager::new();
            let streams = MockStreamRegistry::new();
            let sessions = MockSessionRegistry::new();
            let users = MockUserRepo::new();
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
            );
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
            }],
            audio: vec![AudioCodecCap {
                codec: "aac".to_owned(),
                max_channels: 2,
            }],
            hdr: vec![],
            max_width: 1920,
            max_height: 1080,
            max_bitrate: 10_000_000,
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
        }
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
        }
    }

    fn detail_with(
        container: &str,
        duration_ms: u64,
        video: Vec<VideoTrack>,
        audio: Vec<AudioTrack>,
        subtitles: Vec<EmbeddedSubtitleTrack>,
    ) -> VersionDetail {
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
                edition: None,
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
        }
    }

    fn direct_detail() -> VersionDetail {
        detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        )
    }

    fn transcode_detail() -> VersionDetail {
        detail_with(
            "mp4",
            100_000,
            vec![video("vp9", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        )
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
            created_at: Timestamp::UNIX_EPOCH,
            updated_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn principal() -> Principal {
        Principal {
            user: UserId("u1".to_owned()),
            role: Role::User,
        }
    }

    fn caps(max_bitrate: Option<u64>) -> ClientCapabilities {
        ClientCapabilities {
            platform: "web".to_owned(),
            profile_version: 1,
            max_bitrate,
        }
    }

    fn start_request(start_position_ms: u64) -> SessionStartInput {
        SessionStartInput {
            version: VersionId("v1".to_owned()),
            start_position_ms,
            capabilities: caps(None),
            audio_track: None,
            subtitle: None,
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
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
                subtitle_track: None,
                subtitle_delivery: None,
            },
            started_at: Timestamp::UNIX_EPOCH,
            last_heartbeat_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn page() -> PageRequest {
        PageRequest {
            offset: 0,
            limit: 10,
        }
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
            subtitle_track: track,
            subtitle_delivery: delivery,
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
        assert_eq!(
            manifest_path(DeliveryMode::Direct, "tok"),
            "/stream/tok/file"
        );
        assert_eq!(
            manifest_path(DeliveryMode::Remux, "tok"),
            "/stream/tok/master.m3u8"
        );
        assert_eq!(
            manifest_path(DeliveryMode::Transcode, "tok"),
            "/stream/tok/master.m3u8"
        );
    }

    #[test]
    fn bandwidth_uses_first_video_bitrate_or_default() {
        let with_bitrate = detail_with(
            "mp4",
            1,
            vec![video("h264", Some(6_000_000))],
            vec![],
            vec![],
        );
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
                &selected(
                    Some(SubtitleDelivery::Burned),
                    Some(SubtitleTrackRef::Embedded(0))
                ),
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
            vec![subtitle(2, Some("eng"), SubtitleFormat::Srt)],
        );
        let rendition = subtitle_rendition(
            &selected(
                Some(SubtitleDelivery::HlsVtt),
                Some(SubtitleTrackRef::Embedded(2)),
            ),
            &detail,
        )
        .unwrap();
        assert_eq!(rendition.language, "eng");
        assert_eq!(rendition.name, "eng");
    }

    #[test]
    fn subtitle_rendition_defaults_to_und() {
        let detail = detail_with(
            "mp4",
            1,
            vec![],
            vec![],
            vec![subtitle(3, None, SubtitleFormat::Srt)],
        );
        let embedded_missing = subtitle_rendition(
            &selected(
                Some(SubtitleDelivery::HlsVtt),
                Some(SubtitleTrackRef::Embedded(9)),
            ),
            &detail,
        )
        .unwrap();
        assert_eq!(embedded_missing.language, "und");

        let embedded_no_lang = subtitle_rendition(
            &selected(
                Some(SubtitleDelivery::HlsVtt),
                Some(SubtitleTrackRef::Embedded(3)),
            ),
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

    #[tokio::test]
    async fn start_direct_play() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Direct);
        assert!(started.manifest_url.ends_with("/file"));
        assert_eq!(started.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        assert!(
            harness
                .sessions
                .get(&started.session_id)
                .await
                .unwrap()
                .is_some()
        );
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.mode, DeliveryMode::Direct);
        assert!(registration.direct_path.is_some());
        assert!(harness.transcode.started().is_empty());
        let claims = harness
            .tokens
            .verify(token_of(&started.manifest_url))
            .unwrap();
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
                updated_at: Timestamp::UNIX_EPOCH,
            })
            .await
            .unwrap();

        let request = SessionStartInput {
            version: VersionId("v2".to_owned()),
            start_position_ms: 0,
            capabilities: caps(None),
            audio_track: None,
            subtitle: None,
            target_height: None,
            force_burn: false,
            downmix_stereo: false,
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert!(started.manifest_url.ends_with("/master.m3u8"));
        let specs = harness.transcode.started();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].seek_ms, None);
        assert_eq!(specs[0].max_height, None);
        assert!(specs[0].burn_subtitle_path.is_none());
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.mode, DeliveryMode::Transcode);
        assert_eq!(
            registration.output_dir,
            PathBuf::from(format!("/mock/cache/{}", started.session_id.0))
        );
        assert!(registration.direct_path.is_none());
    }

    #[tokio::test]
    async fn start_transcode_passes_seek_position() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        harness
            .service
            .start(&principal(), start_request(5_000))
            .await
            .unwrap();
        assert_eq!(harness.transcode.started()[0].seek_ms, Some(5_000));
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Remux);
        assert!(started.manifest_url.ends_with("/master.m3u8"));
        assert_eq!(harness.transcode.started().len(), 1);
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
        request.subtitle = Some(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(
            harness.transcode.started()[0].burn_subtitle_path,
            Some("/media/m1".to_owned())
        );
        assert!(
            harness
                .streams
                .registration(&started.session_id)
                .unwrap()
                .subtitle
                .is_none()
        );
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
            language: Some(LanguageCode("eng".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/m1.en.srt".into(),
            translated_from: None,
        }];
        harness.catalog.insert(detail);
        let mut request = start_request(0);
        request.force_burn = true;
        request.subtitle = Some(SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(
            harness.transcode.started()[0].burn_subtitle_path,
            Some("/media/m1.en.srt".to_owned())
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
            vec![subtitle(2, Some("fra"), SubtitleFormat::Srt)],
        ));
        let mut request = start_request(0);
        request.subtitle = Some(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.subtitle.unwrap().language, "fra");
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
            language: Some(LanguageCode("eng".into())),
            format: SubtitleFormat::Srt,
            source: SubtitleSource::External,
            path: "/media/m1.en.srt".into(),
            translated_from: None,
        }];
        harness.catalog.insert(detail);
        let mut request = start_request(0);
        request.subtitle = Some(SubtitleSelection {
            track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();

        let registration = harness.streams.registration(&started.session_id).unwrap();
        assert_eq!(registration.subtitle.unwrap().language, "eng");
        let spec = harness.transcode.started().last().cloned().unwrap();
        assert_eq!(
            spec.soft_subtitle,
            Some(SoftSubtitle {
                source: SoftSubtitleSource::File("/media/m1.en.srt".into()),
                offset_ms: 0,
                duration_ms: 100_000,
            })
        );

        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::File(SubtitleFileId("sf1".into())),
                        offset_ms: Some(2_000),
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
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
            "avi",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![],
        ));
        assert!(matches!(
            harness.service.start(&principal(), start_request(0)).await,
            Err(SessionError::NegotiationFailed)
        ));
        assert!(
            harness
                .sessions
                .list_all(page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
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
        assert!(
            harness
                .sessions
                .list_all(page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
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
        assert!(
            harness
                .sessions
                .list_all(page())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(harness.transcode.started().is_empty());
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
        harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        assert_eq!(started.mode, DeliveryMode::Transcode);
        assert_eq!(harness.transcode.started()[0].max_bitrate, Some(1_000_000));
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

    #[tokio::test]
    async fn update_forces_burn_in_of_selected_subtitle() {
        let harness = Harness::new();
        harness.catalog.insert(detail_with(
            "mp4",
            100_000,
            vec![video("h264", Some(5_000_000))],
            vec![audio(1)],
            vec![subtitle(2, Some("eng"), SubtitleFormat::Srt)],
        ));
        let mut request = start_request(0);
        request.subtitle = Some(SubtitleSelection {
            track: SubtitleTrackRef::Embedded(2),
            offset_ms: None,
        });
        let started = harness.service.start(&principal(), request).await.unwrap();
        assert_eq!(
            started.selected.subtitle_delivery,
            Some(SubtitleDelivery::HlsVtt)
        );
        let renegotiated = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: true,
                    downmix_stereo: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(renegotiated.mode, DeliveryMode::Transcode);
        assert_eq!(
            renegotiated.selected.subtitle_delivery,
            Some(SubtitleDelivery::Burned)
        );
        assert_eq!(
            harness
                .transcode
                .started()
                .last()
                .unwrap()
                .burn_subtitle_path,
            Some("/media/m1".to_owned())
        );
    }

    #[tokio::test]
    async fn update_downmix_passes_flag_to_spec() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        assert!(!harness.transcode.started().last().unwrap().downmix_stereo);
        harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: true,
                },
            )
            .await
            .unwrap();
        assert!(harness.transcode.started().last().unwrap().downmix_stereo);
    }

    #[tokio::test]
    async fn heartbeat_records_completion_at_ninety_percent() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let ack = harness
            .service
            .heartbeat(
                &principal(),
                &started.session_id,
                90_000,
                PlaybackState::Playing,
            )
            .await
            .unwrap();
        assert_eq!(ack.heartbeat_interval_s, HEARTBEAT_INTERVAL_S);
        assert_eq!(
            harness
                .progress
                .get(&principal().user, &VersionId("v1".to_owned()))
                .await
                .unwrap()
                .unwrap()
                .position_ms,
            90_000
        );
        let history = harness
            .progress
            .history(&principal().user, page())
            .await
            .unwrap();
        assert_eq!(history.total, 1);
        assert!(history.items[0].completed);
    }

    #[tokio::test]
    async fn heartbeat_no_completion_below_threshold() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .heartbeat(
                &principal(),
                &started.session_id,
                89_000,
                PlaybackState::Paused,
            )
            .await
            .unwrap();
        assert_eq!(
            harness
                .progress
                .history(&principal().user, page())
                .await
                .unwrap()
                .total,
            0
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .heartbeat(
                &principal(),
                &started.session_id,
                10_000,
                PlaybackState::Playing,
            )
            .await
            .unwrap();
        assert_eq!(
            harness
                .progress
                .history(&principal().user, page())
                .await
                .unwrap()
                .total,
            0
        );
    }

    #[tokio::test]
    async fn heartbeat_touches_active_transcode() {
        let harness = Harness::new();
        harness.catalog.insert(transcode_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .heartbeat(
                &principal(),
                &started.session_id,
                1_000,
                PlaybackState::Playing,
            )
            .await
            .unwrap();
        assert_eq!(harness.transcode.touched(), vec![started.session_id]);
    }

    #[tokio::test]
    async fn heartbeat_tolerates_missing_version() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        harness
            .service
            .heartbeat(
                &principal(),
                &started.session_id,
                95_000,
                PlaybackState::Playing,
            )
            .await
            .unwrap();
        assert_eq!(
            harness
                .progress
                .history(&principal().user, page())
                .await
                .unwrap()
                .total,
            0
        );
    }

    #[tokio::test]
    async fn heartbeat_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness
                .service
                .heartbeat(
                    &principal(),
                    &SessionId("nope".to_owned()),
                    0,
                    PlaybackState::Playing
                )
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_issues_fresh_url_and_token() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let first_url = started.manifest_url.clone();
        let renegotiated = harness
            .service
            .seek(&principal(), &started.session_id, 30_000)
            .await
            .unwrap();
        assert_ne!(renegotiated.manifest_url, first_url);
        let claims = harness
            .tokens
            .verify(token_of(&renegotiated.manifest_url))
            .unwrap();
        assert_eq!(claims.session, started.session_id);
        assert_eq!(claims.version, VersionId("v1".to_owned()));
        assert_eq!(
            harness
                .sessions
                .get(&started.session_id)
                .await
                .unwrap()
                .unwrap()
                .position_ms,
            30_000
        );
    }

    #[tokio::test]
    async fn seek_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness
                .service
                .seek(&principal(), &SessionId("nope".to_owned()), 0)
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_missing_context_is_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness.sessions.insert(bare_session("s1")).await.unwrap();
        assert!(matches!(
            harness
                .service
                .seek(&principal(), &SessionId("s1".to_owned()), 0)
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn seek_version_not_found() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        assert!(matches!(
            harness
                .service
                .seek(&principal(), &started.session_id, 10_000)
                .await,
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let renegotiated = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: Some(2),
                    subtitle: SubtitleChange::Keep,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
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
            vec![subtitle(2, Some("eng"), SubtitleFormat::Srt)],
        ));
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let set = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Set(SubtitleSelection {
                        track: SubtitleTrackRef::Embedded(2),
                        offset_ms: None,
                    }),
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(
            set.selected.subtitle_delivery,
            Some(SubtitleDelivery::HlsVtt)
        );
        let disabled = harness
            .service
            .update(
                &principal(),
                &started.session_id,
                SessionUpdate {
                    audio_track: None,
                    subtitle: SubtitleChange::Disable,
                    target_height: None,
                    force_burn: false,
                    downmix_stereo: false,
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
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
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
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness.catalog.remove(&VersionId("v1".to_owned()));
        assert!(matches!(
            harness
                .service
                .update(
                    &principal(),
                    &started.session_id,
                    SessionUpdate {
                        audio_track: None,
                        subtitle: SubtitleChange::Keep,
                        target_height: None,
                        force_burn: false,
                        downmix_stereo: false,
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
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .end(&principal(), &started.session_id)
            .await
            .unwrap();
        assert_eq!(
            harness.transcode.stopped(),
            vec![started.session_id.clone()]
        );
        assert_eq!(harness.streams.removed(), vec![started.session_id.clone()]);
        assert!(
            harness
                .sessions
                .get(&started.session_id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn end_direct_session_ignores_missing_transcode() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .end(&principal(), &started.session_id)
            .await
            .unwrap();
        assert!(harness.transcode.stopped().is_empty());
        assert_eq!(harness.streams.removed(), vec![started.session_id]);
    }

    #[tokio::test]
    async fn end_not_found() {
        let harness = Harness::new();
        assert!(matches!(
            harness
                .service
                .end(&principal(), &SessionId("nope".to_owned()))
                .await,
            Err(SessionError::NotFound)
        ));
    }

    #[tokio::test]
    async fn active_sessions_lists_all() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let listing = harness
            .service
            .active_sessions(&principal(), page())
            .await
            .unwrap();
        assert_eq!(listing.total, 2);
    }

    #[tokio::test]
    async fn clone_shares_inner_state() {
        let harness = Harness::new();
        harness.catalog.insert(direct_detail());
        let started = harness
            .service
            .start(&principal(), start_request(0))
            .await
            .unwrap();
        let cloned = harness.service.clone();
        let listing = cloned.active_sessions(&principal(), page()).await.unwrap();
        assert_eq!(listing.total, 1);
        assert_eq!(listing.items[0].id, started.session_id);
    }
}
