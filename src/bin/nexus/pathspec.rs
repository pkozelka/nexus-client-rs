use std::path::{Path, PathBuf};
use std::str::FromStr;

const SEP: &str = "::";


/// Pair of paths - local and remote; inspired (but not equal to) git's refspec in push/pull commands.
/// Syntax is: `<LOCAL>::<REMOTE>`.
/// Both must be **absolute**.
/// **Local**:
/// - can be omitted when unused (for `ls`, `rm`)
/// **Remote**:
/// - can be omitted, including the `::` separator, and typically defaults to `/`
/// - however, omission is sometimes forbidden - for instance with `rm`
/// - double-slash is explicitly forbidden as it is often the result of a string-interpolation gone wrong on the caller side
#[derive(Clone)]
pub struct NexusPathSpec {
    local: Option<PathBuf>,
    remote: Option<String>,
}

impl NexusPathSpec {
    pub fn local_or_err(&self) -> anyhow::Result<&Path> {
        match &self.local {
            None => anyhow::bail!("local part is required in PathSpec"),
            Some(local) => Ok(local.as_path())
        }
    }

    /// Use when local part MUST NOT be specified
    pub fn local_assert_none(&self) -> anyhow::Result<()> {
        if self.local.is_some() {
            anyhow::bail!("local part is required to be missing in PathSpec");
        }
        Ok(())
    }

    pub fn remote_or_err(&self) -> anyhow::Result<&str> {
        match &self.remote {
            None => anyhow::bail!("remote part is required in PathSpec"),
            Some(remote) => Ok(remote)
        }
    }

    pub fn remote_or<'a>(&'a self, default_value: &'a str) -> &'a str {
        match &self.remote {
            None => default_value,
            Some(remote) => remote
        }
    }

    pub fn remote_or_default(&self) -> &str {
        self.remote_or("/")
    }
}

impl FromStr for NexusPathSpec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            anyhow::bail!("Empty PathSpec is not allowed")
        }
        if s.find("//").is_some() {
            anyhow::bail!("Double-slash is prohibited in PathSpec: {s}")
        }
        let (local,remote) = match s.find(SEP) {
            None => (s.trim(), ""), // missing separator => only local part is present
            Some(index) => (s[..index].trim(), &s[index + SEP.len()..])
        };
        let local = if local.is_empty() {
            None
        } else {
            let path = PathBuf::from_str(local)?;
            if !path.is_absolute() {
                anyhow::bail!("local part must always be absolute in PathSpec: '{local}'")
            }
            Some(path)
        };
        let remote = match remote.as_bytes() {
            [] => None,
            [b'/', ..] => Some(remote.to_string()),
            _ => anyhow::bail!("Remote part of PathSpec must be absolute (= start with slash): '{remote}'"),
        };
        Ok(Self{ local, remote })
    }
}

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
    pub fn is_directory(&self) -> bool {
        self.repo_path.ends_with('/')
    }
}

impl FromStr for NexusRemoteUri {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with(REPO_PATH_START) {
            anyhow::bail!("Nexus remote URI must start with '{REPO_PATH_START}'");
        }
        let s = &s[REPO_PATH_START.len()..];
        let (repo_id,repo_path) = match s.find('/') {
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
