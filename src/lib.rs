use reqwest::header::CONTENT_TYPE;
use reqwest::Response;

pub use auth::get_credentials;
pub use auth::nexus_url;
pub use client::NexusClient;
pub use remote_sync::nexus_sync_up;
pub use restapi::{NexusRepository, StagingProfiles, StagingRepositories};
use restapi::APPLICATION_JSON;

pub mod model;
mod auth;
mod util;
mod client;
mod restapi;
mod remote_sync;

async fn check_status(response: Response) -> anyhow::Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let content_type = response.headers().get(CONTENT_TYPE);
    match content_type {
        None => {}
        Some(content_type) => {
            if content_type.to_str().unwrap().starts_with(APPLICATION_JSON) {
                let text = response.text().await?;
                anyhow::bail!("HTTP {} {}: with this JSON info: {text}",
                        status.as_str(),
                        status.canonical_reason().unwrap_or(""),
                    );
            }
        }
    }
    let text = response.text().await?;
    anyhow::bail!("HTTP {} {}: {text}",
            status.as_str(),
            status.canonical_reason().unwrap_or("")
        );
}

#[cfg(test)]
mod tests {
    use crate::{model, nexus_url};
    use crate::auth::get_credentials;
    use crate::client::NexusClient;
    use crate::restapi::StagingRepositories;

    #[tokio::test]
    async fn test_usage() -> anyhow::Result<()> {
        std::env::set_var("RUST_LOG", "trace");
        env_logger::init();
        let nexus_url = nexus_url()?;
        let (user, password) = get_credentials(&nexus_url)?;
        let nexus = NexusClient::login(nexus_url, &user, &password)?;
        let start_req = StagingRepositories::list();
        let start_resp = nexus.execute(start_req).await?;
        let list = start_resp.parsed().await?;
        for repo in list {
            println!("repo: {repo:?}");
        }
        Ok(())
    }

    #[test]
    fn test_xml() -> anyhow::Result<()> {
        let description = "hello";
        let body = model::PromoteRequest {
            data: model::PromoteRequestData {
                staged_repository_id: None,
                description: (!description.is_empty()).then(|| description.to_string()),
                target_repository_id: None,
            }
        };

        println!("serde_xml_rs: {}", serde_xml_rs::to_string(&body)?);
        Ok(())
    }
}
