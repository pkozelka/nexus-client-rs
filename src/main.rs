use std::fs::File;
use std::io::{BufRead, BufReader};

use clap::{Parser, Subcommand};
use reqwest::header::{ACCEPT, HeaderMap};
use serde_json::Value;
use url::Url;

mod model;

fn get_credentials() -> anyhow::Result<(Url, String, String)> {
    let nexus_url = match std::env::var("NEXUS_URL") {
        Ok(nexus_url) => nexus_url,
        Err(_) => "https://oss.sonatype.org".to_string()
    };
    log::info!("Nexus server: {nexus_url}");
    let nexus_url = Url::parse(&nexus_url)?;
    let nexus_host = nexus_url.host().unwrap()
        .to_string();
    log::debug!("...host: {nexus_host}");
    if let Ok(auth) = std::env::var("NEXUS_CLIENT_AUTH") {
        return if let Some((user, password)) = auth.split_once(':') {
            Ok((nexus_url, user.to_string(), password.to_string()))
        } else {
            anyhow::bail!("Invalid auth string in NEXUS_CLIENT_AUTH variable")
        }
    }
    let file = File::open("/home/pk/.netrc")?;
    let file = BufReader::new(&file);
    let s = file.lines()
        .filter_map(|line| match line {
            Err(_) => None,
            Ok(line) if line.trim_start().starts_with('#') => None,
            Ok(line) => Some(line)
        })
        .collect::<Vec<String>>()
        .join("");

    let netrc = netrc_rs::Netrc::parse(s, false).unwrap();
    for machine in &netrc.machines {
        match &machine.name {
            None => {}
            Some(name) => {
                if nexus_host.as_str() == name {
                    let user = machine.login.clone().unwrap();
                    let password = machine.password.clone().unwrap();
                    return Ok((nexus_url, user, password));
                }
            }
        }
    }
    anyhow::bail!("Hostname not found in .netrc: '{:?}'", nexus_url.host())
}

async fn staging_repos_list() -> anyhow::Result<()> {
// curl -u $NEXUS_AUTH https://oss.sonatype.org/service/local/staging/profile_repositories > tests/data/profile_repositories
    let (nexus_url, user, password) = get_credentials()?;
    log::info!("NEXUS URL: {nexus_url}");
    log::info!("USER:      {user}");
    log::trace!("PASSWORD:  {password}");


    // let r=  reqwest::RequestBuilder:: basic_auth(user, Some(password))
    //     .build();
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, "application/json".parse()?);
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let r = client.get("https://oss.sonatype.org/service/local/staging/profile_repositories")
        .basic_auth(user, Some(password))
        .build()?;
    let response = client.execute(r).await?;
    let json = response.json::<Value>().await?;
    let repos: model::StagingProfileRepositories = serde_json::from_value(json)?;
    for repo in repos.data {
        println!("{} profile: '{}' id: '{}' # {}", repo.created, repo.profile_id, repo.repository_id, repo.description);
        log::debug!("{repo:?}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Commands::StagingRepoList => {
            staging_repos_list().await?;
        }
        _ => {
            todo!()
        }
    }

    Ok(())
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
    StagingProfileList,
    StagingRepoList,
    StagingRepoStart,
    StagingRepoDrop,
    StagingRepoPromote,
    StagingRepoFinish,
    StagingRepoGet,
    StagingRepoActivity,
}
