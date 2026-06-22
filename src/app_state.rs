use crate::storage::Storage;
use crate::core::process::ProcessManager;
use crate::model::AppSettings;
use std::sync::Mutex;

pub struct AppState {
    pub storage: Storage,
    pub process_manager: ProcessManager,
    pub settings: Mutex<AppSettings>,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        let settings = crate::storage::load_settings();
        Self {
            storage,
            process_manager: ProcessManager::new(),
            settings: Mutex::new(settings),
        }
    }
}
