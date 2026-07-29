pub mod api;
pub mod backup;
pub mod bootstrap;
pub mod capabilities;
pub mod cli;
pub mod enrichment;
mod job_log_layer;
pub mod lockfile;
#[cfg(any(feature = "enrichment", test))]
mod memory;
pub mod observability;
pub mod service;

pub use api::{Built, DefaultState, DefaultStreamState, Repos, WireConfig, app, build_state};
pub use capabilities::{CapabilityInputs, server_capabilities};
pub use cli::{Cli, Command};
pub use enrichment::EnrichmentConfig;
pub use lockfile::ServerLock;
pub use observability::{init_logging, install_metrics};
pub use service::{Config, Runtime, serve};
