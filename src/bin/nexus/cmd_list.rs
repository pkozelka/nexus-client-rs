use std::cmp::Ordering;

use reqwest::Method;

use nexus_client::{NexusClient, NexusRepository, RawRequest};
use nexus_client::model::{DirEntry, NexusResponseData};

use crate::DirFormat;
use crate::nexus_uri::NexusRemoteUri;

pub async fn cmd_list(nexus: NexusClient, nexus_uri: &NexusRemoteUri, format: DirFormat, recurse: bool) -> anyhow::Result<()> {
    let remote_dir = nexus_uri.repo_path_dir_or_err()?;
    if format == DirFormat::Json {
        let request = NexusRepository::nexus_readonly(&nexus_uri.repo_id)
            .list(remote_dir);
        let response = nexus.execute(request).await?;
        let json = response.text().await?;
        println!("{json}");
        return Ok(())
    }
    // Non-recursive
    let mut entries = fetch_dir(&nexus, &nexus_uri.repo_id, remote_dir).await?;
    if !recurse {
        entries.sort_unstable_by(by_text);
        let (files, subdirs) = split_files_subdirs(entries);
        subdirs.into_iter().for_each(|entry| print_entry(format, entry));
        files.into_iter().for_each(|entry| print_entry(format, entry));
        return Ok(())
    }

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DirChunk>(1000);
    sender.clone().send(DirChunk {
        container: None,
        entries,
    }).await?;
    let mut counter = 1;
    while let Some(chunk) = receiver.recv().await {
        if let Some(container) = chunk.container {
            log::debug!("CONTAINER: {container:?}");
            print_entry(format, container);
        }
        let mut entries = chunk.entries;
        entries.sort_unstable_by(by_text);
        let (files, subdirs) = split_files_subdirs(entries);
        files.into_iter().for_each(|entry| print_entry(format, entry));
        // recurse into subdirs
        for entry in subdirs {
            counter += 1;
            let nexus = nexus.clone();
            let sender = sender.clone();
            let repo_id = nexus_uri.repo_id.clone();
            let remote_dir = entry.relative_path.clone();
            tokio::spawn(async move {
                log::debug!("Listing for {remote_dir}");
                match fetch_dir_for_recurse(&nexus, &repo_id, &entry.relative_path).await {
                    Ok(entries) => {
                        if let Err(e) = sender.send(DirChunk { container: Some(entry), entries }).await {
                            log::error!("FATAL: Channel cannot send chunk: {e}");
                        };
                    }
                    Err(e) => {
                        log::error!("Failed to retrieve directory: {e}");
                    }
                }
            }).await?;
        }
        counter -= 1;
        log::info!("counter: {counter}");
        if counter == 0 { break }
    }
    Ok(())
}

#[derive(Debug)]
struct DirChunk {
    container: Option<DirEntry>,
    entries: Vec<DirEntry>,
}

fn print_entry(format: DirFormat, entry: DirEntry) {
    match format {
        DirFormat::Short => {
            let leaf = if entry.leaf { "" } else { "/" };
            println!("{}{leaf}", entry.text)
        },
        DirFormat::Long => {
            let size_or_dir = if entry.size_on_disk == -1 {
                "/".to_string()
            } else {
                format!("{}", entry.size_on_disk)
            };
            println!("{}\t{size_or_dir:>10}\t{}", entry.last_modified, &entry.relative_path[1..])
        },
        _ => panic!("Unknown format: {format:?}"),
    }
}

async fn fetch_dir(nexus: &NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
    let request = NexusRepository::nexus_readonly(repo_id)
        .list(remote_dir);
    let response = nexus.execute(request).await?;
    Ok(response.parsed().await?)
}

async fn fetch_dir_for_recurse(nexus: &NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
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
    Ok(dir)
}

fn split_files_subdirs(directory: Vec<DirEntry>) -> (Vec<DirEntry>, Vec<DirEntry>) {
    let mut files = Vec::with_capacity(directory.len());
    let mut subdirs = Vec::with_capacity(directory.len());
    for entry in directory {
        if entry.leaf {
            files.push(entry)
        } else {
            subdirs.push(entry)
        }
    }
    (files, subdirs)
}

fn by_text(a: &DirEntry, b: &DirEntry) -> Ordering {
    a.text.as_str().cmp(b.text.as_str())
}
