use anyhow::Error;

use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, USER_AGENT};
use reqwest::{Method, Response};
use serde::de::DeserializeOwned;
use url::Url;

pub use auth::get_credentials;

use crate::model::{NexusResponseData, StagingProfile, StagingProfileRepository};

pub mod model;
mod auth;

const APPLICATION_JSON: &str = "application/json";
const APPLICATION_XML: &str = "application/xml";

type Extractor<A> = dyn FnOnce(&str) -> anyhow::Result<A>;

pub struct NexusRequest<A> {
    method: Method,
    url_suffix: String,
    body: String,
    content_type: &'static str,
    accept: &'static str,
    extractor: Box<Extractor<A>>,
}

impl<A: DeserializeOwned + 'static> NexusRequest<A> {
    pub fn json_json<F>(method: Method, url_suffix: String, extractor: F) -> Self
        where F: FnOnce(&str) -> anyhow::Result<A> + 'static
    {
        Self {
            method,
            url_suffix,
            body: "".to_string(),
            content_type: APPLICATION_JSON,
            accept: APPLICATION_JSON,
            extractor: Box::new(extractor),
        }
    }

    pub fn xml_xml<F>(method: Method, url_suffix: String, body: String, extractor: F) -> Self
        where F: FnOnce(&str) -> anyhow::Result<A> + 'static
    {
        Self {
            method,
            url_suffix,
            body,
            content_type: APPLICATION_XML,
            accept: APPLICATION_XML,
            extractor: Box::new(extractor),
        }
    }
}

pub struct NexusResponse<A>
{
    raw_response: reqwest::Response,
    extractor: Box<Extractor<A>>,
}

impl<A: DeserializeOwned> NexusResponse<A> {
    pub async fn parsed(self) -> anyhow::Result<A> {
        let response = Self::check_status(self.raw_response).await?;
        let text = response.text().await?;
        (self.extractor)(&text)
    }

    pub async fn text(self) -> anyhow::Result<String> {
        let response = Self::check_status(self.raw_response).await?;
        Ok(response.text().await?)
    }

    async fn check_status(response: Response) -> anyhow::Result<Response> {
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await?;
            anyhow::bail!("HTTP {} {}: {text}",
                status.as_str(),
                status.canonical_reason().unwrap_or("")
            );
        }
        Ok(response)
    }
}

/// Extracts content carried inside JSON "data" wrapping object
fn json_extract_data<A: DeserializeOwned>(text: &str) -> Result<A, Error> {
    let resp: NexusResponseData = serde_json::from_str(&text)?;
    Ok(serde_json::from_value(resp.data)?)
}

#[derive(Default)]
pub struct StagingProfiles;

impl StagingProfiles {
    pub fn list() -> NexusRequest<Vec<StagingProfile>> {
        NexusRequest::json_json(Method::GET,
                                "/service/local/staging/profiles".to_string(),
                                json_extract_data
        )
    }

    pub fn get(profile_id_key: &str) -> NexusRequest<StagingProfile> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/profiles/{profile_id_key}"),
                                json_extract_data
        )
    }

    pub fn start(profile_id_key: &str, description: &str) -> NexusRequest<String> {
        let _body = model::PromoteRequestData { //todo
            staged_repository_id: None,
            description: (!description.is_empty()).then(|| description.to_string()),
            target_repository_id: None,
        };
        NexusRequest::xml_xml(Method::POST,
                              format!("/service/local/staging/profiles/{profile_id_key}/start"),
            format!(r#"<promoteRequest>
    <data>
        <description>{description}</description>
    </data>
</promoteRequest>"#),
            |text| Ok(text.to_string())
        )
    }

    pub fn drop(_staged_repository_id: &str, _repository_id: &str) -> NexusRequest<()> { todo!() }

    // pub fn finish(staged_repository_id: &str) -> NexusRequest { todo!() }
    // pub fn promote(staged_repository_id: &str) -> NexusRequest { todo!() }
}

pub struct StagingRepositories;

impl StagingRepositories {
    pub fn list() -> NexusRequest<Vec<StagingProfileRepository>> {
        NexusRequest::json_json(Method::GET,
                                "/service/local/staging/profile_repositories".to_string(),
                                json_extract_data
        )
    }

    pub fn get(staged_repository_id: &str) -> NexusRequest<StagingProfileRepository> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/repository/{staged_repository_id}"),
                                |text| Ok(serde_json::from_str(text)?)
        )
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
            .body(request.body)
            .send().await?;
        Ok(NexusResponse {
            raw_response,
            extractor: request.extractor,
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
