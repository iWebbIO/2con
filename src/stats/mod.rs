use crate::storage::get_app_dir;
use slint::ComponentHandle;
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde_json::Value;

#[derive(Default)]
struct SpeedCalculator {
    last_up: u64,
    last_down: u64,
    total_up: u64,
    total_down: u64,
}

pub fn start_stats_poller(slint_handle_weak: slint::Weak<crate::AppWindow>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        let speed_calc = Arc::new(Mutex::new(SpeedCalculator::default()));

        loop {
            interval.tick().await;

            let ui_weak = slint_handle_weak.clone();
            let (is_connected, core_type) = {
                let ui_upgrade = match ui_weak.upgrade() {
                    Some(ui) => ui,
                    None => break, // UI closed
                };
                (ui_upgrade.get_is_connected(), ui_upgrade.get_core_type().to_string())
            };

            if !is_connected {
                // Clear UI values when disconnected
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_download_speed("0 KB/s".into());
                        ui.set_upload_speed("0 KB/s".into());
                    }
                });
                
                // Reset calculations
                let mut calc = speed_calc.lock().unwrap();
                calc.last_up = 0;
                calc.last_down = 0;
                continue;
            }

            let mut up_bytes_diff = 0;
            let mut down_bytes_diff = 0;
            let mut total_up_accum = 0;
            let mut total_down_accum = 0;

            if core_type == "sing-box" {
                // Query clash controller (Sing-box traffic stats)
                if let Ok(res) = reqwest::get("http://127.0.0.1:9090/v1/traffic").await {
                    // Note: /v1/traffic returns chunked JSON. Let's just read a chunk
                    if let Ok(bytes) = res.bytes().await {
                        if let Ok(val) = serde_json::from_slice::<Value>(&bytes) {
                            up_bytes_diff = val["up"].as_u64().unwrap_or(0);
                            down_bytes_diff = val["down"].as_u64().unwrap_or(0);
                            
                            let mut calc = speed_calc.lock().unwrap();
                            calc.total_up += up_bytes_diff;
                            calc.total_down += down_bytes_diff;
                            total_up_accum = calc.total_up;
                            total_down_accum = calc.total_down;
                        }
                    }
                }
            } else {
                // Query Xray gRPC via API command
                let app_dir = get_app_dir();
                let bin_name = if cfg!(target_os = "windows") { "xray.exe" } else { "xray" };
                let mut bin_path = app_dir.join(bin_name);
                if !bin_path.exists() {
                    bin_path = std::env::current_dir().unwrap_or_default().join(bin_name);
                }

                // If xray is in paths
                let mut cmd = StdCommand::new(if bin_path.exists() { bin_path.to_str().unwrap() } else { bin_name });
                cmd.args(&["api", "stats", "--server=127.0.0.1:10085"]);

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        if let Ok(val) = serde_json::from_slice::<Value>(&output.stdout) {
                            let mut current_up = 0;
                            let mut current_down = 0;

                            if let Some(stats) = val["stat"].as_array() {
                                for s in stats {
                                    let name = s["name"].as_str().unwrap_or("");
                                    let value = s["value"].as_str().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
                                    if name.contains("uplink") {
                                        current_up += value;
                                    } else if name.contains("downlink") {
                                        current_down += value;
                                    }
                                }
                            }

                            let mut calc = speed_calc.lock().unwrap();
                            if calc.last_up > 0 && current_up >= calc.last_up {
                                up_bytes_diff = (current_up - calc.last_up) / 2; // Divide by 2 seconds interval
                            }
                            if calc.last_down > 0 && current_down >= calc.last_down {
                                down_bytes_diff = (current_down - calc.last_down) / 2;
                            }
                            calc.last_up = current_up;
                            calc.last_down = current_down;
                            calc.total_up = current_up;
                            calc.total_down = current_down;
                            total_up_accum = calc.total_up;
                            total_down_accum = calc.total_down;
                        }
                    }
                }
            }

            // Format strings
            let speed_up_str = format_speed(up_bytes_diff as f64);
            let speed_down_str = format_speed(down_bytes_diff as f64);
            let total_up_str = format_bytes(total_up_accum);
            let total_down_str = format_bytes(total_down_accum);

            // Update UI
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_upload_speed(speed_up_str.into());
                    ui.set_download_speed(speed_down_str.into());
                    ui.set_total_upload(total_up_str.into());
                    ui.set_total_download(total_down_str.into());
                }
            });
        }
    });
}

fn format_speed(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.2} MB/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{} B/s", bytes_per_sec)
    }
}

fn format_bytes(bytes: u64) -> String {
    let bytes_f = bytes as f64;
    if bytes_f >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GB", bytes_f / (1024.0 * 1024.0 * 1024.0))
    } else if bytes_f >= 1024.0 * 1024.0 {
        format!("{:.1} MB", bytes_f / (1024.0 * 1024.0))
    } else if bytes_f >= 1024.0 {
        format!("{:.1} KB", bytes_f / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
