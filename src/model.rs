use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteRequest {
    pub data: Vec<StagingProfileRepository>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingProfileRepositories {
    pub data: Vec<StagingProfileRepository>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingProfileRepository {
    pub profile_id: String,
    pub profile_name: String,
    pub profile_type: String,
    pub repository_id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub policy: String,
    pub user_id: String,
    pub user_agent: String,
    pub ip_address: String,
    #[serde(rename = "repositoryURI")]
    pub repository_uri: String,
    pub created: String,
    pub created_date: String,
    pub created_timestamp: i64,
    pub updated: String,
    pub updated_date: String,
    pub updated_timestamp: i64,
    pub description: String,
    pub provider: String,
    pub release_repository_id: String,
    pub release_repository_name: String,
    pub notifications: i64,
    pub transitioning: bool,
}
