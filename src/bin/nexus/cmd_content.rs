use std::path::PathBuf;

use clap::Subcommand;

pub async fn cmd_content(content_command: ContentCommands, repository_id: &str) -> anyhow::Result<()> {
    match content_command {
        ContentCommands::Upload { local_path, path } => {
            //TODO if not --force, require the repository to be open and non-transitioning
            let nexus = crate::nexus_client()?;
            let url = nexus.upload_file(&repository_id, &local_path, &path).await?;
            log::info!("File {} uploaded to {url}", local_path.display());
        }
        ContentCommands::Download { local_path, path } => {
            let nexus = crate::nexus_public_client()?;
            let url = nexus.download_file(&repository_id, &local_path, &path).await?;
            log::info!("File {} downloaded from {url}", local_path.display());
        }
    }
    Ok(())
}

#[derive(Subcommand)]
pub enum ContentCommands {
    //TODO: consider more convenient syntax for local/remote paths, allowing only one side to contain the file name
    //TODO: support multiple files on upload, into single directory target
    /// Upload single file
    #[clap(name="put")]
    Upload {
        #[arg(short,long)]
        local_path: PathBuf,
        /// remote path
        #[clap(default_value="/")]
        path: String,
    },
    /// Download single file
    #[clap(name="get")]
    Download {
        #[arg(short,long)]
        local_path: PathBuf,
        /// remote path
        #[clap(default_value="/")]
        path: String,
    },
}
