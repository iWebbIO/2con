use crate::model::{ProfileItem, ProtocolDetail, VmessDetail, VlessDetail, ShadowsocksDetail, TrojanDetail, Hysteria2Detail, TuicDetail, SocksDetail, HttpDetail};
use url::Url;
use base64::{Engine as _, engine::general_purpose};
use std::error::Error;

pub fn decode_base64(s: &str) -> Option<String> {
    let cleaned = s.trim().replace('-', "+").replace('_', "/");
    let mut padded = cleaned.clone();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    if let Ok(bytes) = general_purpose::STANDARD.decode(&padded) {
        String::from_utf8(bytes).ok()
    } else {
        general_purpose::STANDARD.decode(s).ok().and_then(|bytes| String::from_utf8(bytes).ok())
    }
}

pub fn parse_proxy_uri(uri: &str) -> Result<ProfileItem, Box<dyn Error + Send + Sync>> {
    let uri = uri.trim();
    if uri.starts_with("vmess://") {
        return parse_vmess(uri);
    }

    let url = Url::parse(uri)?;
    let scheme = url.scheme();
    let name = url.fragment().map(|f| urlencoding::decode(f).unwrap_or(std::borrow::Cow::Borrowed(f)).into_owned()).unwrap_or_else(|| "Imported Node".to_string());
    
    let host = url.host_str().ok_or("Missing host")?.to_string();
    let port = url.port().unwrap_or(443);

    let detail = match scheme {
        "vless" => {
            let uuid = url.username().to_string();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            
            let network = query.get("type").cloned().unwrap_or_default();
            let host_header = query.get("host").cloned().unwrap_or_default();
            let path = query.get("path").cloned().unwrap_or_default();
            let security = query.get("security").cloned().unwrap_or_default();
            let sni = query.get("sni").cloned().unwrap_or_default();
            let flow = query.get("flow").cloned().unwrap_or_default();

            ProtocolDetail::Vless(VlessDetail {
                uuid,
                encryption: "none".to_string(),
                network,
                host: host_header,
                path,
                tls: security == "tls",
                sni,
                flow,
            })
        }
        "ss" => {
            // ss://[base64(method:password)]@host:port
            // ss://[base64_auth]#[name]
            let auth = url.username();
            let (method, password) = if let Some(decoded) = decode_base64(auth) {
                let parts: Vec<&str> = decoded.splitn(2, ':').collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    ("chacha20-ietf-poly1305".to_string(), decoded)
                }
            } else {
                ("chacha20-ietf-poly1305".to_string(), auth.to_string())
            };

            ProtocolDetail::Shadowsocks(ShadowsocksDetail {
                method,
                password,
            })
        }
        "trojan" => {
            let password = url.password().unwrap_or_else(|| url.username()).to_string();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            let security = query.get("security").cloned().unwrap_or_default();
            let sni = query.get("sni").cloned().unwrap_or_default();

            ProtocolDetail::Trojan(TrojanDetail {
                password,
                network: "tcp".to_string(),
                host: "".to_string(),
                path: "".to_string(),
                tls: security != "none",
                sni,
            })
        }
        "hysteria2" => {
            let auth = url.username().to_string();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            let obfs = query.get("obfs").cloned().unwrap_or_default();
            let obfs_password = query.get("obfs-password").cloned().unwrap_or_default();
            let sni = query.get("sni").cloned().unwrap_or_default();
            let insecure = query.get("insecure").map(|v| v == "1" || v == "true").unwrap_or(false);

            ProtocolDetail::Hysteria2(Hysteria2Detail {
                auth,
                obfs,
                obfs_password,
                sni,
                insecure,
            })
        }
        "tuic" => {
            let uuid = url.username().to_string();
            let password = url.password().unwrap_or_default().to_string();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            let congestion_control = query.get("congestion_control").cloned().unwrap_or_else(|| "cubic".to_string());
            let udp_relay_mode = query.get("udp_relay_mode").cloned().unwrap_or_else(|| "native".to_string());
            let sni = query.get("sni").cloned().unwrap_or_default();
            let insecure = query.get("insecure").map(|v| v == "1" || v == "true").unwrap_or(false);

            ProtocolDetail::Tuic(TuicDetail {
                uuid,
                password,
                congestion_control,
                udp_relay_mode,
                sni,
                insecure,
            })
        }
        "socks" => {
            let username = if url.username().is_empty() { None } else { Some(url.username().to_string()) };
            let password = url.password().map(|p| p.to_string());
            ProtocolDetail::Socks(SocksDetail { username, password })
        }
        "http" => {
            let username = if url.username().is_empty() { None } else { Some(url.username().to_string()) };
            let password = url.password().map(|p| p.to_string());
            ProtocolDetail::Http(HttpDetail { username, password })
        }
        _ => return Err(format!("Unsupported protocol scheme: {}", scheme).into()),
    };

    let detail_json = serde_json::to_string(&detail)?;

    Ok(ProfileItem {
        id: None,
        name,
        address: host,
        port,
        protocol: scheme.to_string(),
        detail: detail_json,
        delay: None,
        is_active: false,
        sub_id: None,
    })
}

fn parse_vmess(uri: &str) -> Result<ProfileItem, Box<dyn Error + Send + Sync>> {
    let content = uri.strip_prefix("vmess://").ok_or("Invalid VMess prefix")?;
    let decoded = decode_base64(content).ok_or("Failed to decode VMess base64")?;
    
    // VMess configuration JSON
    let vmess_json: serde_json::Value = serde_json::from_str(&decoded)?;
    
    let name = vmess_json["ps"].as_str().unwrap_or("VMess Server").to_string();
    let address = vmess_json["add"].as_str().ok_or("Missing add field")?.to_string();
    
    let port = match &vmess_json["port"] {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(443) as u16,
        serde_json::Value::String(s) => s.parse().unwrap_or(443),
        _ => 443
    };

    let uuid = vmess_json["id"].as_str().unwrap_or_default().to_string();
    let alter_id = match &vmess_json["aid"] {
        serde_json::Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        serde_json::Value::String(s) => s.parse().unwrap_or(0),
        _ => 0
    };
    
    let security = vmess_json["scy"].as_str().unwrap_or("auto").to_string();
    let network = vmess_json["net"].as_str().unwrap_or("tcp").to_string();
    let host = vmess_json["host"].as_str().unwrap_or_default().to_string();
    let path = vmess_json["path"].as_str().unwrap_or_default().to_string();
    
    let tls = match &vmess_json["tls"] {
        serde_json::Value::String(s) => s == "tls",
        serde_json::Value::Bool(b) => *b,
        _ => false
    };
    
    let sni = vmess_json["sni"].as_str().unwrap_or_default().to_string();

    let detail = ProtocolDetail::Vmess(VmessDetail {
        uuid,
        alter_id,
        security,
        network,
        host,
        path,
        tls,
        sni,
    });

    let detail_json = serde_json::to_string(&detail)?;

    Ok(ProfileItem {
        id: None,
        name,
        address,
        port,
        protocol: "vmess".to_string(),
        detail: detail_json,
        delay: None,
        is_active: false,
        sub_id: None,
    })
}
