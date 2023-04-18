use std::path::PathBuf;
use clap::{Parser, Subcommand};

use nexus_client::NexusRepository;
use nexus_client::NexusClient;
use nexus_client::{StagingProfiles, StagingRepositories};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::StagingProfile { profile } => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingProfiles::get(&profile)).await?;
            let profile = response.parsed().await?;
            println!("profile id: '{}' name: '{}' mode: '{}' target repo: {}", profile.id, profile.name, profile.mode, profile.promotion_target_repository);
            log::debug!("{profile:?}");
        }
        Commands::StagingProfiles => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingProfiles::list()).await?;
            let list = response.parsed().await?;
            for profile in list {
                println!("profile id: '{}' name: '{}' mode: '{}' target repo: {}", profile.id, profile.name, profile.mode, profile.promotion_target_repository);
                log::debug!("{profile:?}");
            }
        }
        Commands::StagingRepos => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingRepositories::list()).await?;
            let list = response.parsed().await?;
            for repo in list {
                println!("{} profile: '{}' id: '{}' # {}", repo.created, repo.profile_id, repo.repository_id, repo.description);
                log::debug!("{repo:?}");
            }
        }

        Commands::StagingRepoStart { profile, description } => {
            let nexus = nexus_client()?;
            let request = StagingProfiles::start(&profile, &description.unwrap_or("".to_string()));
            let response = nexus.execute(request).await?;
            let staged_repo_id = response.parsed().await?;
            println!("{staged_repo_id:?}");
        }

        Commands::StagingRepoGet { repo } => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingRepositories::get(&repo)).await?;
            let repository = response.parsed().await?;
            log::info!("{repository:?}");
        }
        Commands::StagingRepoDrop { profile, repo }  => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingProfiles::drop(&profile, &repo)).await?;
            response.check().await?;
            log::warn!("Staging repository with profile '{profile}' was successfully dropped: {repo}");
        }
        Commands::StagingRepoPromote { profile, repo } => {
            let nexus = nexus_client()?;
            let request = StagingProfiles::promote(&profile, &repo);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }
        Commands::StagingRepoFinish { profile, repo } => {
            let nexus = nexus_client()?;
            let request = StagingProfiles::promote(&profile, &repo);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }

        Commands::StagingRepoActivity { repo } => {
            let nexus = nexus_client()?;
            let request = StagingRepositories::activity(&repo);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }
        Commands::Deploy { repo, remote_path, deploy_command } => {
            let remote_path = match remote_path {
                None => "/",
                Some(ref remote_path) => remote_path
            };
            match deploy_command {
                DeployCommands::Upload { local_path } => {
                    let nexus = nexus_client()?;
                    let url = nexus.upload_file(&repo, &local_path, remote_path).await?;
                    log::info!("File {} uploaded to {url}", local_path.display());
                }
                DeployCommands::Download { local_path } => {
                    let nexus = nexus_public_client()?;
                    let url = nexus.download_file(&repo, &local_path, remote_path).await?;
                    log::info!("File {} downloaded from {url}", local_path.display());
                }
                DeployCommands::Delete => {
                    let nexus = nexus_client()?;
                    let request = NexusRepository::nexus_deploy(&repo)
                        .delete(remote_path);
                    let response = nexus.execute(request).await?;
                    response.check().await?;
                    println!("Deleted: {remote_path} from repository {repo}");
                }
                DeployCommands::List => {
                    let _nexus = nexus_public_client()?;
                    todo!()
                }
            }
        }
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
    StagingProfiles,
    StagingProfile {
        #[arg(short,long)]
        profile: String,
    },
    StagingRepos,
    StagingRepoStart {
        #[arg(short,long)]
        profile: String,
        #[arg(short,long)]
        description: Option<String>,
    },
    StagingRepoDrop {
        #[arg(short,long)]
        profile: String,
        repo: String,
    },
    StagingRepoPromote {
        #[arg(short,long)]
        profile: String,
        repo: String,
    },
    StagingRepoFinish {
        #[arg(short,long)]
        profile: String,
        repo: String,
    },
    StagingRepoGet {
        repo: String,
    },
    StagingRepoActivity {
        repo: String,
    },

    Deploy {
        #[arg(short,long)]
        repo: String,
        #[arg(short='p',long)]
        remote_path: Option<String>,
        #[command(subcommand)]
        deploy_command: DeployCommands,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    Upload {
        #[arg(short,long)]
        local_path: PathBuf,
    },
    Download {
        #[arg(short,long)]
        local_path: PathBuf,
    },
    Delete,
    List,
}
