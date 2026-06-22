use crate::core::CoreEngine;
use crate::core::config_builder::build_singbox_outbound;
use crate::model::{ProfileItem, AppSettings, RoutingRule};
use serde_json::{json, Value};
use std::error::Error;

pub struct SingboxEngine;

impl CoreEngine for SingboxEngine {
    fn generate_config(
        &self,
        profile: &ProfileItem,
        settings: &AppSettings,
        routing_rules: &[RoutingRule],
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let proxy_outbound = build_singbox_outbound(profile)?;

        // Prepare inbounds
        let mut inbounds = vec![
            json!({
                "type": "socks",
                "tag": "socks-in",
                "listen": "127.0.0.1",
                "listen_port": settings.socks_port,
                "sniff": true
            }),
            json!({
                "type": "http",
                "tag": "http-in",
                "listen": "127.0.0.1",
                "listen_port": settings.http_port,
                "sniff": true
            })
        ];

        // Add TUN if enabled
        if settings.tun_enabled {
            inbounds.push(json!({
                "type": "tun",
                "tag": "tun-in",
                "interface_name": "twocon-tun",
                "inet4_address": "172.19.0.1/30",
                "auto_route": true,
                "strict_route": true,
                "stack": "system",
                "sniff": true
            }));
        }

        // Build route rules
        let mut singbox_rules = vec![
            // Intercept DNS traffic
            json!({
                "port": [53],
                "outbound": "dns-out"
            })
        ];

        for rule in routing_rules {
            let mut singbox_rule = json!({
                "outbound": if rule.outbound == "proxy" { "proxy" } else if rule.outbound == "block" { "block" } else { "direct" }
            });

            if let Some(domains) = &rule.domain {
                singbox_rule["domain"] = json!(domains);
            }
            if let Some(ips) = &rule.ip {
                singbox_rule["ip"] = json!(ips);
            }
            if let Some(ports) = &rule.port {
                // sing-box expects integer array or port range string
                // for simplicity, parse single port or add as is
                singbox_rule["port"] = json!(ports);
            }
            
            singbox_rules.push(singbox_rule);
        }

        let config = json!({
            "log": {
                "level": settings.log_level
            },
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:9090",
                    "secret": ""
                }
            },
            "dns": {
                "servers": [
                    {
                        "tag": "dns-remote",
                        "address": settings.dns_server,
                        "detour": "proxy"
                    },
                    {
                        "tag": "dns-local",
                        "address": "local",
                        "detour": "direct"
                    }
                ],
                "rules": [
                    {
                        "outbound": "direct",
                        "server": "dns-local"
                    }
                ]
            },
            "inbounds": inbounds,
            "outbounds": [
                proxy_outbound,
                {
                    "type": "direct",
                    "tag": "direct"
                },
                {
                    "type": "block",
                    "tag": "block"
                },
                {
                    "type": "dns",
                    "tag": "dns-out"
                }
            ],
            "route": {
                "rules": singbox_rules,
                "auto_detect_interface": true
            }
        });

        Ok(serde_json::to_string_pretty(&config)?)
    }

    fn get_stats_command(&self, _server: &str) -> Option<Vec<String>> {
        // Singbox stats are queried via Clash HTTP controller `/v1/traffic` (127.0.0.1:9090)
        None
    }
}
