use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoutingItem {
    pub id: Option<i64>,
    pub name: String,
    pub rules: String, // JSON serialized Vec<RoutingRule>
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoutingRule {
    pub outbound: String, // "proxy", "direct", "block"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Vec<String>>,
}
