use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use cmd_content::ContentCommands;
use cmd_staging::StagingCommands;
use nexus_client::{nexus_sync_up, NexusClient};

mod cmd_staging;
mod cmd_content;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Staging { staging_command } => {
            cmd_staging::cmd_staging(staging_command).await?;
        }
        Commands::Content { repository_id, content_command } => {
            cmd_content::cmd_content(content_command, &repository_id).await?;
        }
        Commands::Sync { repository_id, local_repo, path, sync_command } => {
            match sync_command {
                SyncCommands::Up => {
                    let nexus = nexus_client()?;
                    let remote_root = match path {
                        None => "",
                        Some(ref rr) => rr
                    };
                    nexus_sync_up(&nexus, &repository_id, remote_root, &local_repo).await?;
                }
                SyncCommands::Down => todo!()
            }
        },
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DirFormat {
    Short,
    Long,
    Json,
    Xml,
}

fn nexus_client() -> anyhow::Result<NexusClient> {
    let nexus_url = nexus_client::nexus_url()?;
    let (user, password) = nexus_client::get_credentials(&nexus_url)?;
    Ok(NexusClient::login(nexus_url, &user, &password)?)
}

fn nexus_public_client() -> anyhow::Result<NexusClient> {
    Ok(NexusClient::anonymous(nexus_client::nexus_url()?)?)
}

/// Simple program to greet a person
#[derive(Parser)]
#[command(author, version, about, long_about = None, bin_name="nexus")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// https://oss.sonatype.org/nexus-staging-plugin/default/docs/index.html
#[derive(Subcommand)]
enum Commands {
    /// Manage staging repositories.
    /// Only for Nexus instances with "staging plugin" configured.
    Staging {
        #[command(subcommand)]
        staging_command: StagingCommands,
    },
    /// Manage repository content
    Content {
        #[arg(short,long="repo")]
        repository_id: String,
        #[command(subcommand)]
        content_command: ContentCommands,
    },
    /// Bulk content synchronization
    Sync {
        #[arg(short,long="repo")]
        repository_id: String,
        #[arg(short,long)]
        local_repo: PathBuf,
        #[arg(short='p',long,default_value="/")]
        path: Option<String>,
        #[command(subcommand)]
        sync_command: SyncCommands,
    },
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync files to nexus repository
    Up,
    /// Sync files from nexus repository
    Down,
}
