use std::path::PathBuf;

use clap::Subcommand;

use nexus_client::NexusRepository;

use crate::DirFormat;

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
        ContentCommands::Delete { path } => {
            //TODO if not --force, require the repository to be open and non-transitioning
            let nexus = crate::nexus_client()?;
            let request = NexusRepository::nexus_readwrite(&repository_id)
                .delete(&path);
            let response = nexus.execute(request).await?;
            response.check().await?;
            log::warn!("Deleted: {path} from repository {repository_id}");
        }
        ContentCommands::DirectoryListing { format, path } => {
            let nexus = crate::nexus_public_client()?;
            let request = NexusRepository::nexus_readonly(&repository_id)
                .list(&path);
            let response = nexus.execute(request).await?;
            if format == DirFormat::Json {
                let json = response.text().await?;
                println!("{json}");
            } else {
                for entry in response.parsed().await? {
                    match format {
                        DirFormat::Short => {
                            let leaf = if entry.leaf { "" } else { "/" };
                            println!("{}{leaf}", entry.text)
                        },
                        DirFormat::Long => {
                            let size_or_dir = if entry.size_on_disk == -1 {
                                " DIRECTORY".to_string()
                            } else {
                                format!("{:10}", entry.size_on_disk)
                            };
                            println!("{}\t{size_or_dir}\t{}", entry.last_modified, entry.relative_path)
                        },
                        _ => panic!("Unknown format: {format:?}"),
                    }
                }
            }
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
    /// Delete a path (file of directory with its contents)
    #[clap(name="rm")]
    Delete {
        /// remote path
        path: String,
    },
    /// List a directory
    #[clap(name="ls")]
    DirectoryListing {
        #[arg(long,default_value="short")]
        format: DirFormat,
        /// remote path
        #[clap(default_value="/")]
        path: String,
    },
}
