use std::path::Path;

use reqwest::Method;
use tokio::spawn;

use crate::{http_get_file, NexusClient, NexusRepository, RawRequest};
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

pub async fn http_download_tree(nexus: &NexusClient, repo_id: &str, remote_dir: &str, local_dir: &Path) -> anyhow::Result<()> {
    let local_dir = local_dir.canonicalize()?;
    log::info!("local_dir: {}", local_dir.display());
    // nexus.download_file()
    let mut handles = Vec::new();
    // let r = NexusRepository::nexus_readonly(repo_id);

    let mut remote_dirs= Vec::new();
    remote_dirs.push(remote_dir.to_string());
    while let Some(remote_dir) = remote_dirs.pop() {
        log::info!("REMOTE_DIR: {remote_dir}");
        let dir = fetch_dir_for_recurse(nexus, repo_id, &remote_dir).await?;
        for entry in dir {
            let local_path = local_dir.join(&entry.relative_path[1..]);
            if entry.leaf {
                let repo_id = repo_id.to_string();
                let nexus = nexus.clone();
                handles.push(spawn(async move {
                    let rpath = &entry.relative_path;
                    log::info!("Download {}::{} \t-> {}", repo_id, rpath, local_path.display());
                    nexus.download_file(&repo_id, &local_path, rpath).await.unwrap();
                    // http_get_file(client, furl, file).await?;
                    log::debug!("downloaded {}", rpath);
                }));
            } else {
                log::info!("--> REMOTE_DIR: {} (local: {})", entry.relative_path, local_path.display());
                if !local_path.exists() {
                    std::fs::create_dir(local_path)?;
                }
                remote_dirs.push(entry.relative_path);
            }
        }
    }
    for handle in handles {
        handle.await?;
    }
    Ok(())
}

pub async fn fetch_dir(nexus: &NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
    let request = NexusRepository::nexus_readonly(repo_id)
        .list(remote_dir);
    let response = nexus.execute(request).await?;
    Ok(response.parsed().await?)
}

/// TODO we have to get rid if `dyn` in the NexusRequest, in order to be able to use it instead of RawRequest.
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
