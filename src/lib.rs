use std::path::Path;

use anyhow::Error;
use reqwest::{Method, Response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, USER_AGENT};
use serde::de::DeserializeOwned;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;
use futures_util::StreamExt;

pub use auth::nexus_url;
pub use auth::get_credentials;

use crate::model::{NexusResponseData, PromoteResponse, StagingProfile, StagingProfileRepository};

pub mod model;
mod auth;
mod util;

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
    pub fn json_json<F>(method: Method, url_suffix: String, body: String, extractor: F) -> Self
        where F: FnOnce(&str) -> anyhow::Result<A> + 'static
    {
        Self {
            method,
            url_suffix,
            body,
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
        let response = check_status(self.raw_response).await?;
        let text = response.text().await?;
        log::trace!("parsing response text: {text}");
        (self.extractor)(&text)
    }

    pub async fn check(self) -> anyhow::Result<Response> {
        check_status(self.raw_response).await
    }

    pub async fn text(self) -> anyhow::Result<String> {
        let response = check_status(self.raw_response).await?;
        let text = response.text().await?;
        log::trace!("returning response text: {text}");
        Ok(text)
    }
}

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
                                "".to_string(),
                                json_extract_data,
        )
    }

    pub fn get(profile_id_key: &str) -> NexusRequest<StagingProfile> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/profiles/{profile_id_key}"),
                                "".to_string(),
                                json_extract_data,
        )
    }

    pub fn start(profile_id_key: &str, description: &str) -> NexusRequest<Option<String>> {
        let body = model::PromoteRequest {
            data: model::PromoteRequestData {
                staged_repository_id: None,
                description: (!description.is_empty()).then(|| description.to_string()),
                target_repository_id: None,
            }
        };
        let xml_body = serde_xml_rs::to_string(&body).unwrap();
        NexusRequest::xml_xml(Method::POST,
                              format!("/service/local/staging/profiles/{profile_id_key}/start"),
                              xml_body,
                              |text| {
                                  let promote_response: PromoteResponse = serde_xml_rs::from_str(text)?;
                                  Ok(promote_response.data.staged_repository_id)
                              },
        )
    }

    pub fn drop(profile_id_key: &str, repository_id: &str) -> NexusRequest<()> {
        // request body is too trivial to bother with JSON - TODO perhaps just fail on strange chars to prevent JSON injection
        let json_body = format!(r##"{{"data": {{"stagedRepositoryId":"{repository_id}"}} }}"##);
        // response body is empty in OK case, otherwise we don't even get to parse it here
        NexusRequest::json_json(Method::POST,
                                format!("/service/local/staging/profiles/{profile_id_key}/drop"),
                                json_body,
                                |_| Ok(()),
        )
    }

    pub fn promote(profile_id_key: &str, repository_id: &str) -> NexusRequest<String> {
        // request body is too trivial to bother with JSON - TODO perhaps just fail on strange chars to prevent JSON injection
        let json_body = format!(r##"{{"data": {{"stagedRepositoryId":"{repository_id}"}} }}"##);
        // response body is empty in OK case, otherwise we don't even get to parse it here
        NexusRequest::json_json(Method::POST,
                                format!("/service/local/staging/profiles/{profile_id_key}/promote"),
                                json_body,
                                |text| Ok(text.to_string()),
        )
    }

    pub fn finish(profile_id_key: &str, repository_id: &str) -> NexusRequest<String> {
        // request body is too trivial to bother with JSON - TODO perhaps just fail on strange chars to prevent JSON injection
        let json_body = format!(r##"{{"data": {{"stagedRepositoryId":"{repository_id}"}} }}"##);
        // response body is empty in OK case, otherwise we don't even get to parse it here
        NexusRequest::json_json(Method::POST,
                                format!("/service/local/staging/profiles/{profile_id_key}/finish"),
                                json_body,
                                |text| Ok(text.to_string()),
        )
    }
}

pub struct StagingRepositories;

impl StagingRepositories {
    pub fn list() -> NexusRequest<Vec<StagingProfileRepository>> {
        NexusRequest::json_json(Method::GET,
                                "/service/local/staging/profile_repositories".to_string(),
                                "".to_string(),
                                json_extract_data,
        )
    }

    pub fn get(staged_repository_id: &str) -> NexusRequest<StagingProfileRepository> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/repository/{staged_repository_id}"),
                                "".to_string(),
                                |text| Ok(serde_json::from_str(text)?),
        )
    }

    pub fn activity(staged_repository_id: &str) -> NexusRequest<String> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/repository/{staged_repository_id}/activity"),
                                "".to_string(),
                                |text| Ok(text.to_string()),
        )
    }
}

