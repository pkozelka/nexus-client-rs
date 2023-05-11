use std::cmp::Ordering;

use nexus_client::{NexusClient, remote_sync};
use nexus_client::model::DirEntry;

use crate::DirFormat;
use crate::nexus_uri::NexusRemoteUri;

pub async fn cmd_list(nexus: NexusClient, nexus_uri: &NexusRemoteUri, dir_printer: DirPrinter, recurse: bool) -> anyhow::Result<()> {
    let remote_dir = nexus_uri.repo_path_dir_or_err()?;
    // Non-recursive
    let mut entries = remote_sync::fetch_dir(&nexus, &nexus_uri.repo_id, remote_dir).await?;
    if !recurse {
        entries.sort_unstable_by(by_text);
        let (files, subdirs) = split_files_subdirs(entries);
        dir_printer.print_dir(&subdirs);
        dir_printer.print_dir(&files);
        return Ok(())
    }

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<DirChunk>(1000);
    sender.clone().send(DirChunk {
        container: None,
        entries,
    }).await?;
    let mut pending = 1;
    while let Some(chunk) = receiver.recv().await {
        if let Some(container) = chunk.container {
            log::debug!("CONTAINER: {container:?}");
            dir_printer.print_entry(&container);
        }
        let mut entries = chunk.entries;
        entries.sort_unstable_by(by_text);
        let (files, subdirs) = split_files_subdirs(entries);
        dir_printer.print_dir(&files);
        // recurse into subdirs
        for entry in subdirs {
            pending += 1;
            let nexus = nexus.clone();
            let sender = sender.clone();
            let repo_id = nexus_uri.repo_id.clone();
            let remote_dir = entry.relative_path.clone();
            tokio::spawn(async move {
                log::debug!("Listing for {remote_dir}");
                match remote_sync::fetch_dir_for_recurse(&nexus, &repo_id, &entry.relative_path).await {
                    Ok(entries) => {
                        if let Err(e) = sender.send(DirChunk { container: Some(entry), entries }).await {
                            log::error!("FATAL: Channel cannot send chunk: {e}");
                        };
                    }
                    Err(e) => {
                        log::error!("Failed to retrieve directory: {e}");
                    }
                }
            });
        }
        pending -= 1;
        log::debug!("pending: {pending}");
        if pending == 0 { break }
    }
    Ok(())
}

#[derive(Debug)]
struct DirChunk {
    container: Option<DirEntry>,
    entries: Vec<DirEntry>,
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

pub struct DirPrinter {
    pub format: DirFormat,
}

impl DirPrinter {
    pub fn print_dir(&self, entries: &[DirEntry]) {
        entries.iter().for_each(|entry| self.print_entry(entry));
    }

    fn print_entry(&self, entry: &DirEntry) {
        match &self.format {
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
            format => panic!("Unknown format: {format:?}"),
        }
    }

}
