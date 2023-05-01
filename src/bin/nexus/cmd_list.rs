use std::cmp::Ordering;

use nexus_client::{NexusClient, NexusRepository};
use nexus_client::model::DirEntry;

use crate::DirFormat;
use crate::nexus_uri::NexusRemoteUri;

pub async fn cmd_list(format: DirFormat, nexus_uri: &NexusRemoteUri, nexus: NexusClient) -> anyhow::Result<()> {
    let remote_dir = nexus_uri.repo_path_dir_or_err()?;
    if format == DirFormat::Json {
        let request = NexusRepository::nexus_readonly(&nexus_uri.repo_id)
            .list(remote_dir);
        let response = nexus.execute(request).await?;
        let json = response.text().await?;
        println!("{json}");
    } else {
        let mut dir = fetch_dir(nexus, &nexus_uri.repo_id, remote_dir).await?;
        dir.sort_unstable_by(by_text);
        let (files, subdirs) = split_files_subdirs(dir);
        subdirs.into_iter().for_each(|entry| print_entry(format, entry));
        files.into_iter().for_each(|entry| print_entry(format, entry));
    }
    Ok(())
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

async fn fetch_dir(nexus: NexusClient, repo_id: &str, remote_dir: &str) -> anyhow::Result<Vec<DirEntry>> {
    let request = NexusRepository::nexus_readonly(repo_id)
        .list(remote_dir);
    let response = nexus.execute(request).await?;
    Ok(response.parsed().await?)
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
