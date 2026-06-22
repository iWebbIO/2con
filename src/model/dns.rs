use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DnsItem {
    pub name: String,
    pub address: String, // e.g., "8.8.8.8", "udp://8.8.8.8", "https://1.1.1.1/dns-query"
    pub domains: Option<Vec<String>>,
}
