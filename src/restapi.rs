use reqwest::Method;
use serde::de::DeserializeOwned;
use anyhow::Error;
use crate::client::NexusRequest;
use crate::model;
use crate::model::{NexusResponseData, PromoteResponse, StagingProfile, StagingProfileRepository};

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
