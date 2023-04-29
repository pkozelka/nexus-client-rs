use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use cmd_staging::StagingCommands;
use nexus_client::{nexus_sync_up, NexusClient, NexusRepository};

mod cmd_staging;

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
        Commands::Content { repository_id, remote_path, content_command: deploy_command } => {
            let remote_path = match remote_path {
                None => "/",
                Some(ref remote_path) => remote_path
            };
            match deploy_command {
                ContentCommands::Upload { local_path } => {
                    let nexus = nexus_client()?;
                    let url = nexus.upload_file(&repository_id, &local_path, remote_path).await?;
                    log::info!("File {} uploaded to {url}", local_path.display());
                }
                ContentCommands::Download { local_path } => {
                    let nexus = nexus_public_client()?;
                    let url = nexus.download_file(&repository_id, &local_path, remote_path).await?;
                    log::info!("File {} downloaded from {url}", local_path.display());
                }
                ContentCommands::Delete => {
                    let nexus = nexus_client()?;
                    let request = NexusRepository::nexus_readwrite(&repository_id)
                        .delete(remote_path);
                    let response = nexus.execute(request).await?;
                    response.check().await?;
                    println!("Deleted: {remote_path} from repository {repository_id}");
                }
                ContentCommands::DirectoryListing { format } => {
                    let nexus = nexus_public_client()?;
                    let request = NexusRepository::nexus_readonly(&repository_id)
                        .list(remote_path);
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
                                DirFormat::RelPath => println!("{}", entry.relative_path),
                                DirFormat::Uri => println!("{}", entry.resource_uri),
                                DirFormat::Long => println!("{:10} {} {}\t # {entry:?}", entry.size_on_disk, entry.last_modified, entry.relative_path),
                                _ => panic!("Unknown format: {format:?}"),
                            }
                        }
                    }
                }
            }
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
    RelPath,
    Uri,
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
        #[arg(short='p',long)]
        remote_path: Option<String>,
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
enum ContentCommands {
    //TODO: consider more convenient syntax for local/remote paths, allowing only one side to contain the file name
    //TODO: support multiple files on upload, into single directory target
    /// Upload single file
    #[clap(name="put")]
    Upload {
        #[arg(short,long)]
        local_path: PathBuf,
    },
    /// Download single file
    #[clap(name="get")]
    Download {
        #[arg(short,long)]
        local_path: PathBuf,
    },
    /// Delete a path (file of directory with its contents)
    #[clap(name="rm")]
    Delete,
    /// List a directory
    #[clap(name="ls")]
    DirectoryListing {
        #[arg(long,default_value="long")]
        format: DirFormat,
    },
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync files to nexus repository
    Up,
    /// Sync files from nexus repository
    Down,
}
