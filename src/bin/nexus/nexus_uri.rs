use std::fmt::{Display, Formatter};
use std::str::FromStr;

const REPO_PATH_START: &str = "::/";

/// Syntax: `<REPO_ID>::<REMOTE_PATH>`
/// - REPO_ID must not contain colons
/// - REMOTE_PATH must start with slash (= be absolute)
#[derive(Clone, Debug)]
pub struct NexusRemoteUri {
    pub repo_id: String,
    pub repo_path: String,
}

impl NexusRemoteUri {
    pub fn is_dir(&self) -> bool {
        self.repo_path.ends_with('/')
    }

    pub fn repo_path_dir_or_err(&self) -> anyhow::Result<&str> {
        if !self.is_dir() {
            anyhow::bail!("Directory path expected: {self}");
        }
        Ok(&self.repo_path)
    }
}

impl FromStr for NexusRemoteUri {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(REPO_PATH_START) {
            anyhow::bail!("Nexus remote URI must start with '{REPO_PATH_START}'");
        }
        let s = &s[REPO_PATH_START.len()..];
        let (repo_id, repo_path) = match s.find('/') {
            None => anyhow::bail!("Missing separator '/' in remote path specification"),
            Some(index) => (s[..index].trim(), &s[index..])
        };
        if repo_id.is_empty() {
            anyhow::bail!("Repository ID must be specified");
        }
        if !repo_path.starts_with("/") {
            anyhow::bail!("Remote path must always be absolute and therefore start with slash: '{repo_path}'");
        }

        Ok(Self {
            repo_id: repo_id.to_string(),
            repo_path: repo_path.to_string(),
        })
    }
}

impl Display for NexusRemoteUri {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(REPO_PATH_START)?;
        f.write_str(&self.repo_id)?;
        f.write_str(&self.repo_path)
    }
}
