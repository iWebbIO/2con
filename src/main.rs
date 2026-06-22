pub mod model;
pub mod storage;
pub mod app_state;
pub mod core;
pub mod subscription;
pub mod routing;
pub mod system_proxy;
pub mod stats;
pub mod ui;

use std::sync::Arc;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Resolve storage directories and SQLite DB
    let app_dir = storage::get_app_dir();
    let db_path = app_dir.join("twocon.db");
    
    println!("[2con] App Directory: {}", app_dir.display());
    println!("[2con] Database Path: {}", db_path.display());

    let storage = storage::Storage::new(db_path)?;

    // 2. Initialize Shared App State
    let app_state = Arc::new(app_state::AppState::new(storage));

    // 3. Create Slint UI handle
    let ui = AppWindow::new()?;

    // 4. Bind UI callbacks to Rust controllers
    ui::bridge::bind_ui_callbacks(&ui, app_state.clone());

    // 5. Spawn background workers
    let slint_weak = ui.as_weak();
    
    // Auto-update scheduler
    subscription::scheduler::start_scheduler(app_state.storage.clone(), slint_weak.clone());
    
    // Stats poller
    stats::start_stats_poller(slint_weak);

    // 6. Block on Slint event loop
    ui.run()?;

    // 7. Graceful cleanup - make sure system proxy is not left on when user closes app
    println!("[2con] Cleaning system proxy settings...");
    let _ = system_proxy::disable_system_proxy();

    Ok(())
}
