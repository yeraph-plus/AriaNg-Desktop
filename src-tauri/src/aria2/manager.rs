use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{error, info, warn};
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

use crate::aria2::config::Aria2Config;
use crate::aria2::port::find_available_port;
use crate::constants::{
    ARIA2_SHUTDOWN_TIMEOUT_SECS, CRASH_WINDOW_SECS, MAX_CRASH_RETRIES, SIDECAR_NAME,
};

pub struct Aria2Manager {
    app_handle: AppHandle,
    config: Arc<Mutex<Aria2Config>>,
    child_handle: Arc<Mutex<Option<CommandChild>>>,
    shutdown_flag: Arc<AtomicBool>,
    app_data_dir: std::path::PathBuf,
    /// Tracks the actual RPC port in use (may differ from config if port was busy)
    actual_port: Arc<Mutex<u16>>,
    /// Tracks the aria2c process PID for fallback kill
    process_pid: Arc<Mutex<Option<u32>>>,
}

impl Aria2Manager {
    pub fn new(
        app_handle: AppHandle,
        config: Aria2Config,
        app_data_dir: std::path::PathBuf,
    ) -> Self {
        let port = config.rpc_port;
        Self {
            app_handle,
            config: Arc::new(Mutex::new(config)),
            child_handle: Arc::new(Mutex::new(None)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            app_data_dir,
            actual_port: Arc::new(Mutex::new(port)),
            process_pid: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the aria2c sidecar process.
    pub fn start(&self) -> Result<(), String> {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        let config = self.config.lock().map_err(|e| e.to_string())?.clone();

        // Find available port
        let port = find_available_port(config.rpc_port)?;
        if port != config.rpc_port {
            info!(
                "Port {} is busy, using port {} instead",
                config.rpc_port, port
            );
        }

        // Update actual port
        if let Ok(mut actual) = self.actual_port.lock() {
            *actual = port;
        }

        // Update config with actual port
        if let Ok(mut cfg) = self.config.lock() {
            cfg.rpc_port = port;
        }

        // Ensure session file exists
        let session_path = Aria2Config::ensure_session_file(&self.app_data_dir)?;

        // Build args with actual port
        let mut args_config = config.clone();
        args_config.rpc_port = port;
        let args = args_config.to_aria2_args(&session_path);

        info!("Starting aria2c on port {} with sidecar", port);

        // Spawn sidecar
        let shell = self.app_handle.shell();
        let cmd = shell
            .sidecar(SIDECAR_NAME)
            .map_err(|e| format!("Failed to create sidecar command: {}", e))?
            .args(&args);

        let (mut rx, child) = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn aria2c: {}", e))?;

        // Store PID for fallback kill
        let pid = child.pid();
        if let Ok(mut pid_lock) = self.process_pid.lock() {
            *pid_lock = Some(pid);
        }

        // Store child handle
        if let Ok(mut handle) = self.child_handle.lock() {
            *handle = Some(child);
        }

        info!("aria2c started successfully on port {}, pid={}", port, pid);

        // Spawn monitor task
        let shutdown_flag = self.shutdown_flag.clone();
        let child_handle = self.child_handle.clone();
        let config_clone = self.config.clone();
        let app_handle = self.app_handle.clone();
        let app_data_dir = self.app_data_dir.clone();
        let actual_port = self.actual_port.clone();
        let process_pid = self.process_pid.clone();

        tauri::async_runtime::spawn(async move {
            let mut crash_times: Vec<Instant> = Vec::new();

            // Read events from the sidecar
            while let Some(event) = rx.recv().await {
                use tauri_plugin_shell::process::CommandEvent;
                match event {
                    CommandEvent::Stdout(line) => {
                        info!("[aria2c stdout] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        warn!("[aria2c stderr] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Terminated(payload) => {
                        let code = payload.code.unwrap_or(-1);
                        let signal = payload.signal;

                        if shutdown_flag.load(Ordering::SeqCst) {
                            info!("aria2c terminated (expected shutdown), code={}", code);
                            break;
                        }

                        warn!(
                            "aria2c terminated unexpectedly, code={}, signal={:?}",
                            code, signal
                        );

                        // Clear old child handle and PID
                        if let Ok(mut handle) = child_handle.lock() {
                            *handle = None;
                        }
                        if let Ok(mut pid_lock) = process_pid.lock() {
                            *pid_lock = None;
                        }

                        // Check crash frequency (circuit breaker)
                        let now = Instant::now();
                        crash_times
                            .retain(|t| now.duration_since(*t).as_secs() < CRASH_WINDOW_SECS);
                        crash_times.push(now);

                        if crash_times.len() as u32 >= MAX_CRASH_RETRIES {
                            error!(
                                "aria2c crashed {} times within {}s, stopping restart attempts",
                                MAX_CRASH_RETRIES, CRASH_WINDOW_SECS
                            );
                            break;
                        }

                        // Exponential backoff: 2, 4, 8, 16, 30 seconds
                        let retry_count = crash_times.len() as u32;
                        let delay_secs = std::cmp::min(2u64.pow(retry_count), 30);
                        info!("Restarting aria2c in {} seconds...", delay_secs);
                        tokio::time::sleep(Duration::from_secs(delay_secs)).await;

                        if shutdown_flag.load(Ordering::SeqCst) {
                            break;
                        }

                        // Restart aria2c
                        let config = match config_clone.lock() {
                            Ok(c) => c.clone(),
                            Err(_) => break,
                        };

                        let port = match find_available_port(config.rpc_port) {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Failed to find available port for restart: {}", e);
                                break;
                            }
                        };

                        if let Ok(mut actual) = actual_port.lock() {
                            *actual = port;
                        }

                        let session_path =
                            match Aria2Config::ensure_session_file(&app_data_dir) {
                                Ok(p) => p,
                                Err(e) => {
                                    error!("Failed to ensure session file: {}", e);
                                    break;
                                }
                            };

                        let mut restart_config = config.clone();
                        restart_config.rpc_port = port;
                        let args = restart_config.to_aria2_args(&session_path);

                        let shell = app_handle.shell();
                        match shell.sidecar(SIDECAR_NAME) {
                            Ok(cmd) => match cmd.args(&args).spawn() {
                                Ok((new_rx, new_child)) => {
                                    // Store new PID
                                    let new_pid = new_child.pid();
                                    if let Ok(mut pid_lock) = process_pid.lock() {
                                        *pid_lock = Some(new_pid);
                                    }
                                    if let Ok(mut handle) = child_handle.lock() {
                                        *handle = Some(new_child);
                                    }
                                    info!(
                                        "aria2c restarted on port {}, pid={}",
                                        port, new_pid
                                    );
                                    drop(new_rx);
                                }
                                Err(e) => {
                                    error!("Failed to restart aria2c: {}", e);
                                }
                            },
                            Err(e) => {
                                error!("Failed to create sidecar for restart: {}", e);
                            }
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Gracefully shutdown aria2c.
    pub fn shutdown(&self) -> Result<(), String> {
        // Prevent double shutdown: only the first call proceeds
        if self
            .shutdown_flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            info!("Shutdown already in progress, skipping");
            return Ok(());
        }

        self.kill_aria2_process();
        Ok(())
    }

    /// Restart aria2c: kill current process, reset state, then start fresh.
    pub fn restart(&self) -> Result<(), String> {
        info!("Restarting aria2c...");

        // Set flag to stop monitor task from interfering
        self.shutdown_flag.store(true, Ordering::SeqCst);

        self.kill_aria2_process();

        // Reset flag so start() can proceed
        self.shutdown_flag.store(false, Ordering::SeqCst);

        self.start()
    }

    /// Internal: kill the aria2c process (RPC + force kill).
    fn kill_aria2_process(&self) {
        let port = self
            .actual_port
            .lock()
            .map(|p| *p)
            .unwrap_or(DEFAULT_RPC_PORT);
        let secret = self
            .config
            .lock()
            .map(|c| c.rpc_secret.clone())
            .unwrap_or_default();

        // Try graceful shutdown via JSON-RPC
        info!("Sending aria2.shutdown RPC call to port {}", port);
        match send_shutdown_rpc(port, &secret) {
            Ok(_) => info!("aria2 shutdown RPC sent successfully"),
            Err(e) => warn!("Failed to send aria2 shutdown RPC: {}", e),
        }

        // Wait for graceful exit
        std::thread::sleep(Duration::from_secs(ARIA2_SHUTDOWN_TIMEOUT_SECS));

        // Force kill via child handle
        let mut killed_via_handle = false;
        if let Ok(mut handle) = self.child_handle.lock() {
            if let Some(child) = handle.take() {
                info!("Force killing aria2c process via child handle");
                match child.kill() {
                    Ok(_) => {
                        killed_via_handle = true;
                        info!("aria2c killed via child handle");
                    }
                    Err(e) => warn!("Failed to kill aria2c via child handle: {}", e),
                }
            } else {
                // Child handle already consumed (process may have exited via RPC)
                info!("No child handle found, process likely already exited");
                killed_via_handle = true; // no need to fallback
            }
        }

        // Fallback: kill by PID using OS command
        if !killed_via_handle {
            if let Ok(pid_lock) = self.process_pid.lock() {
                if let Some(pid) = *pid_lock {
                    warn!("Falling back to OS-level kill for aria2c pid={}", pid);
                    force_kill_by_pid(pid);
                }
            }
        }

        // Clear PID
        if let Ok(mut pid_lock) = self.process_pid.lock() {
            *pid_lock = None;
        }

        info!("aria2c shutdown complete");
    }

    /// Get the current RPC port.
    pub fn get_port(&self) -> u16 {
        self.actual_port
            .lock()
            .map(|p| *p)
            .unwrap_or(DEFAULT_RPC_PORT)
    }

    /// Get the RPC secret.
    pub fn get_secret(&self) -> String {
        self.config
            .lock()
            .map(|c| c.rpc_secret.clone())
            .unwrap_or_default()
    }

    /// Check if aria2c is currently running.
    pub fn is_running(&self) -> bool {
        self.child_handle
            .lock()
            .map(|h| h.is_some())
            .unwrap_or(false)
    }
}

/// Default RPC port constant (used when lock fails)
const DEFAULT_RPC_PORT: u16 = 6800;

/// Force kill a process by PID using OS-level commands.
fn force_kill_by_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        info!("Running taskkill /F /PID {}", pid);
        match std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if output.status.success() {
                    info!("taskkill succeeded: {}", stdout.trim());
                } else {
                    warn!("taskkill failed: {} {}", stdout.trim(), stderr.trim());
                }
            }
            Err(e) => error!("Failed to run taskkill: {}", e),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        info!("Sending SIGKILL to pid {}", pid);
        match std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    info!("kill -9 succeeded for pid {}", pid);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("kill -9 failed: {}", stderr.trim());
                }
            }
            Err(e) => error!("Failed to run kill: {}", e),
        }
    }
}

/// Send an aria2.shutdown JSON-RPC call to gracefully stop aria2.
fn send_shutdown_rpc(port: u16, secret: &str) -> Result<(), String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let body = format!(
        r#"{{"jsonrpc":"2.0","id":"shutdown","method":"aria2.shutdown","params":["token:{}"]}}"#,
        secret
    );

    let request = format!(
        "POST /jsonrpc HTTP/1.1\r\n\
         Host: 127.0.0.1:{}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {}",
        port,
        body.len(),
        body
    );

    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("Invalid address: {}", e))?,
        Duration::from_secs(2),
    )
    .map_err(|e| format!("Failed to connect to aria2 RPC: {}", e))?;

    // Set read/write timeouts to prevent blocking indefinitely
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| format!("Failed to set write timeout: {}", e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| format!("Failed to set read timeout: {}", e))?;

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send shutdown request: {}", e))?;

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    info!("aria2 shutdown RPC response: {}", response);
    Ok(())
}
