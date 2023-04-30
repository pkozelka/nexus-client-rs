use std::path::Path;

use futures_util::StreamExt;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HeaderMap, USER_AGENT};
use reqwest::Method;
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use crate::restapi::{APPLICATION_JSON, APPLICATION_XML};
use crate::util;

type Extractor<A> = dyn FnOnce(&str) -> anyhow::Result<A>;

pub struct NexusRequest<A> {
    pub method: Method,
    pub url_suffix: String,
    pub body: String,
    pub content_type: &'static str,
    pub accept: &'static str,
    pub extractor: Box<Extractor<A>>,
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
        let response = crate::check_status(self.raw_response).await?;
        let text = response.text().await?;
        log::trace!("parsing response text: {text}");
        (self.extractor)(&text)
    }

    pub async fn check(self) -> anyhow::Result<reqwest::Response> {
        crate::check_status(self.raw_response).await
    }

    pub async fn text(self) -> anyhow::Result<String> {
        let response = crate::check_status(self.raw_response).await?;
        let text = response.text().await?;
        log::trace!("returning response text: {text}");
        Ok(text)
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
            .redirect(Policy::none())
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
            .redirect(Policy::none())
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
        let url = self.base_url.join(&format!("/service/local/staging/deployByRepositoryId/{staged_repository_id}{path}"))?;
        log::debug!("uploading(PUT) to: {url}");
        let http_req = self.client.request(Method::PUT, url.clone())
            .header(CONTENT_LENGTH, length)
            .body(vec)
            .build()?;
        let http_response = self.client.execute(http_req).await?;
        crate::check_status(http_response).await?;
        Ok(url)
    }

    pub async fn download_file(&self, staged_repository_id: &str, local_file: &Path, path: &str) -> anyhow::Result<Url> {
        if let Some(dir) = local_file.parent() {
            if ! dir.exists() {
                anyhow::bail!("Directory does not exist: {}", dir.display());
            }
        }
        let url = self.base_url.join(&format!("/service/local/repositories/{staged_repository_id}/content{path}"))?;
        log::debug!("downloading(GET) from: {url}");
        let http_response = self.client.request(Method::GET, url.clone())
            .send().await?;
        let http_response = crate::check_status(http_response).await?;
        let mut stream = http_response.bytes_stream();
        log::trace!("Creating file: {}", local_file.display());
        let mut file = File::create(local_file).await?;
        while let Some(chunk) = stream.next().await {
            file.write(&chunk?).await?;
        }
        Ok(url)
    }
}
