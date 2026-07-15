use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "shadowmask",
    bin_name = "shadowmask",
    version,
    about = "Shadowmask media server",
    after_help = "Run 'shadowmask backup --help' or 'shadowmask recover --help' for backup and recovery details."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn resolved(self) -> Command {
        self.command.unwrap_or(Command::Service)
    }
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum Command {
    #[command(about = "Run the media server (default)")]
    Service,
    #[command(
        about = "Back up all databases into a snapshot archive",
        long_about = "Back up all databases into a single snapshot archive.\n\nEvery SQLite database under the data root (the shared server database and each per-user database) is copied with SQLite VACUUM INTO and written to a tar archive alongside a manifest. VACUUM INTO takes a consistent copy without pausing writes, so this is a hot backup that is safe to run while the server is serving.\n\nOnly databases are archived. Configuration (shadowmask.toml and any secrets) is not included and must be backed up separately. Restore a snapshot with 'shadowmask recover'.",
        after_help = "Examples:\n  shadowmask backup ./shadowmask-backup.tar\n  docker compose exec shadowmask shadowmask backup /data/snapshot.tar"
    )]
    Backup(BackupArgs),
    #[command(
        about = "Restore databases from a snapshot archive",
        long_about = "Restore all databases from a snapshot archive created by 'shadowmask backup'.\n\nThe server must be stopped first. Recovery takes an exclusive lock on the data root and refuses to run against a live instance, so an in-use database is never corrupted. Each database from the archive replaces the current one atomically, and stale WAL and SHM files are removed.\n\nConfiguration is not part of the archive and is left untouched. Start the server again after a successful recovery.",
        after_help = "Examples:\n  shadowmask recover --from ./shadowmask-backup.tar\n  docker compose run --rm shadowmask recover --from /data/snapshot.tar"
    )]
    Recover(RecoverArgs),
}

#[derive(Debug, PartialEq, Eq, Args)]
pub struct BackupArgs {
    #[arg(
        value_name = "ARCHIVE",
        help = "Path of the snapshot archive to create"
    )]
    pub out: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Args)]
pub struct RecoverArgs {
    #[arg(
        long,
        value_name = "ARCHIVE",
        help = "Path of the snapshot archive to restore from"
    )]
    pub from: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(args: &[&str]) -> Command {
        Cli::try_parse_from(args).unwrap().resolved()
    }

    #[test]
    fn no_subcommand_defaults_to_service() {
        assert_eq!(resolve(&["shadowmask"]), Command::Service);
    }

    #[test]
    fn explicit_service_subcommand() {
        assert_eq!(resolve(&["shadowmask", "service"]), Command::Service);
    }

    #[test]
    fn backup_takes_positional_out() {
        assert_eq!(
            resolve(&["shadowmask", "backup", "snap.tar"]),
            Command::Backup(BackupArgs {
                out: PathBuf::from("snap.tar"),
            })
        );
    }

    #[test]
    fn recover_reads_from() {
        assert_eq!(
            resolve(&["shadowmask", "recover", "--from", "snap.tar"]),
            Command::Recover(RecoverArgs {
                from: PathBuf::from("snap.tar"),
            })
        );
    }

    #[test]
    fn recover_requires_from() {
        assert!(Cli::try_parse_from(["shadowmask", "recover"]).is_err());
    }

    #[test]
    fn command_definition_is_valid() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn backup_help_documents_the_process() {
        use clap::CommandFactory;
        let mut cli = Cli::command();
        let help = cli
            .find_subcommand_mut("backup")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(help.contains("VACUUM INTO"));
        assert!(help.contains("hot backup"));
        assert!(help.contains("docker compose exec"));
    }

    #[test]
    fn recover_help_documents_the_process() {
        use clap::CommandFactory;
        let mut cli = Cli::command();
        let help = cli
            .find_subcommand_mut("recover")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(help.contains("server must be stopped"));
        assert!(help.contains("exclusive lock"));
        assert!(help.contains("--from"));
    }
}
