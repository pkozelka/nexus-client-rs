use std::path::PathBuf;

use clap::Subcommand;

use nexus_client::NexusRepository;

use crate::DirFormat;

pub async fn cmd_content(deploy_command: ContentCommands, repository_id: &str, remote_path: &str) -> anyhow::Result<()> {
    match deploy_command {
        ContentCommands::Upload { local_path } => {
            let nexus = crate::nexus_client()?;
            let url = nexus.upload_file(&repository_id, &local_path, remote_path).await?;
            log::info!("File {} uploaded to {url}", local_path.display());
        }
        ContentCommands::Download { local_path } => {
            let nexus = crate::nexus_public_client()?;
            let url = nexus.download_file(&repository_id, &local_path, remote_path).await?;
            log::info!("File {} downloaded from {url}", local_path.display());
        }
        ContentCommands::Delete => {
            let nexus = crate::nexus_client()?;
            let request = NexusRepository::nexus_readwrite(&repository_id)
                .delete(remote_path);
            let response = nexus.execute(request).await?;
            response.check().await?;
            println!("Deleted: {remote_path} from repository {repository_id}");
        }
        ContentCommands::DirectoryListing { format } => {
            let nexus = crate::nexus_public_client()?;
            let request = NexusRepository::nexus_readonly(&repository_id)
                .list(remote_path);
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
                        DirFormat::RelPath => println!("{}", entry.relative_path),
                        DirFormat::Uri => println!("{}", entry.resource_uri),
                        DirFormat::Long => println!("{:10} {} {}\t # {entry:?}", entry.size_on_disk, entry.last_modified, entry.relative_path),
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
    },
    /// Download single file
    #[clap(name="get")]
    Download {
        #[arg(short,long)]
        local_path: PathBuf,
    },
    /// Delete a path (file of directory with its contents)
    #[clap(name="rm")]
    Delete,
    /// List a directory
    #[clap(name="ls")]
    DirectoryListing {
        #[arg(long,default_value="long")]
        format: DirFormat,
    },
}
