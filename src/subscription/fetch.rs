use crate::model::{ProfileItem, ProtocolDetail, VmessDetail, VlessDetail, ShadowsocksDetail, TrojanDetail};
use crate::subscription::parser::{decode_base64, parse_proxy_uri};
use std::error::Error;
use serde::Deserialize;
use serde_json::Value;

pub async fn fetch_subscription(url: &str) -> Result<Vec<ProfileItem>, Box<dyn Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
        
    let response = client.get(url)
        .header("User-Agent", "clash") // Mock clash user-agent to get clash/v2ray profiles
        .send()
        .await?;

    let text = response.text().await?;
    
    // 1. Try Clash YAML first
    if let Ok(clash_list) = parse_clash_yaml(&text) {
        if !clash_list.is_empty() {
            return Ok(clash_list);
        }
    }

    // 2. Try SIP008 JSON format
    if let Ok(sip_list) = parse_sip008(&text) {
        if !sip_list.is_empty() {
            return Ok(sip_list);
        }
    }

    // 3. Try base64 decoded URI list or raw URI list
    let decoded_text = decode_base64(&text).unwrap_or(text);
    
    let mut profiles = Vec::new();
    for line in decoded_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(profile) = parse_proxy_uri(line) {
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

// Parse Clash YAML profiles
fn parse_clash_yaml(yaml_content: &str) -> Result<Vec<ProfileItem>, Box<dyn Error + Send + Sync>> {
    #[derive(Deserialize)]
    struct ClashConfig {
        proxies: Option<Vec<Value>>,
    }

    let config: ClashConfig = serde_yaml::from_str(yaml_content)?;
    let mut list = Vec::new();

    if let Some(proxies) = config.proxies {
        for p in proxies {
            if let Some(profile) = map_clash_proxy_to_profile(&p) {
                list.push(profile);
            }
        }
    }

    Ok(list)
}

fn map_clash_proxy_to_profile(p: &Value) -> Option<ProfileItem> {
    let name = p["name"].as_str()?.to_string();
    let server = p["server"].as_str()?.to_string();
    let port = p["port"].as_u64()? as u16;
    let proto_type = p["type"].as_str()?;

    let detail = match proto_type {
        "vmess" => {
            let uuid = p["uuid"].as_str().unwrap_or_default().to_string();
            let alter_id = p["alterId"].as_u64().unwrap_or(0) as u32;
            let security = p["cipher"].as_str().unwrap_or("auto").to_string();
            let network = p["network"].as_str().unwrap_or("tcp").to_string();
            let host = p["ws-opts"]["headers"]["Host"].as_str().unwrap_or_default().to_string();
            let path = p["ws-opts"]["path"].as_str().unwrap_or_default().to_string();
            let tls = p["tls"].as_bool().unwrap_or(false);
            let sni = p["servername"].as_str().unwrap_or_default().to_string();

            ProtocolDetail::Vmess(VmessDetail {
                uuid,
                alter_id,
                security,
                network,
                host,
                path,
                tls,
                sni,
            })
        }
        "vless" => {
            let uuid = p["uuid"].as_str().unwrap_or_default().to_string();
            let network = p["network"].as_str().unwrap_or("tcp").to_string();
            let host = p["ws-opts"]["headers"]["Host"].as_str().unwrap_or_default().to_string();
            let path = p["ws-opts"]["path"].as_str().unwrap_or_default().to_string();
            let tls = p["tls"].as_bool().unwrap_or(false);
            let sni = p["servername"].as_str().unwrap_or_default().to_string();
            let flow = p["flow"].as_str().unwrap_or_default().to_string();

            ProtocolDetail::Vless(VlessDetail {
                uuid,
                encryption: "none".to_string(),
                network,
                host,
                path,
                tls,
                sni,
                flow,
            })
        }
        "ss" => {
            let cipher = p["cipher"].as_str().unwrap_or("chacha20-ietf-poly1305").to_string();
            let password = p["password"].as_str().unwrap_or_default().to_string();

            ProtocolDetail::Shadowsocks(ShadowsocksDetail {
                method: cipher,
                password,
            })
        }
        "trojan" => {
            let password = p["password"].as_str().unwrap_or_default().to_string();
            let sni = p["servername"].as_str().unwrap_or_default().to_string();
            let tls = p["tls"].as_bool().unwrap_or(true);

            ProtocolDetail::Trojan(TrojanDetail {
                password,
                network: "tcp".to_string(),
                host: "".to_string(),
                path: "".to_string(),
                tls,
                sni,
            })
        }
        _ => return None,
    };

    let detail_json = serde_json::to_string(&detail).ok()?;

    Some(ProfileItem {
        id: None,
        name,
        address: server,
        port,
        protocol: proto_type.to_string(),
        detail: detail_json,
        delay: None,
        is_active: false,
        sub_id: None,
    })
}

// Parse SIP008 (Shadowsocks JSON schema)
fn parse_sip008(json_content: &str) -> Result<Vec<ProfileItem>, Box<dyn Error + Send + Sync>> {
    #[derive(Deserialize)]
    struct SipServer {
        id: Option<String>,
        server: String,
        server_port: u16,
        password: String,
        method: String,
        remarks: Option<String>,
    }

    #[derive(Deserialize)]
    struct Sip008Config {
        servers: Vec<SipServer>,
    }

    let config: Sip008Config = serde_json::from_str(json_content)?;
    let mut list = Vec::new();

    for s in config.servers {
        let name = s.remarks.or(s.id).unwrap_or_else(|| "SS Node".to_string());
        let detail = ProtocolDetail::Shadowsocks(ShadowsocksDetail {
            method: s.method,
            password: s.password,
        });
        
        let detail_json = serde_json::to_string(&detail)?;
        list.push(ProfileItem {
            id: None,
            name,
            address: s.server,
            port: s.server_port,
            protocol: "ss".to_string(),
            detail: detail_json,
            delay: None,
            is_active: false,
            sub_id: None,
        });
    }

    Ok(list)
}
