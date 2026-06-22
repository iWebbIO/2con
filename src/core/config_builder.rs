use crate::model::{ProfileItem, ProtocolDetail, AppSettings, RoutingRule};
use serde_json::{json, Value};
use std::error::Error;

pub fn build_singbox_outbound(profile: &ProfileItem) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let detail: ProtocolDetail = serde_json::from_str(&profile.detail)?;
    
    let mut outbound = json!({
        "tag": "proxy",
        "server": profile.address,
        "server_port": profile.port
    });

    match detail {
        ProtocolDetail::Vmess(d) => {
            outbound["type"] = json!("vmess");
            outbound["uuid"] = json!(d.uuid);
            outbound["security"] = json!(d.security);
            if !d.network.is_empty() {
                outbound["transport"] = json!({
                    "type": d.network,
                    "path": d.path,
                    "headers": { "Host": d.host }
                });
            }
            if d.tls {
                outbound["tls"] = json!({
                    "enabled": true,
                    "server_name": d.sni
                });
            }
        }
        ProtocolDetail::Vless(d) => {
            outbound["type"] = json!("vless");
            outbound["uuid"] = json!(d.uuid);
            if !d.flow.is_empty() {
                outbound["flow"] = json!(d.flow);
            }
            if d.tls {
                outbound["tls"] = json!({
                    "enabled": true,
                    "server_name": d.sni
                });
            }
            if !d.network.is_empty() {
                outbound["transport"] = json!({
                    "type": d.network,
                    "path": d.path,
                    "headers": { "Host": d.host }
                });
            }
        }
        ProtocolDetail::Shadowsocks(d) => {
            outbound["type"] = json!("shadowsocks");
            outbound["method"] = json!(d.method);
            outbound["password"] = json!(d.password);
        }
        ProtocolDetail::Trojan(d) => {
            outbound["type"] = json!("trojan");
            outbound["password"] = json!(d.password);
            if d.tls {
                outbound["tls"] = json!({
                    "enabled": true,
                    "server_name": d.sni
                });
            }
        }
        ProtocolDetail::Hysteria2(d) => {
            outbound["type"] = json!("hysteria2");
            outbound["auth"] = json!(d.auth);
            if !d.obfs.is_empty() {
                outbound["obfs"] = json!({
                    "type": d.obfs,
                    "password": d.obfs_password
                });
            }
            if !d.sni.is_empty() {
                outbound["tls"] = json!({
                    "enabled": true,
                    "server_name": d.sni,
                    "insecure": d.insecure
                });
            }
        }
        ProtocolDetail::Tuic(d) => {
            outbound["type"] = json!("tuic");
            outbound["uuid"] = json!(d.uuid);
            outbound["password"] = json!(d.password);
            outbound["congestion_control"] = json!(d.congestion_control);
            outbound["udp_relay_mode"] = json!(d.udp_relay_mode);
            outbound["tls"] = json!({
                "enabled": true,
                "server_name": d.sni,
                "insecure": d.insecure
            });
        }
        ProtocolDetail::Wireguard(d) => {
            outbound["type"] = json!("wireguard");
            outbound["private_key"] = json!(d.private_key);
            outbound["peer_public_key"] = json!(d.public_key);
            outbound["local_address"] = json!(d.ip_addresses);
            outbound["mtu"] = json!(d.mtu);
            if !d.reserved.is_empty() {
                outbound["reserved"] = json!(d.reserved);
            }
        }
        ProtocolDetail::Socks(d) => {
            outbound["type"] = json!("socks");
            if let Some(user) = d.username {
                outbound["username"] = json!(user);
            }
            if let Some(pass) = d.password {
                outbound["password"] = json!(pass);
            }
        }
        ProtocolDetail::Http(d) => {
            outbound["type"] = json!("http");
            if let Some(user) = d.username {
                outbound["username"] = json!(user);
            }
            if let Some(pass) = d.password {
                outbound["password"] = json!(pass);
            }
        }
    }

    Ok(outbound)
}

pub fn build_xray_outbound(profile: &ProfileItem) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let detail: ProtocolDetail = serde_json::from_str(&profile.detail)?;
    
    let mut outbound = json!({
        "tag": "proxy"
    });

    match detail {
        ProtocolDetail::Vmess(d) => {
            outbound["protocol"] = json!("vmess");
            outbound["settings"] = json!({
                "vnext": [{
                    "address": profile.address,
                    "port": profile.port,
                    "users": [{
                        "id": d.uuid,
                        "alterId": d.alter_id,
                        "security": d.security
                    }]
                }]
            });
            
            let mut stream_settings = json!({});
            if !d.network.is_empty() {
                stream_settings["network"] = json!(d.network);
                if d.network == "ws" {
                    stream_settings["wsSettings"] = json!({
                        "path": d.path,
                        "headers": { "Host": d.host }
                    });
                } else if d.network == "grpc" {
                    stream_settings["grpcSettings"] = json!({
                        "serviceName": d.path
                    });
                }
            }
            if d.tls {
                stream_settings["security"] = json!("tls");
                stream_settings["tlsSettings"] = json!({
                    "serverName": d.sni
                });
            }
            if !stream_settings.as_object().unwrap().is_empty() {
                outbound["streamSettings"] = stream_settings;
            }
        }
        ProtocolDetail::Vless(d) => {
            outbound["protocol"] = json!("vless");
            outbound["settings"] = json!({
                "vnext": [{
                    "address": profile.address,
                    "port": profile.port,
                    "users": [{
                        "id": d.uuid,
                        "encryption": d.encryption,
                        "flow": if d.flow.is_empty() { Value::Null } else { json!(d.flow) }
                    }]
                }]
            });

            let mut stream_settings = json!({});
            if !d.network.is_empty() {
                stream_settings["network"] = json!(d.network);
            }
            if d.tls {
                stream_settings["security"] = json!("tls");
                stream_settings["tlsSettings"] = json!({
                    "serverName": d.sni
                });
            }
            if !stream_settings.as_object().unwrap().is_empty() {
                outbound["streamSettings"] = stream_settings;
            }
        }
        ProtocolDetail::Shadowsocks(d) => {
            outbound["protocol"] = json!("shadowsocks");
            outbound["settings"] = json!({
                "servers": [{
                    "address": profile.address,
                    "port": profile.port,
                    "method": d.method,
                    "password": d.password
                }]
            });
        }
        ProtocolDetail::Trojan(d) => {
            outbound["protocol"] = json!("trojan");
            outbound["settings"] = json!({
                "servers": [{
                    "address": profile.address,
                    "port": profile.port,
                    "password": d.password
                }]
            });
            if d.tls {
                outbound["streamSettings"] = json!({
                    "security": "tls",
                    "tlsSettings": {
                        "serverName": d.sni
                    }
                });
            }
        }
        // Xray doesn't natively support TUIC, Hysteria2 without custom forks or external binaries.
        // If these are passed, generate them as a custom/socks chain or warning fallback.
        _ => {
            // Fallback: SOCKS client inbound pointing to proxy address
            outbound["protocol"] = json!("socks");
            outbound["settings"] = json!({
                "servers": [{
                    "address": profile.address,
                    "port": profile.port
                }]
            });
        }
    }

    Ok(outbound)
}
