use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use crate::storage::get_app_dir;

#[derive(Clone)]
pub struct ProcessManager {
    child: Arc<Mutex<Option<Child>>>,
    reader_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            reader_task: Arc::new(Mutex::new(None)),
        }
    }

    pub fn stop(&self) {
        // Kill existing process
        let mut child_guard = self.child.lock().unwrap();
        if let Some(mut child) = child_guard.take() {
            let _ = child.start_kill();
        }
        
        let mut task_guard = self.reader_task.lock().unwrap();
        if let Some(task) = task_guard.take() {
            task.abort();
        }
    }

    pub fn start<F>(&self, core_type: &str, config_content: &str, log_callback: F) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.stop();

        let app_dir = get_app_dir();
        std::fs::create_dir_all(&app_dir)?;

        // Write configuration content to file
        let config_path = app_dir.join("active_config.json");
        std::fs::write(&config_path, config_content)?;

        // Resolve binary name
        let bin_name = if cfg!(target_os = "windows") {
            format!("{}.exe", core_type)
        } else {
            core_type.to_string()
        };

        // Locate binary (local folder, app_dir, or system path)
        let mut bin_path = PathBuf::from(&bin_name);
        if !bin_path.exists() {
            bin_path = app_dir.join(&bin_name);
        }
        if !bin_path.exists() {
            bin_path = std::env::current_dir()?.join(&bin_name);
        }

        // If still not exists, try finding in system PATH by using it directly
        log_callback(format!("[2con] Attempting to launch core: {}", bin_path.display()));

        let mut cmd = Command::new(&bin_path);
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Adjust parameters for sing-box vs xray
        if core_type == "sing-box" {
            cmd.arg("run").arg("-c").arg(&config_path);
        } else {
            cmd.arg("-c").arg(&config_path);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!(
                    "[2con Error] Failed to start core binary '{}'. Ensure it is placed in the workspace or under '{}/'. Error: {}",
                    bin_name,
                    app_dir.display(),
                    e
                );
                log_callback(err_msg.clone());
                return Err(err_msg.into());
            }
        };

        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

        let child_arc = self.child.clone();
        *child_arc.lock().unwrap() = Some(child);

        // Spawn logging task
        let log_cb = Arc::new(log_callback);
        let cb_out = log_cb.clone();
        let cb_err = log_cb.clone();

        let task = tokio::spawn(async move {
            let mut out_reader = BufReader::new(stdout).lines();
            let mut err_reader = BufReader::new(stderr).lines();

            loop {
                tokio::select! {
                    res = out_reader.next_line() => {
                        match res {
                            Ok(Some(line)) => cb_out(line),
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    res = err_reader.next_line() => {
                        match res {
                            Ok(Some(line)) => cb_err(line),
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }
            cb_out("[2con] Core process stopped reading stream.".to_string());
        });

        *self.reader_task.lock().unwrap() = Some(task);
        Ok(())
    }
}
