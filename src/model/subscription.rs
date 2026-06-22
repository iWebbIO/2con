use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SubItem {
    pub id: Option<i64>,
    pub name: String,
    pub url: String,
    pub last_updated: Option<String>,
    pub update_interval: u32, // in hours
    pub upload: Option<u64>,  // bytes
    pub download: Option<u64>, // bytes
    pub total: Option<u64>,   // total bytes allowed
    pub expire: Option<String>,
}
