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
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("catalog entity not found")]
    NotFound,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("library not found")]
    NotFound,
    #[error("scan already in progress")]
    ScanInProgress,
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user not found")]
    NotFound,
    #[error("username already taken")]
    UsernameTaken,
    #[error("access denied")]
    AccessDenied,
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
