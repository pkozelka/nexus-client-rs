use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteRequestData {
    pub staged_repository_id: Option<String>,
    pub description: Option<String>,
    pub target_repository_id: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NexusResponseData {
    pub data: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOfObjects<T> {
    pub data: Vec<T>,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingProfile {
    #[serde(rename = "resourceURI")]
    pub resource_uri: String,
    pub id: String,
    pub name: String,
    pub repository_template_id: String,
    pub repository_type: String,
    pub repository_target_id: String,
    pub in_progress: bool,
    pub order: i64,
    #[serde(rename = "deployURI")]
    pub deploy_uri: String,
    pub target_groups: Vec<String>,
    pub finish_notify_roles: Vec<Value>,
    pub promotion_notify_roles: Vec<Value>,
    pub drop_notify_roles: Vec<Value>,
    pub close_rule_sets: Vec<String>,
    pub promote_rule_sets: Vec<Value>,
    pub promotion_target_repository: String,
    pub mode: String,
    pub finish_notify_creator: bool,
    pub promotion_notify_creator: bool,
    pub drop_notify_creator: bool,
    pub auto_staging_disabled: bool,
    pub repositories_searchable: bool,
    pub properties: Properties,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Properties {
    #[serde(rename = "@class")]
    pub class: String,
}
