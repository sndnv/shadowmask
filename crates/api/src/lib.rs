pub mod dto;
pub mod error;
pub mod extract;
pub mod handlers;
pub mod middleware;
pub mod pagination;
pub mod router;
pub mod state;

pub use dto::server::{Capability, FontFile, FontSet};
pub use router::{
    basic_ui_router, bounded, cors_layer, diagnostics_router, download_router, fonts_router,
    image_router, job_log_router, maintenance_router, play_on_router, playlist_router,
    report_router, router, server_log_router, stream_router, subtitle_router,
    subtitle_search_router, trickplay_router, webhook_router,
};
pub use state::{
    AppState, DiagnosticsState, DownloadState, ImageState, JobLogState, MaintenanceState,
    PlayOnState, PlaylistState, ReportState, ServerCapabilities, ServerFonts, ServerLogState,
    StreamState, SubtitleSearchState, SubtitleState, TrickplayState, WebhookClient, WebhookState,
};
