use anyhow::Error;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::client::NexusRequest;
use crate::model;
use crate::model::{NexusResponseData, PromoteResponse, StagingActivity, StagingProfile, StagingProfileRepository};

pub const APPLICATION_JSON: &str = "application/json";
pub const APPLICATION_XML: &str = "application/xml";

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

    pub fn start(profile_id_key: &str, description: &str) -> NexusRequest<PromoteResponse> {
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
                              |text| Ok(serde_xml_rs::from_str(text)?),
        )
    }


    /// NOT WORKING - set repository's description.
    /// It was reverse-engineered from OSSRH but it doesn't work even there.
    /// Let's update once the real one is found.
    pub fn describe(profile_id_key: &str, repository_id: &str, description: &str) -> NexusRequest<String> {
        // request body is too trivial to bother with JSON - TODO perhaps just fail on strange chars to prevent JSON injection
        let json_expr_description = if description.is_empty() {
            "".to_string()
        } else {
            format!(r##", "description":"{}""##, json_escape(description))
        };
        let json_body = format!(r##"{{"data": {{"repositoryId":"{repository_id}"{json_expr_description}}}}}"##);
        // response body is empty in OK case, otherwise we don't even get to parse it here
        NexusRequest::json_json(Method::PUT,
                                format!("/service/local/staging/profiles/{profile_id_key}"),
                                json_body,
                                |text| Ok(text.to_string()),
        )
    }

    pub fn finish(profile_id_key: &str, repository_id: &str, description: &str) -> NexusRequest<String> {
        // request body is too trivial to bother with JSON - TODO perhaps just fail on strange chars to prevent JSON injection
        let json_expr_description = if description.is_empty() {
            "".to_string()
        } else {
            format!(r##", "description":"{}""##, json_escape(description))
        };
        let json_body = format!(r##"{{"data": {{"stagedRepositoryId":"{repository_id}"{json_expr_description}}}}}"##);
        // response body is empty in OK case, otherwise we don't even get to parse it here
        NexusRequest::json_json(Method::POST,
                                format!("/service/local/staging/profiles/{profile_id_key}/finish"),
                                json_body,
                                |text| Ok(text.to_string()),
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
}

fn json_escape(text: &str) -> String {
    text.replace("\"", "\\\"")
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

    pub fn activity(staged_repository_id: &str) -> NexusRequest<Vec<StagingActivity>> {
        NexusRequest::json_json(Method::GET,
                                format!("/service/local/staging/repository/{staged_repository_id}/activity"),
                                "".to_string(),
                                |text| Ok(serde_json::from_str(text)?),
        )
    }

    //TODO more elegant solution would be great here
    pub fn activity_xml(staged_repository_id: &str) -> NexusRequest<String> {
        NexusRequest::xml_xml(Method::GET,
                              format!("/service/local/staging/repository/{staged_repository_id}/activity"),
                              "".to_string(),
                              |text| Ok(text.to_string()),
        )
    }
}

pub struct NexusRepository {
    pub /*todo just for now*/ repo_path: String,
}

const STAGING_PREFIX: &str = "@staging:";

impl NexusRepository {
    pub fn nexus_readwrite(repository_id: &str) -> Self {
        let repo_path = if repository_id.starts_with(STAGING_PREFIX) {
            let repository_id = &repository_id[STAGING_PREFIX.len()..];
            format!("/service/local/staging/deployByRepositoryId/{repository_id}")
        } else {
            format!("/service/local/repositories/{repository_id}/content")
        };
        Self { repo_path }
    }

    pub fn nexus_readonly(repository_id: &str) -> Self {
        let repository_id = if repository_id.starts_with(STAGING_PREFIX) {
            &repository_id[STAGING_PREFIX.len()..]
        } else {
            repository_id
        };
        let repo_path = format!("/service/local/repositories/{repository_id}/content");
        Self { repo_path }
    }

    pub fn delete(&self, path: &str) -> NexusRequest<()> {
        NexusRequest {
            method: Method::DELETE,
            url_suffix: format!("{}{path}", self.repo_path),
            body: "".to_string(),
            content_type: "",
            accept: "",
            extractor: Box::new(|_| Ok(())),
        }
    }

    pub fn list(&self, path: &str) -> NexusRequest<Vec<model::DirEntry>> {
        NexusRequest::json_json(
            Method::GET,
            format!("{}{path}", self.repo_path),
            "".to_string(),
            json_extract_data,
        )
    }
}
