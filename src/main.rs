
use clap::{Parser, Subcommand};

use nexus_client::{NexusClient, StagingProfiles, StagingRepositories};


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
}
