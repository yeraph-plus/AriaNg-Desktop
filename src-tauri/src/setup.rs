use log::info;
use tauri::{App, Manager};

use crate::aria2::config;
use crate::aria2::manager::Aria2Manager;

/// Initialize the application on startup.
/// - Ensure aria2.conf exists (generate defaults on first run)
/// - Start aria2 sidecar process
/// - Register Aria2Manager in Tauri's managed state
/// - Inject AriaNg RPC host/port configuration
pub fn initialize(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Ensure the app data directory exists
    std::fs::create_dir_all(&app_data_dir)?;

    info!("App data directory: {:?}", app_data_dir);

    // Ensure aria2.conf exists (creates default on first run)
    let conf_path = config::ensure_conf_file(&app_data_dir)
        .map_err(|e| format!("Failed to ensure aria2.conf: {}", e))?;

    let preferred_port = config::get_preferred_port(&conf_path);
    let rpc_secret = config::get_rpc_secret(&conf_path);

    info!("aria2.conf loaded: preferred_port={}", preferred_port);

    // Create and start aria2 manager
    let manager = Aria2Manager::new(
        app.handle().clone(),
        conf_path,
        rpc_secret,
        preferred_port,
        app_data_dir,
    );

    match manager.start() {
        Ok(()) => info!("aria2c started successfully"),
        Err(e) => {
            log::error!("Failed to start aria2c: {}", e);
        }
    }

    // Register manager in Tauri state
    app.manage(manager);

    // Inject AriaNg RPC host/port override into localStorage
    setup_ariang_config_injection(app)?;

    // Inject title sync script to keep window title in sync with AriaNg's document.title
    setup_title_sync(app)?;

    // Inject CSS to hide buttons managed by Tauri (Shutdown Aria2, Save Session)
    inject_custom_styles(app)?;

    Ok(())
}

/// Inject JavaScript into the webview to ensure AriaNg's localStorage
/// has the correct RPC host, port, and protocol for the local aria2c.
/// The RPC secret is intentionally left for the user to configure through AriaNg's settings UI.
fn setup_ariang_config_injection(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let manager = app.state::<Aria2Manager>();
    let port = manager.get_port();

    let js = format!(
        r#"
        (function() {{
            function configureAriaNg() {{
                try {{
                    var optionsJson = localStorage.getItem('AriaNg.Options');

                    if (!optionsJson) {{
                        // First run - AriaNg hasn't initialized yet, wait
                        setTimeout(configureAriaNg, 500);
                        return;
                    }}

                    var options = JSON.parse(optionsJson);
                    var rpcPort = '{port}';
                    var needsReload = false;

                    // Only update host/port/protocol, leave secret for user
                    if (options.rpcHost !== '127.0.0.1' || options.rpcPort !== rpcPort || options.protocol !== 'ws') {{
                        options.rpcHost = '127.0.0.1';
                        options.rpcPort = rpcPort;
                        options.protocol = 'ws';
                        localStorage.setItem('AriaNg.Options', JSON.stringify(options));
                        console.log('[AriaNg App] RPC connection updated: ws://127.0.0.1:' + rpcPort);
                        needsReload = true;
                    }}

                    if (needsReload) {{
                        setTimeout(function() {{ location.reload(); }}, 200);
                    }}
                }} catch (e) {{
                    console.error('[AriaNg App] Failed to configure AriaNg:', e);
                }}
            }}

            if (document.readyState === 'complete') {{
                setTimeout(configureAriaNg, 1000);
            }} else {{
                window.addEventListener('load', function() {{
                    setTimeout(configureAriaNg, 1000);
                }});
            }}
        }})();
        "#,
        port = port,
    );

    window
        .eval(&js)
        .map_err(|e| format!("Failed to inject AriaNg config: {}", e))?;

    info!("AriaNg RPC host/port injection scheduled");
    Ok(())
}

/// Inject JavaScript to sync AriaNg's document.title to the native window title.
/// Uses MutationObserver to watch for title changes set by AriaNg (e.g. download speed).
fn setup_title_sync(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let js = r#"
        (function() {
            var lastTitle = '';

            function syncTitle() {
                var title = document.title;
                if (title && title !== lastTitle) {
                    lastTitle = title;
                    window.__TAURI_INTERNALS__.invoke('sync_window_title', { title: title });
                }
            }

            function setupObserver() {
                var titleEl = document.querySelector('title');
                if (!titleEl) {
                    setTimeout(setupObserver, 500);
                    return;
                }

                // Initial sync
                syncTitle();

                // Watch for DOM-based title changes
                var observer = new MutationObserver(syncTitle);
                observer.observe(titleEl, { childList: true, characterData: true, subtree: true });
            }

            if (document.readyState === 'complete') {
                setupObserver();
            } else {
                window.addEventListener('load', setupObserver);
            }
        })();
    "#;

    window
        .eval(js)
        .map_err(|e| format!("Failed to inject title sync script: {}", e))?;

    info!("Title sync script injected");
    Ok(())
}

/// Inject CSS to hide AriaNg UI elements that are managed by Tauri.
/// Hides "Shutdown Aria2" and "Save Session" buttons since aria2 lifecycle
/// is fully managed by the Tauri sidecar manager.
fn inject_custom_styles(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let js = r#"
        (function() {
            var style = document.createElement('style');
            style.textContent = 'button[ng-click="shutdown()"] { display: none !important; }';
            document.head.appendChild(style);
        })();
    "#;

    window
        .eval(js)
        .map_err(|e| format!("Failed to inject custom styles: {}", e))?;

    info!("Custom styles injected (hidden: shutdown, saveSession buttons)");
    Ok(())
}
