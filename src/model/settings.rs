use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppSettings {
    pub core_type: String, // "xray" or "sing-box"
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub socks_port: u16,
    pub http_port: u16,
    pub dns_server: String,
    pub routing_preset: String, // "Bypass LAN & China", "Global", etc.
    pub selected_profile_id: Option<i64>,
    pub log_level: String, // "debug", "info", "warning", "error"
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            core_type: "sing-box".to_string(),
            system_proxy_enabled: false,
            tun_enabled: false,
            socks_port: 20808,
            http_port: 20809,
            dns_server: "8.8.8.8".to_string(),
            routing_preset: "Bypass LAN & China".to_string(),
            selected_profile_id: None,
            log_level: "info".to_string(),
        }
    }
}
