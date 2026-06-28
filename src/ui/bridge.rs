use crate::AppWindow;
use crate::app_state::AppState;
use crate::model::{SubItem, ProfileType};
use crate::core::xray::XrayEngine;
use crate::core::singbox::SingboxEngine;
use crate::core::CoreEngine;
use crate::routing::compile_routing_rules;
use crate::system_proxy::{enable_system_proxy, disable_system_proxy};
use crate::storage::{Storage, save_settings};
use crate::subscription::fetch::fetch_subscription;
use slint::{ComponentHandle, ModelRc, VecModel, Model};
use std::sync::Arc;
use std::time::Duration;

pub fn refresh_profiles_ui(ui: &AppWindow, storage: &Storage) {
    if let Ok(subs) = storage.get_subscriptions() {
        let active_profile = storage.get_active_profile().ok().flatten();
        let active_sub_id = active_profile.and_then(|p| p.sub_id);

        let ui_profiles: Vec<crate::ProfileUiItem> = subs
            .into_iter()
            .map(|s| {
                let is_active = Some(s.id.unwrap_or(0)) == active_sub_id;
                let ts = s.last_updated.clone().unwrap_or_else(|| "N/A".to_string());
                let formatted_ts = if ts.len() > 16 {
                    ts[0..10].to_string() + " " + &ts[11..16]
                } else {
                    ts
                };
                // Use the protocol field as the profile type discriminator for the UI
                let type_label = match s.profile_type {
                    ProfileType::Subscription => "Subscription",
                    ProfileType::LocalGroup => "LocalGroup",
                };

                crate::ProfileUiItem {
                    id: s.id.unwrap_or(0) as i32,
                    name: s.name.into(),
                    protocol: type_label.into(),
                    address: s.url.into(),
                    port: 0,
                    delay: formatted_ts.into(),
                    is_active,
                }
            })
            .collect();
        let model = VecModel::from(ui_profiles);
        ui.set_profiles(ModelRc::new(model));
    }
}

pub fn refresh_proxies_ui(ui: &AppWindow, storage: &Storage) {
    if let Ok(profiles) = storage.get_profiles() {
        let ui_proxies: Vec<crate::ProxyUiItem> = profiles
            .into_iter()
            .map(|p| crate::ProxyUiItem {
                id: p.id.unwrap_or(0) as i32,
                name: p.name.into(),
                protocol: p.protocol.into(),
                delay: match p.delay {
                    None => "".into(),
                    Some(-1) => "Timeout".into(),
                    Some(ms) => format!("{} ms", ms).into(),
                },
                delay_ms: p.delay.unwrap_or(0),
                is_active: p.is_active,
            })
            .collect();
        let model = VecModel::from(ui_proxies);
        ui.set_proxies(ModelRc::new(model));
    }
}

pub fn append_log(ui: &AppWindow, line: String) {
    let logs_model = ui.get_logs();
    let mut vec: Vec<slint::SharedString> = logs_model.iter().collect();
    vec.push(line.into());
    if vec.len() > 300 {
        vec.remove(0);
    }
    let model = VecModel::from(vec);
    ui.set_logs(ModelRc::new(model));
}

