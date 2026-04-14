use log::info;
use tauri::{App, Manager};

use crate::aria2::config::Aria2Config;
use crate::aria2::manager::Aria2Manager;
use crate::constants::ARIANG_OPTIONS_FILE_NAME;

/// Initialize the application on startup.
/// - Load or create aria2 config
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

    // Load or create config
    let config = Aria2Config::load_or_create(&app_data_dir)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    info!("Config loaded: port={}, download_dir={}", config.rpc_port, config.download_dir);

    // Create and start aria2 manager
    let manager = Aria2Manager::new(app.handle().clone(), config, app_data_dir);

    match manager.start() {
        Ok(()) => info!("aria2c started successfully"),
        Err(e) => {
            log::error!("Failed to start aria2c: {}", e);
        }
    }

    // Register manager in Tauri state
    app.manage(manager);

    // Inject AriaNg RPC host/port (secret is left for user to configure in AriaNg UI)
    setup_ariang_config_injection(app)?;

    // Inject title sync script to keep window title in sync with AriaNg's document.title
    setup_title_sync(app)?;

    // Inject CSS to hide buttons managed by Tauri (Shutdown Aria2, Save Session)
    inject_custom_styles(app)?;

    Ok(())
}

/// Inject JavaScript into the webview to:
/// 1. Restore persisted AriaNg.Options from disk into localStorage (if available)
/// 2. Configure the correct RPC host/port/protocol
/// 3. Periodically persist AriaNg.Options changes back to disk via Tauri command
fn setup_ariang_config_injection(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("Failed to get main window")?;

    let manager = app.state::<Aria2Manager>();
    let port = manager.get_port();

    // Load previously persisted AriaNg options from disk
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let options_path = app_data_dir.join(ARIANG_OPTIONS_FILE_NAME);
    let saved_options = std::fs::read_to_string(&options_path).ok();

    // Escape the JSON string for embedding in JS (handle quotes, backslashes, newlines)
    let saved_options_js = match &saved_options {
        Some(json) => {
            let escaped = json
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('\n', "\\n")
                .replace('\r', "\\r");
            format!("'{}'", escaped)
        }
        None => "null".to_string(),
    };

    let js = format!(
        r#"
        (function() {{
            var savedOptions = {saved_options_js};

            function configureAriaNg() {{
                try {{
                    var optionsJson = localStorage.getItem('AriaNg.Options');

                    if (!optionsJson && !savedOptions) {{
                        // First run - AriaNg hasn't initialized yet, wait
                        setTimeout(configureAriaNg, 500);
                        return;
                    }}

                    // Restore persisted options if localStorage is empty but disk has saved data
                    if (!optionsJson && savedOptions) {{
                        localStorage.setItem('AriaNg.Options', savedOptions);
                        optionsJson = savedOptions;
                        console.log('[AriaNg App] Restored options from disk');
                    }}

                    var options = JSON.parse(optionsJson);
                    var rpcPort = '{port}';
                    var needsReload = false;

                    // Only update host/port/protocol, leave secret and other settings for user
                    if (options.rpcHost !== '127.0.0.1' || options.rpcPort !== rpcPort || options.protocol !== 'ws') {{
                        options.rpcHost = '127.0.0.1';
                        options.rpcPort = rpcPort;
                        options.protocol = 'ws';
                        localStorage.setItem('AriaNg.Options', JSON.stringify(options));
                        console.log('[AriaNg App] RPC connection updated: ws://127.0.0.1:' + rpcPort);
                        needsReload = true;
                    }}

                    // Start periodic persistence of AriaNg.Options to disk
                    setupOptionsPersistence();

                    if (needsReload) {{
                        setTimeout(function() {{ location.reload(); }}, 200);
                    }}
                }} catch (e) {{
                    console.error('[AriaNg App] Failed to configure AriaNg:', e);
                }}
            }}

            // Periodically check for AriaNg.Options changes and persist to disk
            function setupOptionsPersistence() {{
                var lastSaved = localStorage.getItem('AriaNg.Options') || '';

                setInterval(function() {{
                    try {{
                        var current = localStorage.getItem('AriaNg.Options');
                        if (current && current !== lastSaved) {{
                            lastSaved = current;
                            window.__TAURI_INTERNALS__.invoke('save_ariang_options', {{ json: current }});
                            console.log('[AriaNg App] Options persisted to disk');
                        }}
                    }} catch (e) {{
                        console.error('[AriaNg App] Failed to persist options:', e);
                    }}
                }}, 5000);
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
        saved_options_js = saved_options_js,
        port = port,
    );

    window
        .eval(&js)
        .map_err(|e| format!("Failed to inject AriaNg config: {}", e))?;

    if saved_options.is_some() {
        info!("AriaNg config injection scheduled (with persisted options restored)");
    } else {
        info!("AriaNg config injection scheduled (no persisted options found)");
    }
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
