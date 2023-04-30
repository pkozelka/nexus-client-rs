use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use cmd_content::ContentCommands;
use cmd_staging::StagingCommands;
use nexus_client::{http_upload, NexusClient, NexusRepository};

use crate::nexus_uri::NexusRemoteUri;

mod cmd_staging;
mod cmd_content;
mod nexus_uri;

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
        Commands::Download { local_path, nexus_uri, } => {
            log::info!("downloading {local_path:?} from {nexus_uri}");
            todo!()
        },
        Commands::Upload { local_path, nexus_uri} => {
            log::info!("uploading {local_path:?} to {nexus_uri}");
            let nexus = nexus_client()?;
            // dir-dir checking TODO perhaps move this into upload function?
            match (local_path.is_dir(), nexus_uri.is_dir()) {
                (true, true) => {
                    http_upload(&nexus, &nexus_uri.repo_id, &nexus_uri.repo_path, &local_path).await?;
                }
                //TODO file -> dir will be ok, we just add the name
                //TODO file -> file is completely ok
                (local_is_dir, remote_is_dir) => anyhow::bail!("Unsupported transfer: localdir({local_is_dir} -> remotedir({remote_is_dir}))")
            }
        },
        Commands::Remove { nexus_uri } => {
            //TODO if not --force, require the repository to be open and non-transitioning
            //TODO support wildcards?
            //TODO if remote specified is not dir, make sure it really is not dir on nexus, otherwise fail (caller must prove to be aware that he deletes a directory!)
            let nexus = crate::nexus_client()?;
            let request = NexusRepository::nexus_readwrite(&nexus_uri.repo_id)
                .delete(&nexus_uri.repo_path);
            let response = nexus.execute(request).await?;
            response.check().await?;
            log::warn!("Removed: {nexus_uri}");
        }
        Commands::List { format, nexus_uri } => {
            let nexus = crate::nexus_public_client()?;
            let remote_dir = nexus_uri.repo_path_dir_or_err()?;
            let request = NexusRepository::nexus_readonly(&nexus_uri.repo_id)
                .list(remote_dir);
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
                                "/".to_string()
                            } else {
                                format!("{}", entry.size_on_disk)
                            };
                            println!("{}\t{size_or_dir:>10}\t{}", entry.last_modified, entry.relative_path)
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
    Download {
        local_path: PathBuf,
        #[arg(value_parser = clap::value_parser!(NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },

    Upload {
        local_path: PathBuf,
        #[arg(value_parser = clap::value_parser!(NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },

    /// List a directory
    #[clap(name="ls")]
    List {
        #[arg(long,default_value="short")]
        format: DirFormat,
        #[arg(value_parser = clap::value_parser!(NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },

    /// Remove a path on remote repo (file of directory with its contents)
    #[clap(name="rm")]
    Remove {
        #[arg(value_parser = clap::value_parser!(NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync files to nexus repository
    Up,
    /// Sync files from nexus repository
    Down,
}
