use std::path::{Path, PathBuf};

use reqwest::Method;
use tokio::spawn;
use tokio::task::JoinHandle;

use crate::{NexusClient, NexusRepository, RawRequest};
use crate::model::{DirEntry, NexusResponseData};

/// Full blind upload of a directory
pub async fn http_upload(nexus: &NexusClient, repository_id: &str, remote_root: &str, root: &Path) -> anyhow::Result<()> {
    let walker = walkdir::WalkDir::new(root)
        .sort_by_file_name();
    let root = root.display().to_string();
    log::debug!("root: {root}");
    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            continue;
        }
        let epd = entry_path.display().to_string();
        let relpath = &epd[root.len()..];
        let abspath = format!("{remote_root}{relpath}");
        println!("* {epd} -> ::/{repository_id}{abspath}");
        //TODO: abspath must start with slash
        nexus.upload_file(repository_id, entry_path, &abspath).await?;
    }
    Ok(())
}

async fn download_op(nexus: NexusClient, repo_id: String, rpath: String, local_path: PathBuf) -> anyhow::Result<()> {
    log::debug!("Downloading {}::{} \t-> {}", repo_id, rpath,  local_path.display());
    nexus.download_file(&repo_id, &local_path, &rpath).await?;
    // http_get_file(client, furl, file).await?;
    log::debug!("downloaded {}", rpath);
    Ok(())
}

pub async fn http_download_tree(nexus: &NexusClient, repo_id: &str, remote_root: &str, local_root: &Path) -> anyhow::Result<()> {
    let mut handles: Vec<JoinHandle<anyhow::Result<()>>> = Vec::new();
    let mut subdirs = Vec::new();
    subdirs.push("".to_string());
    while let Some(subdir) = subdirs.pop() {
        let local_path = local_root.join(&subdir);
        if !local_path.exists() {
            tokio::fs::create_dir(local_path).await?;
        }
        // subdir: either empty, or has trailing slash; never leading slash
        let remote_dir = format!("{remote_root}{subdir}");
        let entries = fetch_dir_for_recurse(nexus, repo_id, &remote_dir).await?;
        for entry in &entries {
            if entry.leaf {
                let subpath = format!("{subdir}{}", entry.text);
                let local_path = local_root.join(&subpath);
                let remote_path = format!("{remote_root}{subpath}");
                handles.push(spawn(download_op(nexus.clone(),
                                               repo_id.to_string(),
                                               remote_path,
                                               local_path)));
            } else {
                let subpath = format!("{subdir}{}/", entry.text);
                subdirs.push(subpath);
            }
        }
    }
    let mut errors = Vec::new();
    let mut cnt = 0;
    for handle in handles {
        if let Err(e) = handle.await? {
            log::error!("{e}");
            errors.push(e);
        } else {
            cnt += 1;
        }
    }
    log::info!("Downloaded {cnt} files from ::/{repo_id}{remote_root} to {}/", local_root.display());
    if errors.is_empty() {
        Ok(())
    } else {
        log::error!("{} errors encountered, proceeding with first", errors.len());
        Err(errors.pop().unwrap())
    }
}

pub async fn fetch_dir(nexus: &NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
    let request = NexusRepository::nexus_readonly(repo_id)
        .list(remote_dir);
    let response = nexus.execute(request).await?;
    Ok(response.parsed().await?)
}

/// TODO we have to get rid of `dyn` in the NexusRequest, in order to be able to use it instead of RawRequest.
/// After doing so, this function can be fully replaced with the original [fetch_dir] one.
pub async fn fetch_dir_for_recurse(nexus: &NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
    log::trace!("{remote_dir}  START:");
    let nexus_url_path = NexusRepository::nexus_readonly(repo_id).repo_path;
    let request = RawRequest {
        method: Method::GET,
        url_suffix: format!("{nexus_url_path}{remote_dir}"),
        body: Default::default(),
        content_type: "application/json",
        accept: "application/json",
    };
    let response = nexus.execute_raw(request).await?;
    let resp: NexusResponseData = serde_json::from_str(&response.text().await?)?;
    let dir: Vec<DirEntry> = serde_json::from_value(resp.data)?;
    log::trace!("{remote_dir}  END.");
    Ok(dir)
}
