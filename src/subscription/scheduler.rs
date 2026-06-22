use crate::storage::Storage;
use crate::subscription::fetch::fetch_subscription;
use crate::ui::bridge::refresh_profiles_ui;
use slint::ComponentHandle;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;

pub fn start_scheduler(storage: Storage, slint_handle_weak: slint::Weak<crate::AppWindow>) {
    tokio::spawn(async move {
        // Run a check loop every 1 hour
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;

            let subs = match storage.get_subscriptions() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let now = Utc::now();
            let mut updated_any = false;

            for sub in subs {
                let should_update = match &sub.last_updated {
                    None => true,
                    Some(date_str) => {
                        if let Ok(last) = chrono::DateTime::parse_from_rfc3339(date_str) {
                            let diff = now.signed_duration_since(last.with_timezone(&Utc));
                            diff.num_hours() >= sub.update_interval as i64
                        } else {
                            true
                        }
                    }
                };

                if should_update {
                    println!("[2con] Auto-updating subscription: {}", sub.name);
                    match fetch_subscription(&sub.url).await {
                        Ok(profiles) => {
                            if let Some(sub_id) = sub.id {
                                // Delete old profiles
                                let _ = storage.clear_profiles_by_sub_id(sub_id);
                                // Insert new profiles
                                for mut p in profiles {
                                    p.sub_id = Some(sub_id);
                                    let _ = storage.add_profile(&p);
                                }
                                
                                // Update last_updated field
                                let mut updated_sub = sub.clone();
                                updated_sub.last_updated = Some(now.to_rfc3339());
                                let _ = storage.add_subscription(&updated_sub);
                                updated_any = true;
                            }
                        }
                        Err(e) => {
                            eprintln!("[2con Error] Scheduled update failed for sub '{}': {}", sub.name, e);
                        }
                    }
                }
            }

            if updated_any {
                // Refresh profiles view in Slint
                let storage_clone = storage.clone();
                let ui_weak = slint_handle_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        refresh_profiles_ui(&ui, &storage_clone);
                    }
                });
            }
        }
    });
}