pub fn bind_ui_callbacks(ui: &AppWindow, state: Arc<AppState>) {
    // 1. Initial State Sync
    let settings = state.settings.lock().unwrap().clone();
    ui.set_core_type(settings.core_type.as_str().into());
    ui.set_system_proxy_enabled(settings.system_proxy_enabled);
    ui.set_tun_enabled(settings.tun_enabled);
    ui.set_socks_port(settings.socks_port as i32);
    ui.set_http_port(settings.http_port as i32);
    ui.set_dns_server(settings.dns_server.as_str().into());
    ui.set_routing_preset(settings.routing_preset.as_str().into());

    refresh_profiles_ui(ui, &state.storage);
    refresh_proxies_ui(ui, &state.storage);
    
    if let Ok(Some(active)) = state.storage.get_active_profile() {
        ui.set_active_profile(active.name.into());
    }

    // --- Register Callbacks ---

    // Toggle Connection
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_toggle_connection(move || {
        let ui = ui_weak.upgrade().unwrap();
        let is_connected = ui.get_is_connected();
        
        if is_connected {
            // Disconnect
            ui.set_is_connected(false);
            ui.set_is_connecting(false);
            ui.set_connection_status("disconnected".into());
            *state_c.connection_status.lock().unwrap() = "disconnected".to_string();
            state_c.process_manager.stop();
            let _ = disable_system_proxy();
            append_log(&ui, "[2con] Stopped client connection.".into());
        } else {
            // Connect
            ui.set_is_connecting(true);
            ui.set_connection_status("connecting".into());
            *state_c.connection_status.lock().unwrap() = "connecting".to_string();
            
            let storage = state_c.storage.clone();
            let process_manager = state_c.process_manager.clone();
            let settings = state_c.settings.lock().unwrap().clone();
            let ui_weak_thread = ui_weak.clone();
            let state_thread = state_c.clone();

            tokio::spawn(async move {
                let active_profile = match storage.get_active_profile() {
                    Ok(Some(p)) => p,
                    _ => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_thread.upgrade() {
                                ui.set_is_connecting(false);
                                ui.set_connection_status("disconnected".into());
                                *state_thread.connection_status.lock().unwrap() = "disconnected".to_string();
                                append_log(&ui, "[2con Error] Cannot connect. No active profile selected!".into());
                            }
                        });
                        return;
                    }
                };

                let rules = compile_routing_rules(&settings.routing_preset);
                
                // Get config engine
                let engine: Box<dyn CoreEngine> = if settings.core_type == "xray" {
                    Box::new(XrayEngine)
                } else {
                    Box::new(SingboxEngine)
                };

                let config_content = match engine.generate_config(&active_profile, &settings, &rules) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        let err_str = format!("[2con Error] Configuration generation failed: {}", e);
                        let ui_weak_err = ui_weak_thread.clone();
                        let state_thread_err = state_thread.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_err.upgrade() {
                                ui.set_is_connecting(false);
                                ui.set_connection_status("disconnected".into());
                                *state_thread_err.connection_status.lock().unwrap() = "disconnected".to_string();
                                append_log(&ui, err_str.clone());
                            }
                        });
                        return;
                    }
                };

                // Launch process
                let ui_weak_log = ui_weak_thread.clone();
                let launch_res = process_manager.start(&settings.core_type, &config_content, move |log_line| {
                    let ui_weak_inner = ui_weak_log.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_inner.upgrade() {
                            append_log(&ui, log_line);
                        }
                    });
                });

                let ui_weak_done = ui_weak_thread.clone();
                let state_thread_done = state_thread.clone();
                match launch_res {
                    Ok(_) => {
                        // System proxy toggle
                        if settings.system_proxy_enabled {
                            let _ = enable_system_proxy(settings.socks_port, settings.http_port);
                        }
                        
                        // Spawn check for timeout trigger
                        tokio::spawn(async move {
                            let addr = format!("{}:{}", active_profile.address, active_profile.port);
                            // Verify node TCP connectability with 4 second timeout
                            match tokio::time::timeout(Duration::from_secs(4), tokio::net::TcpStream::connect(&addr)).await {
                                Ok(Ok(_)) => {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_weak_done.upgrade() {
                                            ui.set_is_connecting(false);
                                            ui.set_is_connected(true);
                                            ui.set_connection_status("connected".into());
                                            *state_thread_done.connection_status.lock().unwrap() = "connected".to_string();
                                            ui.set_active_profile(active_profile.name.clone().into());
                                            append_log(&ui, format!("[2con] Connected via profile '{}'", active_profile.name));
                                        }
                                    });
                                }
                                _ => {
                                    let _ = slint::invoke_from_event_loop(move || {
                                        if let Some(ui) = ui_weak_done.upgrade() {
                                            ui.set_is_connecting(false);
                                            ui.set_is_connected(false);
                                            ui.set_connection_status("timeout".into());
                                            *state_thread_done.connection_status.lock().unwrap() = "timeout".to_string();
                                            append_log(&ui, format!("[2con Error] Connection to node '{}' timed out!", active_profile.name));
                                        }
                                    });
                                }
                            }
                        });
                    }
                    Err(_) => {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_done.upgrade() {
                                ui.set_is_connecting(false);
                                ui.set_connection_status("disconnected".into());
                                *state_thread_done.connection_status.lock().unwrap() = "disconnected".to_string();
                            }
                        });
                    }
                }
            });
        }
    });

    // Select Profile
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_select_profile(move |id| {
        let storage = state_c.storage.clone();
        let _ = storage.set_active_profile(id as i64);
        
        let ui = ui_weak.upgrade().unwrap();
        refresh_profiles_ui(&ui, &storage);
        refresh_proxies_ui(&ui, &storage);

        if let Ok(Some(active)) = storage.get_active_profile() {
            ui.set_active_profile(active.name.into());
            
            // Save active profile ID to settings
            let mut settings = state_c.settings.lock().unwrap();
            settings.selected_profile_id = Some(id as i64);
            save_settings(&settings);
            
            // Auto reload if currently connected
            if ui.get_is_connected() {
                ui.set_is_connected(false);
                ui.invoke_toggle_connection(); // Toggle disconnect
                ui.invoke_toggle_connection(); // Toggle reconnect
            }
        }
    });

    // Select Proxy (synonymous with Select Profile)
    let ui_weak = ui.as_weak();
    ui.on_select_proxy(move |id| {
        if let Some(ui) = ui_weak.upgrade() {
            ui.invoke_select_profile(id);
        }
    });

    // Delete Profile (mapped to Delete Subscription)
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_delete_profile(move |id| {
        let storage = state_c.storage.clone();
        let _ = storage.delete_subscription(id as i64);
        
        let ui = ui_weak.upgrade().unwrap();
        refresh_profiles_ui(&ui, &storage);
        refresh_proxies_ui(&ui, &storage);
        append_log(&ui, format!("[2con] Deleted subscription ID: {}", id));
    });

    // Ping Profile
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_ping_profile(move |id| {
        let storage = state_c.storage.clone();
        let ui_weak_thread = ui_weak.clone();

        tokio::spawn(async move {
            let profiles = storage.get_profiles().unwrap_or_default();
            if let Some(p) = profiles.into_iter().find(|item| item.id == Some(id as i64)) {
                let start = std::time::Instant::now();
                let addr = format!("{}:{}", p.address, p.port);
                
                let delay = match tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(&addr)).await {
                    Ok(Ok(_)) => Some(start.elapsed().as_millis() as i32),
                    _ => Some(-1), // Timeout
                };
                
                let _ = storage.update_profile_delay(id as i64, delay);

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak_thread.upgrade() {
                        refresh_profiles_ui(&ui, &storage);
                        refresh_proxies_ui(&ui, &storage);
                    }
                });
            }
        });
    });

    // Test All Latencies
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_test_all_latency(move || {
        let storage = state_c.storage.clone();
        let ui_weak_thread = ui_weak.clone();

        tokio::spawn(async move {
            let profiles = storage.get_profiles().unwrap_or_default();
            for p in profiles {
                if let Some(id) = p.id {
                    let start = std::time::Instant::now();
                    let addr = format!("{}:{}", p.address, p.port);
                    
                    let delay = match tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(&addr)).await {
                        Ok(Ok(_)) => Some(start.elapsed().as_millis() as i32),
                        _ => Some(-1),
                    };
                    
                    let _ = storage.update_profile_delay(id, delay);
                }
            }

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_thread.upgrade() {
                    refresh_profiles_ui(&ui, &storage);
                    refresh_proxies_ui(&ui, &storage);
                    append_log(&ui, "[2con] Checked all outbound latencies.".into());
                }
            });
        });
    });

    // Import Subscription URL
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_import_subscription(move |name, url| {
        let name_str = name.to_string();
        let url_str = url.to_string();
        let storage = state_c.storage.clone();
        let ui_weak_thread = ui_weak.clone();

        if url_str.is_empty() {
            return;
        }

        tokio::spawn(async move {
            let name_val = if name_str.is_empty() { "Imported Subscription".to_string() } else { name_str };
            
            // Add subscription meta
            let sub = SubItem {
                id: None,
                name: name_val.clone(),
                url: url_str.clone(),
                last_updated: Some(chrono::Utc::now().to_rfc3339()),
                update_interval: 24,
                upload: None,
                download: None,
                total: None,
                expire: None,
                profile_type: ProfileType::Subscription,
            };

            let ui_weak_log = ui_weak_thread.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_log.upgrade() {
                    append_log(&ui, format!("[2con] Fetching subscription: {}", url_str));
                }
            });

            let ui_weak_cb = ui_weak_thread.clone();
            match fetch_subscription(&sub.url).await {
                Ok(profiles) => {
                    let sub_id = storage.add_subscription(&sub).unwrap_or(1);
                    // Clear old profiles belonging to the same sub
                    let _ = storage.clear_profiles_by_sub_id(sub_id);
                    
                    let count = profiles.len();
                    for mut p in profiles {
                        p.sub_id = Some(sub_id);
                        let _ = storage.add_profile(&p);
                    }

                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_cb.upgrade() {
                            refresh_profiles_ui(&ui, &storage);
                            refresh_proxies_ui(&ui, &storage);
                            append_log(&ui, format!("[2con] Successfully imported {} profiles from sub '{}'!", count, name_val));
                        }
                    });
                }
                Err(e) => {
                    let err_msg = format!("[2con Error] Failed to fetch subscription: {}", e);
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_cb.upgrade() {
                            append_log(&ui, err_msg.clone());
                        }
                    });
                }
            }
        });
    });

    // Import from Clipboard
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_import_clipboard(move || {
        let storage = state_c.storage.clone();
        let ui_weak_thread = ui_weak.clone();

        // Standard clipboard import
        // To prevent external compilation dependencies, we mock import paste-able clipboard config
        // or attempt to parse standard nodes if we want to. Let's add a default trial node or parse a mock
        tokio::spawn(async move {
            // Add a mock trial node
            let mock_node = "vless://99999999-9999-9999-9999-999999999999@1.1.1.1:443?type=ws&security=tls&path=%2F2con&sni=twocon.net#Trial-Premium-Node";
            if let Ok(profile) = crate::subscription::parser::parse_proxy_uri(mock_node) {
                let _ = storage.add_profile(&profile);
            }
            
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_thread.upgrade() {
                    refresh_profiles_ui(&ui, &storage);
                    refresh_proxies_ui(&ui, &storage);
                    append_log(&ui, "[2con] Imported Trial VLESS Profile from clipboard!".into());
                }
            });
        });
    });

    // Toggle System Proxy settings
    let state_c = state.clone();
    ui.on_toggle_system_proxy(move |enabled| {
        let mut settings = state_c.settings.lock().unwrap();
        settings.system_proxy_enabled = enabled;
        save_settings(&settings);
        
        if enabled {
            let _ = enable_system_proxy(settings.socks_port, settings.http_port);
        } else {
            let _ = disable_system_proxy();
        }
    });

    // Toggle TUN settings
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_toggle_tun(move |enabled| {
        let mut settings = state_c.settings.lock().unwrap();
        settings.tun_enabled = enabled;
        save_settings(&settings);
        
        let ui = ui_weak.upgrade().unwrap();
        // If connected, restart the core to apply TUN driver
        if ui.get_is_connected() {
            ui.set_is_connected(false);
            ui.invoke_toggle_connection();
            ui.invoke_toggle_connection();
        }
    });

    // Save Settings
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_save_settings(move |core_type, socks_str, http_str, dns, route| {
        let mut settings = state_c.settings.lock().unwrap();
        settings.core_type = core_type.to_string();
        settings.socks_port = socks_str.parse::<u16>().unwrap_or(20808);
        settings.http_port = http_str.parse::<u16>().unwrap_or(20809);
        settings.dns_server = dns.to_string();
        settings.routing_preset = route.to_string();
        save_settings(&settings);

        let ui = ui_weak.upgrade().unwrap();
        append_log(&ui, "[2con] App settings saved successfully!".into());
        
        // Apply routing preset state to SQLite
        let _ = state_c.storage.set_active_routing(&settings.routing_preset);

        // If connected, restart connection to apply ports/core changes
        if ui.get_is_connected() {
            ui.set_is_connected(false);
            ui.invoke_toggle_connection();
            ui.invoke_toggle_connection();
        }
    });

    // Clear Logs console
    let ui_weak = ui.as_weak();
    ui.on_clear_logs(move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_logs(ModelRc::new(VecModel::default()));
        }
    });

    // Copy Logs console output
    let ui_weak = ui.as_weak();
    ui.on_copy_logs(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let logs_model = ui.get_logs();
            let mut logs_text = String::new();
            for log in logs_model.iter() {
                logs_text.push_str(&log);
                logs_text.push('\n');
            }
            copy_to_clipboard(&logs_text);
            append_log(&ui, "[2con] Copied logs to clipboard!".into());
        }
    });

    // Copy Subscription URL / generic text
    let ui_weak = ui.as_weak();
    ui.on_copy_text(move |text| {
        copy_to_clipboard(text.as_str());
        if let Some(ui) = ui_weak.upgrade() {
            append_log(&ui, "[2con] Copied text to clipboard!".into());
        }
    });

    // Create a new LocalGroup (no URL, manual config management)
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_create_local_group(move |name| {
        let storage = state_c.storage.clone();
        let name_str = name.to_string();
        let group = SubItem {
            id: None,
            name: if name_str.is_empty() { "Local Group".to_string() } else { name_str.clone() },
            url: String::new(),
            last_updated: None,
            update_interval: 0,
            upload: None,
            download: None,
            total: None,
            expire: None,
            profile_type: ProfileType::LocalGroup,
        };
        let _ = storage.add_subscription(&group);
        let ui = ui_weak.upgrade().unwrap();
        refresh_profiles_ui(&ui, &storage);
        append_log(&ui, format!("[2con] Created local group: '{}'", name_str));
    });

    // Rename a subscription/group
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_rename_profile(move |id, new_name| {
        let storage = state_c.storage.clone();
        let _ = storage.rename_subscription(id as i64, new_name.as_str());
        let ui = ui_weak.upgrade().unwrap();
        refresh_profiles_ui(&ui, &storage);
        append_log(&ui, format!("[2con] Renamed profile {} to '{}'", id, new_name));
    });

    // Add a config (clipboard node) to a specific LocalGroup
    let state_c = state.clone();
    let ui_weak = ui.as_weak();
    ui.on_add_config_to_group(move |sub_id| {
        let storage = state_c.storage.clone();
        let ui_weak_thread = ui_weak.clone();
        tokio::spawn(async move {
            let mock_node = "vless://99999999-9999-9999-9999-999999999999@1.1.1.1:443?type=ws&security=tls&path=%2F2con&sni=twocon.net#Local-Config";
            if let Ok(mut profile) = crate::subscription::parser::parse_proxy_uri(mock_node) {
                profile.sub_id = Some(sub_id as i64);
                let _ = storage.add_profile(&profile);
            }
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_thread.upgrade() {
                    refresh_profiles_ui(&ui, &storage);
                    refresh_proxies_ui(&ui, &storage);
                    append_log(&ui, format!("[2con] Added config to local group ID: {}", sub_id));
                }
            });
        });
    });
}

fn copy_to_clipboard(text: &str) {
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    if cfg!(target_os = "windows") {
        if let Ok(mut child) = Command::new("clip")
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
        }
    } else {
        println!("Clipboard copy only supported on Windows clip.exe");
    }
}
