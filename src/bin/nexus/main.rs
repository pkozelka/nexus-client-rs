use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use cmd_staging::StagingCommands;
use nexus_client::{http_upload, NexusClient, NexusRepository};
use nexus_client::remote_sync::http_download_tree;

use crate::cmd_list::DirPrinter;
use crate::nexus_uri::NexusRemoteUri;

mod cmd_staging;
mod nexus_uri;
mod cmd_list;

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
        Commands::Download { local_path, nexus_uri, } => {
            log::info!("downloading {local_path:?} from {nexus_uri}");
            let nexus = nexus_public_client()?;
            match (local_path.is_dir(), nexus_uri.is_dir()) {
                (_, true) => {
                    // tree download
                    http_download_tree(&nexus, &nexus_uri.repo_id, &nexus_uri.repo_path, &local_path).await?;
                }
                (local_is_dir, false) => {
                    // single file download
                    let local_path = if local_is_dir {
                        let file_name = match nexus_uri.repo_path.rfind("/") {
                            None => panic!("There must always be at least one slash: {nexus_uri}"),
                            Some(index) => &nexus_uri.repo_path[index + 1..]
                        };
                        local_path.join(file_name)
                    } else {
                        local_path
                    };
                    let url = nexus.download_file(&nexus_uri.repo_id, &local_path, &nexus_uri.repo_path).await?;
                    log::info!("File {} downloaded from {url}", local_path.display());
                }
            }
        }
        Commands::Upload { local_path, nexus_uri } => {
            log::info!("uploading {local_path:?} to {nexus_uri}");
            let nexus = nexus_client()?;
            // dir-dir checking TODO perhaps move this into upload function?
            match (local_path.is_dir(), nexus_uri.is_dir()) {
                (true, true) => {
                    // tree upload
                    http_upload(&nexus, &nexus_uri.repo_id, &nexus_uri.repo_path, &local_path).await?;
                }
                (false, remote_is_dir) => {
                    // single file upload
                    let remote_path = if remote_is_dir {
                        // file -> dir is ok, we just add the name
                        format!("{}{}", nexus_uri.repo_path, local_path.file_name().unwrap().to_string_lossy())
                    } else {
                        // file -> file is completely ok
                        nexus_uri.repo_path.clone()
                    };
                    let url = nexus.upload_file(&nexus_uri.repo_id, &local_path, &remote_path).await?;
                    log::info!("File {} uploaded to {url}", local_path.display());
                }
                (local_is_dir, remote_is_dir) => anyhow::bail!("Unsupported transfer: localdir({local_is_dir}) -> remotedir({remote_is_dir})")
            }
        }
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
        Commands::List { recurse, format, long, nexus_uri } => {
            let nexus = crate::nexus_public_client()?;
            if format == DirFormat::Json {
                let request = NexusRepository::nexus_readonly(&nexus_uri.repo_id)
                    .list(&nexus_uri.repo_path);
                let response = nexus.execute(request).await?;
                let json = response.text().await?;
                println!("{json}");
                return Ok(());
            }
            let dir_printer = DirPrinter {
                format: if long { DirFormat::Long } else { format },
            };
            cmd_list::cmd_list(nexus, &nexus_uri, dir_printer, recurse).await?;
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

/// Sonatype Nexus Unofficial Client
#[derive(Parser)]
#[command(author, version, about, long_about = None, bin_name = "nexus")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download repository - entire or a subtree
    Download {
        local_path: PathBuf,
        #[arg(value_parser = clap::value_parser ! (NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },
    /// Upload local dir to a repository
    Upload {
        local_path: PathBuf,
        #[arg(value_parser = clap::value_parser ! (NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },
    /// List a directory in a remote repository
    #[clap(name = "ls")]
    List {
        /// recurse into subdirectories
        #[arg(long, short = 'R')]
        recurse: bool,
        #[arg(long, default_value = "short")]
        format: DirFormat,
        /// shortcut for `--format=long`
        #[arg(short, long)]
        long: bool,
        #[arg(value_parser = clap::value_parser ! (NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },

    /// Remove a path on remote repo (file of directory with its contents)
    #[clap(name = "rm")]
    Remove {
        #[arg(value_parser = clap::value_parser ! (NexusRemoteUri))]
        nexus_uri: NexusRemoteUri,
    },
    /// Manage staging repositories.
    /// Only for Nexus instances with "staging plugin" configured.
    Staging {
        #[command(subcommand)]
        staging_command: StagingCommands,
    },
}
