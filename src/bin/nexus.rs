use std::path::PathBuf;

use clap::{Parser, Subcommand};

use nexus_client::{NexusClient, NexusRepository, StagingProfiles, StagingRepositories};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Staging { staging_command } => {
            match staging_command {
                StagingCommands::Profile { profile } => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingProfiles::get(&profile)).await?;
                    let profile = response.parsed().await?;
                    println!("{} {} mode={} target={}", profile.id, profile.name, profile.mode, profile.promotion_target_repository);
                    log::debug!("{profile:?}");
                }
                StagingCommands::Profiles => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingProfiles::list()).await?;
                    let list = response.parsed().await?;
                    for profile in list {
                        println!("{} {} mode={} target={}", profile.id, profile.name, profile.mode, profile.promotion_target_repository);
                        log::debug!("{profile:?}");
                    }
                }
                StagingCommands::Repos => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingRepositories::list()).await?;
                    let list = response.parsed().await?;
                    for repo in list {
                        println!("{} {} {} # {}", repo.repository_id, repo.profile_id, repo.created, repo.description);
                        log::debug!("{repo:?}");
                    }
                }

                StagingCommands::Repo { repository_id } => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingRepositories::get(&repository_id)).await?;
                    let repo = response.parsed().await?;
                    log::info!("{repo:?}");
                }

                StagingCommands::RepoActivity { repository_id } => {
                    let nexus = nexus_client()?;
                    let request = StagingRepositories::activity(&repository_id);
                    let response = nexus.execute(request).await?;
                    let activity = response.parsed().await?;
                    println!("{activity:?}"); //TODO
                }
                StagingCommands::RepoStart { profile_id, description } => {
                    let nexus = nexus_client()?;
                    let request = StagingProfiles::start(&profile_id, &description.unwrap_or("".to_string()));
                    let response = nexus.execute(request).await?;
                    let staged_repo_id = response.parsed().await?;
                    println!("{staged_repo_id:?}");
                }
                StagingCommands::RepoDrop { profile_id, repository_id } => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingProfiles::drop(&profile_id, &repository_id)).await?;
                    response.check().await?;
                    log::warn!("Staging repository with profile '{profile_id}' was successfully dropped: {repository_id}");
                }
                StagingCommands::RepoPromote { profile_id, repository_id } => {
                    let nexus = nexus_client()?;
                    let request = StagingProfiles::promote(&profile_id, &repository_id);
                    let response = nexus.execute(request).await?;
                    let s = response.text().await?;
                    println!("{s:?}");
                }

                StagingCommands::RepoFinish { profile_id, repository_id } => {
                    let nexus = nexus_client()?;
                    let request = StagingProfiles::promote(&profile_id, &repository_id);
                    let response = nexus.execute(request).await?;
                    let s = response.text().await?;
                    println!("{s:?}");
                }
            }
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
                ContentCommands::DirectoryListing => {
                    let _nexus = nexus_public_client()?;
                    todo!()
                }
            }
        }
        Commands::Sync { .. } => todo!(),
    }

    Ok(())
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
enum StagingCommands {
    /// Show available staging profiles
    Profiles,
    /// Show one staging profile
    Profile {
        #[arg(short,long)]
        profile: String,
    },
    /// Show all current staging repositories
    Repos,
    /// Show one staging repository
    Repo {
        repository_id: String,
    },
    /// Retrieve current activity status on a staging repository
    RepoActivity {
        repository_id: String,
    },
    /// Create a new staging repository
    RepoStart {
        // TODO: make profile_id optional, defaulting to single profile existing
        // TODO: profile_id could also come from an env var
        // TODO: allow profile id syntax: `@name` to select profile by its name
        #[arg(short,long)]
        profile_id: String,
        #[arg(short,long)]
        description: Option<String>,
    },
    /// Drop staging repository
    RepoDrop {
        #[arg(short,long)]
        profile_id: String,
        // TODO: allow repository id syntax: `@desc=string` to select repo by description (must resolve to only one)
        repository_id: String,
    },
    /// Promote (close) staging repository, exposing it to others for consuming
    RepoPromote {
        #[arg(short,long)]
        profile_id: String,
        repository_id: String,
    },
    /// Finish (release) staging repository into the target repository (typically `releases`)
    RepoFinish {
        #[arg(short,long)]
        profile_id: String,
        repository_id: String,
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
    DirectoryListing,
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync files to nexus repository
    Up,
    /// Sync files from nexus repository
    Down,
}
