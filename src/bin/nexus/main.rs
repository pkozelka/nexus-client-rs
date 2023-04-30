use clap::{Parser, Subcommand, ValueEnum};

use cmd_content::ContentCommands;
use cmd_staging::StagingCommands;
use nexus_client::{nexus_sync_up, NexusClient, NexusRepository};

use crate::pathspec::NexusPathSpec;

mod cmd_staging;
mod cmd_content;
mod pathspec;

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
        Commands::Pull { repo_id, path_spec } => {
            println!("pulling from repository {repo_id}");
            println!(" to   local path {:?}", path_spec.local_or_err()?);
            println!("from remote path {:?}", path_spec.remote_or_default());
            todo!()
        },
        Commands::Push { repo_id, path_spec } => {
            let local_path = path_spec.local_or_err()?;
            let remote_path = path_spec.remote_or_default();
            log::info!("pushing to {repo_id}: {}::{remote_path}", local_path.display());
            let nexus = nexus_client()?;
            let remote_root = remote_path;
            //TODO add dir-dir checking
            nexus_sync_up(&nexus, &repo_id, remote_root, local_path).await?;
        },
        Commands::Remove { repo_id, path_spec } => {
            //TODO if not --force, require the repository to be open and non-transitioning
            //TODO support wildcards?
            path_spec.local_assert_none()?;
            let remote_path = path_spec.remote_or_err()?;
            let nexus = crate::nexus_client()?;
            let request = NexusRepository::nexus_readwrite(&repo_id)
                .delete(remote_path);
            let response = nexus.execute(request).await?;
            response.check().await?;
            log::warn!("Removed: {remote_path} from repository {repo_id}");
        }
        Commands::List { format, repo_id, path_spec } => {
            let nexus = crate::nexus_public_client()?;
            path_spec.local_assert_none()?;
            let remote_path = path_spec.remote_or_default();
            let request = NexusRepository::nexus_readonly(&repo_id)
                .list(&remote_path);
            let response = nexus.execute(request).await?;
            if format == DirFormat::Json {
                let json = response.text().await?;
                println!("{json}");
            } else {
                for entry in response.parsed().await? {
                    match format {
                        DirFormat::Short => {
                            let leaf = if entry.leaf { "" } else { "/" };
                            println!("{}{leaf}", entry.text)
                        },
                        DirFormat::Long => {
                            let size_or_dir = if entry.size_on_disk == -1 {
                                " DIRECTORY".to_string()
                            } else {
                                format!("{:10}", entry.size_on_disk)
                            };
                            println!("{}\t{size_or_dir}\t{}", entry.last_modified, entry.relative_path)
                        },
                        _ => panic!("Unknown format: {format:?}"),
                    }
                }
            }

        }
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
    Pull {
        repo_id: String,
        #[arg(value_parser = clap::value_parser!(NexusPathSpec))]
        path_spec: NexusPathSpec,
    },

    Push {
        repo_id: String,
        #[arg(value_parser = clap::value_parser!(NexusPathSpec))]
        path_spec: NexusPathSpec,
    },

    /// List a directory
    #[clap(name="ls")]
    List {
        #[arg(long,default_value="short")]
        format: DirFormat,
        repo_id: String,
        #[arg(value_parser = clap::value_parser!(NexusPathSpec))]
        path_spec: NexusPathSpec,
    },

    /// Remove a path on remote repo (file of directory with its contents)
    #[clap(name="rm")]
    Remove {
        repo_id: String,
        #[arg(value_parser = clap::value_parser!(NexusPathSpec))]
        path_spec: NexusPathSpec,
    }
}

const SEP: &str = "::";

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync files to nexus repository
    Up,
    /// Sync files from nexus repository
    Down,
}
