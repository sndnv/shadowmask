use clap::Parser;
use server::cli::{Cli, Command};
use server::{Config, init_logging, install_metrics, serve};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match Cli::parse().resolved() {
        Command::Service => {
            let config = Config::load()?;
            init_logging(
                &config.log_level,
                &config.sqlx_log_level,
                &config.job_log_dir,
            );
            let metrics = install_metrics();
            serve(config, metrics, async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
        }
        Command::Backup(args) => {
            let config = Config::load()?;
            let created_at = jiff::Timestamp::now().as_millisecond();
            server::backup::snapshot(&config.db_root, &args.out, created_at).await?;
            println!("wrote snapshot to {}", args.out.display());
            Ok(())
        }
        Command::Recover(args) => {
            let config = Config::load()?;
            let manifest = server::backup::restore(&args.from, &config.db_root)?;
            println!(
                "recovered {} databases into {}",
                manifest.databases.len(),
                config.db_root.display()
            );
            Ok(())
        }
    }
}