pub struct NexusRepository {
    repo_path: String,
}

impl NexusRepository {
    pub fn nexus_deploy(repository_id: &str) -> Self {
        let repo_path = format!("/service/local/staging/deployByRepositoryId/{repository_id}");
        Self { repo_path }
    }

    pub fn nexus_readonly(repository_id: &str) -> Self {
        let repo_path = format!("/service/local/repositories/{repository_id}/content");
        Self { repo_path }
    }

    pub fn delete(&self, path: &str) -> NexusRequest<()> {
        NexusRequest {
            method: Method::DELETE,
            url_suffix: format!("{}/{path}", self.repo_path),
            body: "".to_string(),
            content_type: "",
            accept: "",
            extractor: Box::new(|_|Ok(())),
        }
    }
}

/// https://oss.sonatype.org/nexus-staging-plugin/default/docs/index.html
pub struct NexusClient {
    base_url: Url,
    client: reqwest::Client,
}

impl NexusClient {
    pub fn login(base_url: Url, user: &str, password: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "https://github.com/pkozelka/nexus-client-rs".parse()?);
        headers.insert(AUTHORIZATION,  util::basic_auth(user, Some(password)));
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            base_url,
            client,
        })
    }

    pub fn anonymous(base_url: Url) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, "https://github.com/pkozelka/nexus-client-rs".parse()?);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            base_url,
            client,
        })
    }

    pub async fn execute<A: DeserializeOwned + 'static>(&self, request: NexusRequest<A>) -> anyhow::Result<NexusResponse<A>> {
        let url = self.base_url.join(&request.url_suffix)?;
        log::debug!("requesting: {} {url}", request.method);
        let http_request = self.client.request(request.method, url)
            .header(ACCEPT, request.accept);
        let http_request = if !request.body.is_empty() {
            log::debug!("- sending '{}' body: {}", request.content_type, request.body);
            http_request
                .header(CONTENT_TYPE, request.content_type)
                .body(request.body)
        } else {
            http_request
        };
        let http_response = http_request.send().await?;
        let content_length = http_response.content_length().unwrap_or(0);
        log::debug!("- received '{:?}' body, content-length = {content_length}", http_response.headers().get(CONTENT_TYPE));
        Ok(NexusResponse {
            raw_response: http_response,
            extractor: request.extractor,
        })
    }

    pub async fn upload_file(&self, staged_repository_id: &str, file: &Path, path: &str) -> anyhow::Result<Url> {
        let mut file = File::open(file).await?;
        let mut vec = Vec::new();
        file.read_to_end(&mut vec).await?;
        let length = file.metadata().await?.len();
        let url = self.base_url.join(&format!("/service/local/staging/deployByRepositoryId/{staged_repository_id}/{path}"))?;
        log::debug!("uploading(PUT) to: {url}");
        let http_req = self.client.request(Method::PUT, url.clone())
            .header(CONTENT_LENGTH, length)
            .body(vec)
            .build()?;
        let http_response = self.client.execute(http_req).await?;
        check_status(http_response).await?;
        Ok(url)
    }

    pub async fn download_file(&self, staged_repository_id: &str, local_file: &Path, path: &str) -> anyhow::Result<Url> {
        if let Some(dir) = local_file.parent() {
            if ! dir.exists() {
                anyhow::bail!("Directory does not exist: {}", dir.display());
            }
        }
        let url = self.base_url.join(&format!("/service/local/repositories/{staged_repository_id}/content/{path}"))?;
        log::debug!("downloading(GET) from: {url}");
        let http_response = self.client.request(Method::GET, url.clone())
            .send().await?;
        let http_response = check_status(http_response).await?;
        let mut stream = http_response.bytes_stream();
        log::trace!("Creating file: {}", local_file.display());
        let mut file = File::create(local_file).await?;
        while let Some(chunk) = stream.next().await {
            file.write(&chunk?).await?;
        }
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use crate::{model, NexusClient, StagingRepositories};
    use crate::auth::get_credentials;

    #[tokio::test]
    async fn test_usage() -> anyhow::Result<()> {
        std::env::set_var("RUST_LOG", "trace");
        env_logger::init();
        let (server, user, password) = get_credentials()?;
        let nexus = NexusClient::login(server, &user, &password)?;
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
