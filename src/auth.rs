use std::fs::File;
use std::io::{BufRead, BufReader};

use url::Url;

pub fn nexus_url() -> Result<Url, url::ParseError> {
    let nexus_url = match std::env::var("NEXUS_URL") {
        Ok(nexus_url) => nexus_url,
        Err(_) => "https://oss.sonatype.org".to_string()
    };
    log::debug!("Nexus server: {nexus_url}");
    Url::parse(&nexus_url)
}

pub fn get_credentials(nexus_url: &Url) -> anyhow::Result<(String, String)> {
    let nexus_host = nexus_url.host().unwrap()
        .to_string();
    log::debug!("...host: {nexus_host}");

    // Try env var
    if let Ok(auth) = std::env::var("NEXUS_AUTH") {
        return if let Some((user, password)) = auth.split_once(':') {
            Ok((user.to_string(), password.to_string()))
        } else {
            anyhow::bail!("Invalid auth string in NEXUS_AUTH variable")
        };
    }

    // Try ~/.netrc
    let netrc = match dirs::home_dir() {
        Some(home_dir) => home_dir.join(".netrc"),
        None => anyhow::bail!("Missing a homedir"),
    };
    if netrc.exists() {
        let file = File::open(netrc)?;
        let file = BufReader::new(&file);
        let s = file.lines()
            .filter_map(|line| match line {
                Err(_) => None,
                Ok(line) if line.trim_start().starts_with('#') => None,
                Ok(line) => Some(line)
            })
            .collect::<Vec<String>>()
            .join("\n");

        let netrc = netrc_rs::Netrc::parse(s, false).unwrap();
        for machine in &netrc.machines {
            match &machine.name {
                None => {}
                Some(name) => {
                    if nexus_host.as_str() == name {
                        let user = machine.login.clone().unwrap();
                        let password = machine.password.clone().unwrap();
                        return Ok((user, password));
                    }
                }
            }
        }
        anyhow::bail!("Hostname not found in .netrc: '{:?}'", nexus_url.host())
    }

    // Give up
    anyhow::bail!("No authentication available for {nexus_url}");
}
