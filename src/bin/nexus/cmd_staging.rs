use clap::Subcommand;

use nexus_client::{StagingProfiles, StagingRepositories};
use nexus_client::model::{StagingProfile, StagingProfileRepository};

use crate::DirFormat;

pub async fn cmd_staging(staging_command: StagingCommands) -> anyhow::Result<()> {
    match staging_command {
        StagingCommands::Profile { profile } => {
            let nexus = crate::nexus_client()?;
            let response = nexus.execute(StagingProfiles::get(&profile)).await?;
            let profile = response.parsed().await?;
            let StagingProfile { resource_uri, id, name, repository_template_id, repository_type, repository_target_id, in_progress, order, deploy_uri, target_groups, finish_notify_roles, promotion_notify_roles, drop_notify_roles, close_rule_sets, promote_rule_sets, promotion_target_repository, mode, finish_notify_creator, promotion_notify_creator, drop_notify_creator, auto_staging_disabled, repositories_searchable, properties } = profile;
            println!(r##"id={id}
name={name}
mode={mode}
auto_staging_disabled={auto_staging_disabled}
close_rule_sets={close_rule_sets:?}
deploy_uri={deploy_uri}
drop_notify_creator={drop_notify_creator}
drop_notify_roles={drop_notify_roles:?}
finish_notify_creator={finish_notify_creator}
finish_notify_roles={finish_notify_roles:?}
in_progress={in_progress}
order={order}
promotion_target_repository={promotion_target_repository}
properties={properties:?}
repository_target_id={repository_target_id}
repository_template_id={repository_template_id}
repositories_searchable={repositories_searchable}
promote_rule_sets={promote_rule_sets:?}
promotion_notify_creator={promotion_notify_creator}
promotion_notify_roles={promotion_notify_roles:?}
repository_type={repository_type}
resource_uri={resource_uri}
target_groups={target_groups:?}
"##);
        }
        StagingCommands::Profiles => {
            let nexus = crate::nexus_client()?;
            let response = nexus.execute(StagingProfiles::list()).await?;
            let list = response.parsed().await?;
            for profile in list {
                println!("{} {} mode={} target={}", profile.id, profile.name, profile.mode, profile.promotion_target_repository);
                log::debug!("{profile:?}");
            }
        }
        StagingCommands::Repos { format } => {
            let nexus = crate::nexus_client()?;
            let response = nexus.execute(StagingRepositories::list()).await?;
            let list = response.parsed().await?;
            match format {
                DirFormat::Short => {
                    for StagingProfileRepository { repository_id, repository_type, transitioning, description, .. } in list {
                        let t = if transitioning { "transitioning" } else { "ready" };
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
            let nexus = crate::nexus_client()?;
            let response = nexus.execute(StagingRepositories::get(&repository_id)).await?;
            let repo = response.parsed().await?;
            log::info!("{repo:?}");
        }

        StagingCommands::RepoActivity { repository_id, format } => {
            let nexus = crate::nexus_client()?;
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
            let nexus = crate::nexus_client()?;
            let request = StagingProfiles::start(&profile_id, &description.unwrap_or("".to_string()));
            let response = nexus.execute(request).await?;
            let response = response.parsed().await?;
            let staged_repo_id = response.data.staged_repository_id.ok_or(anyhow::anyhow!("No ID returned"))?;
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
        StagingCommands::RepoDescribe { profile_id, repository_id, description } => {
            let nexus = crate::nexus_client()?;
            let description = description.as_ref().map_or("", |s|s.as_str());
            let request = StagingProfiles::describe(&profile_id, &repository_id, description);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }
        StagingCommands::RepoFinish { profile_id, repository_id, description } => {
            let nexus = crate::nexus_client()?;
            let description = description.as_ref().map_or("", |s|s.as_str());
            let request = StagingProfiles::finish(&profile_id, &repository_id, description);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }

        StagingCommands::RepoPromote { profile_id, repository_id } => {
            let nexus = crate::nexus_client()?;
            let request = StagingProfiles::promote(&profile_id, &repository_id);
            let response = nexus.execute(request).await?;
            let s = response.text().await?;
            println!("{s:?}");
        }
        StagingCommands::RepoDrop { profile_id, repository_ids } => {
            if repository_ids.is_empty() {
                anyhow::bail!("Nothing to drop!");
            }
            let nexus = crate::nexus_client()?;
            for repository_id in repository_ids {
                let response = nexus.execute(StagingProfiles::drop(&profile_id, &repository_id)).await?;
                response.check().await?;
                log::warn!("Staging repository with profile '{profile_id}' was successfully dropped: {repository_id}");
            }
        }
    }
    Ok(())
}

#[derive(Subcommand)]
pub enum StagingCommands {
    /// Show available staging profiles
    Profiles,
    /// Show one staging profile
    Profile {
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile: String,
    },
    /// Show all current staging repositories
    Repos {
        #[arg(long, default_value = "short")]
        format: DirFormat,
    },
    /// Show one staging repository
    Repo {
        repository_id: String,
    },
    /// Retrieve current activity status on a staging repository
    #[command(name = "activity")]
    RepoActivity {
        repository_id: String,
        #[arg(long, default_value = "long")]
        format: DirFormat,
    },
    /// Create a new staging repository
    #[command(name = "start")]
    RepoStart {
        // TODO: make profile_id optional, defaulting to single profile existing
        // TODO: allow profile id syntax: `@name` to select profile by its name
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile_id: String,
        #[arg(long, default_value = "short")]
        format: DirFormat,
        description: Option<String>,
    },
    /// Finish (close) staging repository, exposing it to others for consuming.
    /// Hidden until a working solution is found.
    #[clap(hide(true))]
    #[command(name = "desc")]
    RepoDescribe {
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile_id: String,
        repository_id: String,
        description: Option<String>,
    },
    /// Finish (close) staging repository, exposing it to others for consuming
    #[command(name = "finish")]
    RepoFinish {
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile_id: String,
        repository_id: String,
        description: Option<String>,
    },
    /// Promote (release) staging repository into the target repository (typically `releases`)
    #[command(name = "promote")]
    RepoPromote {
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile_id: String,
        repository_id: String,
    },
    /// Drop staging repository
    #[command(name = "drop")]
    RepoDrop {
        #[arg(short, long, env = "NEXUS_STAGING_PROFILE")]
        profile_id: String,
        // TODO: allow repository id syntax: `@desc=string` to select repo by description (must resolve to only one)
        repository_ids: Vec<String>,
    },
}
