use thiserror::Error;

use crate::session::PlaybackSession;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("entity not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("repository backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Error)]
pub enum JobLogError {
    #[error("invalid job id")]
    InvalidId,
    #[error("job log backend error: {0}")]
    Backend(String),
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("access token expired")]
    TokenExpired,
    #[error("invalid token")]
    InvalidToken,
    #[error("unknown or expired link code")]
    UnknownLinkCode,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found")]
    NotFound,
    #[error("version not found")]
    VersionNotFound,
    #[error("concurrent stream limit reached")]
    ConcurrentLimit { active: Vec<PlaybackSession> },
    #[error("stream negotiation failed")]
    NegotiationFailed,
    #[error("access denied")]
    Forbidden,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("catalog entity not found")]
    NotFound,
    #[error("access denied")]
    Forbidden,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("library not found")]
    NotFound,
    #[error("scan already in progress")]
    ScanInProgress,
    #[error("access denied")]
    Forbidden,
    #[error("feature disabled")]
    Disabled,
    #[error("job cannot be cancelled")]
    NotCancellable,
    #[error(transparent)]
    Walk(#[from] WalkError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum WalkError {
    #[error("scan root not found: {0}")]
    RootNotFound(String),
    #[error("scan root unreadable: {0}")]
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WatchPlanError {
    #[error("scheduled library has no scan schedule")]
    MissingSchedule,
    #[error("library has no roots to watch")]
    NoRoots,
}

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user not found")]
    NotFound,
    #[error("username already taken")]
    UsernameTaken,
    #[error("access denied")]
    AccessDenied,
    #[error("current password is incorrect")]
    InvalidPassword,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("ffprobe failed: {0}")]
    Backend(String),
    #[error("could not parse ffprobe output: {0}")]
    Parse(String),
}

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("metadata provider backend error: {0}")]
    Backend(String),
    #[error("could not parse metadata response: {0}")]
    Parse(String),
    #[error("metadata not found")]
    NotFound,
}

#[derive(Debug, Error)]
pub enum ArtworkError {
    #[error("failed to download artwork: {0}")]
    Download(String),
    #[error("failed to decode artwork: {0}")]
    Decode(String),
    #[error("failed to store artwork: {0}")]
    Store(String),
}

#[derive(Debug, Error)]
pub enum SubtitleError {
    #[error("subtitle provider backend error: {0}")]
    Backend(String),
    #[error("could not parse subtitle response: {0}")]
    Parse(String),
    #[error("subtitle not found")]
    NotFound,
    #[error("could not store subtitle: {0}")]
    Store(String),
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("could not parse profile data: {0}")]
    Parse(String),
    #[error("invalid profile: {0}")]
    Invalid(String),
}

#[derive(Debug, Error)]
pub enum TranscodeError {
    #[error("failed to start transcode: {0}")]
    Spawn(String),
    #[error("transcode session not found")]
    NotFound,
}

#[derive(Debug, Error)]
pub enum TrickplayError {
    #[error("trickplay generation failed: {0}")]
    Backend(String),
}

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("failed to start fetch: {0}")]
    Spawn(String),
    #[error("failed to download media: {0}")]
    Download(String),
    #[error("fetch produced no output: {0}")]
    NoOutput(String),
    #[error("fetch io error: {0}")]
    Io(String),
}

#[derive(Debug, Error)]
pub enum TranscriptionError {
    #[error("transcription backend error: {0}")]
    Backend(String),
    #[error("unsupported transcription request: {0}")]
    Unsupported(String),
}

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("translation backend error: {0}")]
    Backend(String),
    #[error("unsupported translation request: {0}")]
    Unsupported(String),
}

#[derive(Debug, Error)]
pub enum UpscaleError {
    #[error("upscale backend error: {0}")]
    Backend(String),
    #[error("unsupported upscale request: {0}")]
    Unsupported(String),
    #[error("upscale precondition failed: {0}")]
    Precondition(String),
}

#[derive(Debug, Error)]
pub enum StreamTokenError {
    #[error("stream token expired")]
    Expired,
    #[error("invalid stream token")]
    Invalid,
    #[error("failed to create stream token: {0}")]
    Create(String),
}

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("stream session is not active")]
    NotLive,
    #[error("invalid stream request")]
    Invalid,
}
