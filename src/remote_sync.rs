use std::path::Path;

use crate::NexusClient;

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
