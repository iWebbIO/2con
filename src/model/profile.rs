use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProfileItem {
    pub id: Option<i64>,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub protocol: String,  // e.g., "vmess", "vless", "ss", "trojan", "hysteria2", "tuic", "wireguard", "socks", "http"
    pub detail: String,    // JSON serialized string of ProtocolDetail
    pub delay: Option<i32>,
    pub is_active: bool,
    pub sub_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProtocolDetail {
    Vmess(VmessDetail),
    Vless(VlessDetail),
    Shadowsocks(ShadowsocksDetail),
    Trojan(TrojanDetail),
    Hysteria2(Hysteria2Detail),
    Tuic(TuicDetail),
    Wireguard(WireguardDetail),
    Socks(SocksDetail),
    Http(HttpDetail),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct VmessDetail {
    pub uuid: String,
    pub alter_id: u32,
    pub security: String,
    pub network: String, // tcp, ws, grpc, etc.
    pub host: String,
    pub path: String,
    pub tls: bool,
    pub sni: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct VlessDetail {
    pub uuid: String,
    pub encryption: String, // none
    pub network: String,
    pub host: String,
    pub path: String,
    pub tls: bool,
    pub sni: String,
    pub flow: String, // xtls-rprx-vision, etc.
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ShadowsocksDetail {
    pub method: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct TrojanDetail {
    pub password: String,
    pub network: String,
    pub host: String,
    pub path: String,
    pub tls: bool,
    pub sni: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Hysteria2Detail {
    pub auth: String,
    pub obfs: String,
    pub obfs_password: String,
    pub sni: String,
    pub insecure: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct TuicDetail {
    pub uuid: String,
    pub password: String,
    pub congestion_control: String,
    pub udp_relay_mode: String,
    pub sni: String,
    pub insecure: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct WireguardDetail {
    pub private_key: String,
    pub public_key: String,
    pub ip_addresses: Vec<String>,
    pub mtu: u16,
    pub reserved: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct SocksDetail {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct HttpDetail {
    pub username: Option<String>,
    pub password: Option<String>,
}
