use std::path::Path;

use crate::NexusClient;

/// Full blind upload of a directory
/// TODO implement optional removal of existing extra (or different) files in remote repo
pub async fn nexus_sync_up(nexus: &NexusClient, repository_id: &str, remote_root: &str, root: &Path) -> anyhow::Result<()> {
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
        let abspath = format!("{remote_root}/{relpath}");
        println!("* {abspath} <- {epd}");
        //TODO: abspath must start with slash
        nexus.upload_file(repository_id, entry_path, &abspath).await?;
    }
    Ok(())
}
