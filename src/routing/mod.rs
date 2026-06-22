use crate::model::RoutingRule;

pub fn compile_routing_rules(preset_name: &str) -> Vec<RoutingRule> {
    match preset_name {
        "Global" => {
            vec![
                RoutingRule {
                    outbound: "proxy".to_string(),
                    domain: None,
                    ip: None,
                    port: Some("1-65535".to_string()),
                    protocol: None,
                }
            ]
        }
        "Bypass LAN & China" | _ => {
            vec![
                // Direct rules for LAN and China
                RoutingRule {
                    outbound: "direct".to_string(),
                    domain: Some(vec![
                        "geosite:private".to_string(),
                        "geosite:cn".to_string(),
                    ]),
                    ip: Some(vec![
                        "geoip:private".to_string(),
                        "geoip:cn".to_string(),
                    ]),
                    port: None,
                    protocol: None,
                },
                // Proxy rule for geosite geolocation-!cn
                RoutingRule {
                    outbound: "proxy".to_string(),
                    domain: Some(vec![
                        "geosite:geolocation-!cn".to_string(),
                    ]),
                    ip: None,
                    port: None,
                    protocol: None,
                }
            ]
        }
    }
}
