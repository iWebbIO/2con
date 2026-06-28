use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Discriminates between a URL-backed subscription and a local manually-managed group.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum ProfileType {
    #[default]
    Subscription,
    LocalGroup,
}

impl fmt::Display for ProfileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileType::Subscription => write!(f, "subscription"),
            ProfileType::LocalGroup => write!(f, "local_group"),
        }
    }
}

impl FromStr for ProfileType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local_group" => Ok(ProfileType::LocalGroup),
            _ => Ok(ProfileType::Subscription), // Default to Subscription for unknown/existing rows
        }
    }
}

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
    #[serde(default)]
    pub profile_type: ProfileType,
}

