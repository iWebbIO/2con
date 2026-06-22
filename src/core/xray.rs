use crate::core::CoreEngine;
use crate::core::config_builder::build_xray_outbound;
use crate::model::{ProfileItem, AppSettings, RoutingRule};
use serde_json::{json, Value};
use std::error::Error;

pub struct XrayEngine;

impl CoreEngine for XrayEngine {
    fn generate_config(
        &self,
        profile: &ProfileItem,
        settings: &AppSettings,
        routing_rules: &[RoutingRule],
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let proxy_outbound = build_xray_outbound(profile)?;

        // Build xray routing rules
        let mut xray_rules = Vec::new();
        
        // Intercept API traffic rule
        xray_rules.push(json!({
            "type": "field",
            "inboundTag": ["api"],
            "outboundTag": "api"
        }));

        for rule in routing_rules {
            let mut xray_rule = json!({
                "type": "field",
                "outboundTag": if rule.outbound == "proxy" { "proxy" } else if rule.outbound == "block" { "block" } else { "direct" }
            });
            
            if let Some(domains) = &rule.domain {
                xray_rule["domain"] = json!(domains);
            }
            if let Some(ips) = &rule.ip {
                xray_rule["ip"] = json!(ips);
            }
            if let Some(ports) = &rule.port {
                xray_rule["port"] = json!(ports);
            }
            if let Some(protocols) = &rule.protocol {
                xray_rule["protocol"] = json!(protocols);
            }
            
            xray_rules.push(xray_rule);
        }

        let config = json!({
            "log": {
                "loglevel": settings.log_level
            },
            "api": {
                "services": [
                    "HandlerService",
                    "StatsService"
                ],
                "tag": "api"
            },
            "stats": {},
            "policy": {
                "system": {
                    "statsInboundDownlink": true,
                    "statsInboundUplink": true,
                    "statsOutboundDownlink": true,
                    "statsOutboundUplink": true
                }
            },
            "inbounds": [
                {
                    "tag": "socks-in",
                    "port": settings.socks_port,
                    "listen": "127.0.0.1",
                    "protocol": "socks",
                    "settings": {
                        "auth": "noauth",
                        "udp": true
                    }
                },
                {
                    "tag": "http-in",
                    "port": settings.http_port,
                    "listen": "127.0.0.1",
                    "protocol": "http",
                    "settings": {}
                },
                {
                    "listen": "127.0.0.1",
                    "port": 10085,
                    "protocol": "dokodemo-door",
                    "settings": {
                        "address": "127.0.0.1"
                    },
                    "tag": "api"
                }
            ],
            "outbounds": [
                proxy_outbound,
                {
                    "protocol": "freedom",
                    "tag": "direct",
                    "settings": {}
                },
                {
                    "protocol": "blackhole",
                    "tag": "block",
                    "settings": {}
                }
            ],
            "routing": {
                "domainStrategy": "IPIfNonMatch",
                "rules": xray_rules
            },
            "dns": {
                "servers": [
                    settings.dns_server,
                    "localhost"
                ]
            }
        });

        Ok(serde_json::to_string_pretty(&config)?)
    }

    fn get_stats_command(&self, server: &str) -> Option<Vec<String>> {
        // xray api stats --server=127.0.0.1:10085
        Some(vec![
            "api".to_string(),
            "stats".to_string(),
            format!("--server={}", server),
        ])
    }
}
