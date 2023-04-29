use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use nexus_client::{nexus_sync_up, NexusClient, NexusRepository, StagingProfiles, StagingRepositories};
use nexus_client::model::StagingProfileRepository;

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
                StagingCommands::Repos { format } => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingRepositories::list()).await?;
                    let list = response.parsed().await?;
                    match format {
                        DirFormat::Short => {
                            for StagingProfileRepository {repository_id, repository_type, transitioning, description, .. } in list {
                                let t= if transitioning {"transitioning" } else {"ready"};
                                println!("{repository_id}\t{repository_type}\t{t}\t{description}");
                            }
                        }
                        DirFormat::Long => {
                            for repo in list {
                                // TODO not sure if this is a good choice for the "long" format
                                println!("{repo:?}");
                            }
                        }
                        _ => todo!()
                    }
                }

                StagingCommands::Repo { repository_id } => {
                    let nexus = nexus_client()?;
                    let response = nexus.execute(StagingRepositories::get(&repository_id)).await?;
                    let repo = response.parsed().await?;
                    log::info!("{repo:?}");
                }

                StagingCommands::RepoActivity { repository_id, format } => {
                    let nexus = nexus_client()?;
                    if format == DirFormat::Json {
                        let request = StagingRepositories::activity(&repository_id);
                        let response = nexus.execute(request).await?;
                        let text = response.text().await?;
                        println!("{text}");
                    } else if format == DirFormat::Xml {
                        let request = StagingRepositories::activity_xml(&repository_id);
                        let response = nexus.execute(request).await?;
                        let text = response.text().await?;
                        println!("{text}");
                    } else {
                        let request = StagingRepositories::activity(&repository_id);
                        let response = nexus.execute(request).await?;
                        let activities = response.parsed().await?;
                        match format {
                            DirFormat::Short => {
                                for activity in activities {
                                    let last_event_name = match activity.events.last() {
                                        None => "?",
                                        Some(event) => &event.name
                                    };
                                    println!("{}: {last_event_name}", activity.name);
                                }
                            }
                            DirFormat::Long => {
                                println!("Activities for staging repository '{repository_id}'");
                                for activity in activities {
                                    println!("\nactivity '{}' for staging repository - started {}, stopped {}", activity.name, activity.started, activity.stopped);
                                    for event in &activity.events {
                                        println!("* {} [{}] {}", event.timestamp, event.severity, event.name);
                                        for prop in &event.properties {
                                            println!("    {}:{}", prop.name, prop.value);
                                        }
                                    }
                                }
                            }
                            _ => todo!()
                        }
                    }
                }
                StagingCommands::RepoStart { profile_id, format, description } => {
                    let nexus = nexus_client()?;
                    let request = StagingProfiles::start(&profile_id, &description.unwrap_or("".to_string()));
                    let response = nexus.execute(request).await?;
                    let response = response.parsed().await?;
                    let staged_repo_id = response.data.staged_repository_id.ok_or( anyhow::anyhow!("No ID returned"))?;
                    match format {
                        DirFormat::Short => {
                            println!("{staged_repo_id}");
                        }
                        DirFormat::Long => {
                            println!("{staged_repo_id}\t{}", response.data.description.unwrap_or("".to_string()));
                        }
                        _ => panic!("Unsupported format: {format:?}"),
                    }
                }
                StagingCommands::RepoDrop { profile_id, repository_ids } => {
                    if repository_ids.is_empty() {
                        anyhow::bail!("Nothing to drop!");
                    }
                    let nexus = nexus_client()?;
                    for repository_id in repository_ids {
                        let response = nexus.execute(StagingProfiles::drop(&profile_id, &repository_id)).await?;
                        response.check().await?;
                        log::warn!("Staging repository with profile '{profile_id}' was successfully dropped: {repository_id}");
                    }
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
                    let request = StagingProfiles::finish(&profile_id, &repository_id);
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
enum DirFormat {
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
enum StagingCommands {
    /// Show available staging profiles
    Profiles,
    /// Show one staging profile
    Profile {
        #[arg(short,long,env="NEXUS_STAGING_PROFILE")]
        profile: String,
    },
    /// Show all current staging repositories
    Repos {
        #[arg(long,default_value="short")]
        format: DirFormat,
    },
    /// Show one staging repository
    Repo {
        repository_id: String,
    },
    /// Retrieve current activity status on a staging repository
    #[command(name="activity")]
    RepoActivity {
        repository_id: String,
        #[arg(long,default_value="long")]
        format: DirFormat,
    },
    /// Create a new staging repository
    #[command(name="start")]
    RepoStart {
        // TODO: make profile_id optional, defaulting to single profile existing
        // TODO: profile_id could also come from an env var
        // TODO: allow profile id syntax: `@name` to select profile by its name
        #[arg(short,long,env="NEXUS_STAGING_PROFILE")]
        profile_id: String,
        #[arg(long,default_value="short")]
        format: DirFormat,
        description: Option<String>,
    },
    /// Drop staging repository
    #[command(name="drop")]
    RepoDrop {
        #[arg(short,long,env="NEXUS_STAGING_PROFILE")]
        profile_id: String,
        // TODO: allow repository id syntax: `@desc=string` to select repo by description (must resolve to only one)
        repository_ids: Vec<String>,
    },
    /// Promote (close) staging repository, exposing it to others for consuming
    #[command(name="promote")]
    RepoPromote {
        #[arg(short,long,env="NEXUS_STAGING_PROFILE")]
        profile_id: String,
        repository_id: String,
    },
    /// Finish (release) staging repository into the target repository (typically `releases`)
    #[command(name="finish")]
    RepoFinish {
        #[arg(short,long,env="NEXUS_STAGING_PROFILE")]
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
