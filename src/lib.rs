use std::marker::PhantomData;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, USER_AGENT};
use reqwest::Method;
use serde::de::DeserializeOwned;
use url::Url;

pub use auth::get_credentials;

use crate::model::{NexusResponseData, StagingProfile, StagingProfileRepository};

pub mod model;
mod auth;

const APPLICATION_JSON: &str = "application/json";
// const APPLICATION_XML: &str = "application/xml";

pub struct NexusRequest<A> {
    method: Method,
    url_suffix: String,
    content_type: &'static str,
    accept: &'static str,
    _phantom: PhantomData<A>,
}

pub struct NexusResponse<A> {
    raw_response: reqwest::Response,
    _phantom: PhantomData<A>
}

impl<A: DeserializeOwned> NexusResponse<A> {
    pub async fn parsed(self) -> anyhow::Result<A> {
        // TODO: somehow, make this code dependent on A
        let resp: NexusResponseData = self.raw_response.json().await?;
        Ok(serde_json::from_value(resp.data)?)
    }
}

#[derive(Default)]
pub struct StagingProfiles;

impl StagingProfiles {
    pub fn list() -> NexusRequest<Vec<StagingProfile>> { todo!() }

    pub fn get(_profile_id_key: &str) -> NexusRequest<StagingProfile> { todo!() }

    pub fn start(_profile_id_key: &str, _description: &str) -> NexusRequest<String> { todo!() }

    pub fn drop(_staged_repository_id: &str, _repository_id: &str) -> NexusRequest<()> { todo!() }

    // pub fn finish(staged_repository_id: &str) -> NexusRequest { todo!() }
    // pub fn promote(staged_repository_id: &str) -> NexusRequest { todo!() }
}

pub struct StagingRepositories;

impl StagingRepositories {
    pub fn list() -> NexusRequest<Vec<StagingProfileRepository>> {
        NexusRequest {
            method: Method::GET,
            url_suffix: "/service/local/staging/profile_repositories".to_string(),
            content_type: APPLICATION_JSON,
            accept: APPLICATION_JSON,
            _phantom: Default::default(),
        }
    }
    pub fn get(_staged_repository_id: &str) -> NexusRequest<StagingProfileRepository> { todo!() }
}

/// https://oss.sonatype.org/nexus-staging-plugin/default/docs/index.html
pub struct NexusClient {
    base_url: Url,
    client: reqwest::Client,
    /// until https://github.com/seanmonstar/reqwest/pull/1398 gets implemented:
    credentials: (String,String),
}

impl NexusClient {

    pub fn new(base_url: Url, user: &str, password: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "https://github.com/pkozelka/nexus-client-rs".parse()?);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            base_url,
            client,
            credentials: (user.to_string(), password.to_string()),
        })
    }

    pub async fn execute<A>(&self, request: NexusRequest<A>) -> anyhow::Result<NexusResponse<A>> {
        let url = self.base_url.join(&request.url_suffix)?;
        log::info!("requesting: {url}");
        let raw_response = self.client.request(request.method, url)
            .basic_auth(&self.credentials.0, Some(&self.credentials.1))
            .header(ACCEPT, request.accept)
            .header(CONTENT_TYPE, request.content_type)
            .send().await?;
        Ok(NexusResponse {
            raw_response,
            _phantom: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{NexusClient, StagingRepositories};
    use crate::auth::get_credentials;

    #[tokio::test]
    async fn test_usage() -> anyhow::Result<()> {
        std::env::set_var("RUST_LOG", "trace");
        env_logger::init();
        let (server, user, password) = get_credentials()?;
        let nexus = NexusClient::new(server, &user, &password)?;
        let start_req = StagingRepositories::list();
        let start_resp = nexus.execute(start_req).await?;
        let list = start_resp.parsed().await?;
        for repo in list {
            println!("repo: {repo:?}");
        }
        Ok(())
    }
}
