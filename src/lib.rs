use std::marker::PhantomData;
use anyhow::Error;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, USER_AGENT};
use reqwest::Method;
use serde::de::DeserializeOwned;
use url::Url;

pub use auth::get_credentials;

use crate::model::{NexusResponseData, StagingProfile, StagingProfileRepository};

pub mod model;
mod auth;

const APPLICATION_JSON: &str = "application/json";
const APPLICATION_XML: &str = "application/xml";

pub struct NexusRequest<A> {
    method: Method,
    url_suffix: String,
    content_type: &'static str,
    accept: &'static str,
    _phantom: PhantomData<A>,
}

pub struct NexusResponse<A>
{
    raw_response: reqwest::Response,
    extractor: Box<dyn FnOnce(&str) -> anyhow::Result<A>>,
    _phantom: PhantomData<A>
}

impl<A: DeserializeOwned> NexusResponse<A> {
    pub async fn parsed(self) -> anyhow::Result<A> {
        // TODO: somehow, make this code dependent on A
        let text = self.raw_response.text().await?;
        (self.extractor)(&text)
    }
}

fn parse_response_data<A: DeserializeOwned>(text: &str) -> Result<A, Error> {
    let resp: NexusResponseData = serde_json::from_str(&text)?;
    Ok(serde_json::from_value(resp.data)?)
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

impl<A> NexusRequest<A> {
    pub fn json_json(method: Method, url_suffix: String) -> Self {
        Self {
            method,
            url_suffix,
            content_type: APPLICATION_JSON,
            accept: APPLICATION_JSON,
            _phantom: Default::default(),
        }
    }

    pub fn xml_xml(method: Method, url_suffix: String) -> Self {
        Self {
            method,
            url_suffix,
            content_type: APPLICATION_XML,
            accept: APPLICATION_XML,
            _phantom: Default::default(),
        }
    }
}

pub struct StagingRepositories;

impl StagingRepositories {
    pub fn list() -> NexusRequest<Vec<StagingProfileRepository>> {
        NexusRequest::json_json(Method::GET, "/service/local/staging/profile_repositories".to_string())
    }

    pub fn get(staged_repository_id: &str) -> NexusRequest<StagingProfileRepository> {
        NexusRequest::json_json(Method::GET, format!("/service/local/staging/profile_repositories/{staged_repository_id}"))
    }
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

    pub async fn execute<A: DeserializeOwned + 'static>(&self, request: NexusRequest<A>) -> anyhow::Result<NexusResponse<A>> {
        let url = self.base_url.join(&request.url_suffix)?;
        log::info!("requesting: {url}");
        let raw_response = self.client.request(request.method, url)
            .basic_auth(&self.credentials.0, Some(&self.credentials.1))
            .header(ACCEPT, request.accept)
            .header(CONTENT_TYPE, request.content_type)
            .send().await?;
        Ok(NexusResponse {
            raw_response,
            extractor: Box::new(parse_response_data),
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
