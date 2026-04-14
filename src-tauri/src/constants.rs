/// Default RPC port for aria2
pub const DEFAULT_RPC_PORT: u16 = 6800;

/// Number of ports to scan when default is occupied
pub const PORT_SCAN_RANGE: u16 = 100;

/// Maximum crash restart attempts within the crash window
pub const MAX_CRASH_RETRIES: u32 = 5;

/// Time window (seconds) for crash retry counting
pub const CRASH_WINDOW_SECS: u64 = 60;

/// Timeout (seconds) for graceful aria2 shutdown
pub const ARIA2_SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Sidecar binary name used at runtime.
/// Note: externalBin in tauri.conf.json uses "binaries/aria2c" (source path for build),
/// but sidecar() at runtime uses just the base name.
pub const SIDECAR_NAME: &str = "aria2c";

/// Config file name
pub const CONFIG_FILE_NAME: &str = "config.json";

/// AriaNg options persistence file name
pub const ARIANG_OPTIONS_FILE_NAME: &str = "ariang_options.json";

/// aria2 session file name
pub const SESSION_FILE_NAME: &str = "aria2.session";
