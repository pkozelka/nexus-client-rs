use url::Url;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub fn get_credentials() -> anyhow::Result<(Url, String, String)> {
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
