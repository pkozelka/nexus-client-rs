
use clap::{Parser, Subcommand};

use nexus_client::{NexusClient, StagingProfiles, StagingRepositories};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
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
            let response = nexus.execute(StagingProfiles::start(&profile, &description.unwrap_or("".to_string()))).await?;
            let staged_repo_id = response.parsed().await?;
            println!("{staged_repo_id}");
        }

        Commands::StagingRepoDrop { profile, repo }  => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingProfiles::drop(&profile, &repo)).await?;
            response.parsed().await?;
            log::warn!("Dropped: {repo}");
        }
        Commands::StagingRepoPromote => {}
        Commands::StagingRepoFinish => {}
        Commands::StagingRepoGet { repo } => {
            let nexus = nexus_client()?;
            let response = nexus.execute(StagingRepositories::get(&repo)).await?;
            let repository = response.parsed().await?;
            log::info!("{repository:?}");
        }
        Commands::StagingRepoActivity => {}
    }

    Ok(())
}

fn nexus_client() -> anyhow::Result<NexusClient> {
    let (server, user, password) = nexus_client::get_credentials()?;
    Ok(NexusClient::new(server, &user, &password)?)
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
    StagingRepoPromote,
    StagingRepoFinish,
    StagingRepoGet {
        repo: String,
    },
    StagingRepoActivity,
}
