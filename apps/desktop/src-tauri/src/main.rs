// Prevents additional console window on Windows in release, do not remove!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::ipc::CapabilityBuilder;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

/// Permissions the bundled local UI already has (see
/// `capabilities/default.json`) — mirrored here so a self-hosted remote
/// server the user points the app at gets the same functionality, but ONLY
/// for that exact origin (see `grant_remote_capability_for_server_url`).
const REMOTE_SERVER_PERMISSIONS: &[&str] = &[
    "core:default",
    "core:window:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "core:window:allow-close",
    "core:window:allow-maximize",
    "core:window:allow-minimize",
    "core:window:allow-start-dragging",
    "core:window:allow-center",
    "core:window:allow-is-visible",
    "global-shortcut:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-is-registered",
    "shell:default",
    "shell:allow-open",
    "allow-get-server-url",
    "allow-set-server-url",
    "allow-navigate-to",
    "allow-hide-window",
    "allow-copy-and-paste-link",
];

/// Reduces a URL to its origin glob (`scheme://host[:port]/*`) for use as a
/// Tauri capability's `remote.urls` entry. Rejects anything that isn't a
/// plain http/https URL with a host.
fn remote_origin_pattern(url_str: &str) -> Option<String> {
    let parsed: tauri::Url = url_str.parse().ok()?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    let origin = match parsed.port() {
        Some(port) => format!("{}://{}:{}", parsed.scheme(), host, port),
        None => format!("{}://{}", parsed.scheme(), host),
    };
    Some(format!("{origin}/*"))
}

/// Grants the main window's native IPC bridge to exactly the origin of
/// `url` — never a wildcard. This is how the app stays "point it at any
/// self-hosted server" without handing every HTTPS site on the internet the
/// same native command access a static `remote.urls: ["https://*/*"]` entry
/// would (the previous approach, and a genuine SSRF/IPC-bridge vulnerability).
///
/// Tauri's dynamic-ACL grants are additive with no revoke API, so callers
/// that change the configured server URL at runtime (see `save_settings`)
/// must restart the app afterward — otherwise the previous origin would keep
/// its grant until the process exits.
fn grant_remote_capability_for_server_url(app: &AppHandle, url: &str) {
    let Some(pattern) = remote_origin_pattern(url) else {
        eprintln!("Not granting remote capability: invalid server URL {url}");
        return;
    };

    let mut capability = CapabilityBuilder::new("dynamic-server-remote")
        .window("main")
        .remote(pattern);
    for permission in REMOTE_SERVER_PERMISSIONS {
        capability = capability.permission(*permission);
    }

    if let Err(e) = app.add_capability(capability) {
        eprintln!("Failed to grant remote capability for {url}: {e}");
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
struct Config {
    server_url: Option<String>,
    hotkey: Option<String>,
    window_x: Option<f64>,
    window_y: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        Self { server_url: None, hotkey: None, window_x: None, window_y: None }
    }
}

// Holds a pending update download so the user can trigger install via a tray item.
struct PendingUpdate(Mutex<Option<tauri_plugin_updater::Update>>);

// Helper to get the path to the config file
fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;

    // Ensure the directory exists
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    Ok(config_dir.join("config.json"))
}

fn normalize_server_url(url: &str) -> String {
    let mut normalized = url.trim().to_string();
    let lower = normalized.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        normalized = format!("https://{}", normalized);
    }
    if normalized.ends_with('/') {
        normalized.pop();
    }
    normalized = normalized
        .replace("://localhost:", "://127.0.0.1:")
        .replace("://localhost/", "://127.0.0.1/");
    if normalized.ends_with("://localhost") {
        normalized = normalized.replace("://localhost", "://127.0.0.1:3000");
    }
    normalized
}

// Read the server URL from local configuration
#[tauri::command]
fn get_server_url(app: AppHandle) -> Result<Option<String>, String> {
    let path = get_config_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: Config = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(config.server_url)
}

// Write the server URL to local configuration and return the normalized URL
#[tauri::command]
fn set_server_url(app: AppHandle, url: String) -> Result<String, String> {
    let path = get_config_path(&app)?;
    let normalized_url = normalize_server_url(&url);
    let mut config: Config = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };
    config.server_url = Some(normalized_url.clone());
    let content = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    grant_remote_capability_for_server_url(&app, &normalized_url);
    Ok(normalized_url)
}

// Navigate the main window to a specific URL (Rust-driven to bypass WebKit security sandboxing)
#[tauri::command]
fn navigate_to(app: AppHandle, url: String) -> Result<(), String> {
    println!("navigate_to called with url: {}", url);
    if let Some(window) = app.get_webview_window("main") {
        let target_url: tauri::Url = url.parse().map_err(|e| {
            println!("Invalid URL: {}", e);
            format!("Invalid URL: {}", e)
        })?;
        println!("navigating to {:?}", target_url);
        if let Err(e) = window.navigate(target_url.clone()) {
            println!("navigate error: {}", e);
            return Err(e.to_string());
        }
    } else {
        println!("main window not found!");
    }
    Ok(())
}

// Hide the picker window
#[tauri::command]
fn hide_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Save the window's current position to config
#[tauri::command]
fn save_window_position(app: AppHandle, x: f64, y: f64) -> Result<(), String> {
    let path = get_config_path(&app)?;
    let mut config: Config = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };
    config.window_x = Some(x);
    config.window_y = Some(y);
    let content = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

// macOS: Accessibility check + direct CGEvent key injection via ApplicationServices.
// enigo 0.1.3 is unreliable on recent macOS; we call CGEventPost directly instead.
#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn CGEventSourceCreate(state_id: i32) -> *mut std::ffi::c_void;
    fn CGEventCreateKeyboardEvent(
        source: *mut std::ffi::c_void,
        virtual_key: u16,
        key_down: bool,
    ) -> *mut std::ffi::c_void;
    fn CGEventSetFlags(event: *mut std::ffi::c_void, flags: u64);
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
}

// CFRelease is in CoreFoundation, already transitively linked by Tauri/AppKit.
#[cfg(target_os = "macos")]
extern "C" {
    fn CFRelease(cf: *const std::ffi::c_void);
}

// Copy the selected URL to the clipboard and paste it into the active window
#[tauri::command]
fn copy_and_paste_link(app: AppHandle, url: String) -> Result<(), String> {
    // 1. Copy URL to clipboard first (always succeeds, no special permissions needed).
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| format!("pbcopy spawn failed: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(url.as_bytes())
                .map_err(|e| format!("pbcopy write failed: {e}"))?;
        }
        child
            .wait()
            .map_err(|e| format!("pbcopy wait failed: {e}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;
        clipboard
            .set_text(&url)
            .map_err(|e| format!("Failed to copy to clipboard: {e}"))?;
    }

    // 2. Check Accessibility permission before hiding the window.
    //    enigo's CGEventPost silently does nothing without it, so if we hid
    //    the window first the user would see a blank state with no feedback.
    #[cfg(target_os = "macos")]
    if !unsafe { AXIsProcessTrusted() } {
        // Open the Accessibility pane so the user can grant access.
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        return Err("Accessibility permission required for auto-paste.\n\
             System Preferences has been opened — add this app under Privacy → Accessibility, \
             then try again.\n\
             (The URL is already in your clipboard.)"
            .to_string());
    }

    // 3. Hide the app, wait for focus to return, then paste — all on a SEPARATE
    //    thread. A non-async Tauri command runs on the main (UI) thread, so a
    //    blocking sleep here would starve the event loop: the hide we request below
    //    couldn't actually run until the command returned, meaning the window would
    //    stay frozen on screen and we'd post Cmd+V while our own picker is still
    //    frontmost. Running off-thread lets the main loop process the hide and switch
    //    focus to the target app first, so the paste lands where the user expects.
    let app_handle = app.clone();
    std::thread::spawn(move || {
        // Hide the whole application so focus returns to the previously-active app.
        // On macOS, window.hide() only orders the window out (orderOut:) — the app
        // itself stays frontmost, so the synthesized Cmd+V would land on our own
        // (now invisible) app. Hiding the application (like Cmd+H) deactivates us and
        // macOS restores focus to the previous app.
        #[cfg(target_os = "macos")]
        let _ = app_handle.hide();

        #[cfg(not(target_os = "macos"))]
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.hide();
        }

        // Give the app time to fully hide and the previous app to regain focus.
        std::thread::sleep(std::time::Duration::from_millis(250));

        // Simulate Cmd+V in the now-focused window via direct CGEventPost.
        // We post a real Command key press around the 'v' press rather than only
        // setting the command flag mask: with an HIDSystemState source the system
        // reconciles an event's modifier flags against the actual hardware keyboard
        // state, so a flag-only 'v' has its Command bit stripped (no Command key is
        // physically held) and apps like Discord receive a bare 'v'. Pressing the
        // Command key for real makes the modifier state consistent so the paste fires.
        // (enigo 0.1.3 silently fails on recent macOS; we call the framework directly.)
        #[cfg(target_os = "macos")]
        unsafe {
            // kCGEventSourceStateHIDSystemState = 1
            let source = CGEventSourceCreate(1);

            const CMD_FLAG: u64 = 0x0010_0000; // kCGEventFlagMaskCommand
            const KEY_V: u16 = 9; // 'v'
            const KEY_CMD: u16 = 55; // left Command (kVK_Command)
            const HID_TAP: u32 = 0; // kCGHIDEventTap

            // Helper to post a single keyboard event with the given flags.
            let post = |key: u16, down: bool, flags: u64| {
                let event = CGEventCreateKeyboardEvent(source, key, down);
                CGEventSetFlags(event, flags);
                CGEventPost(HID_TAP, event);
                CFRelease(event as *const _);
            };

            // Command down → V down (with Command) → V up (with Command) → Command up.
            post(KEY_CMD, true, CMD_FLAG);
            std::thread::sleep(std::time::Duration::from_millis(10));
            post(KEY_V, true, CMD_FLAG);
            std::thread::sleep(std::time::Duration::from_millis(10));
            post(KEY_V, false, CMD_FLAG);
            std::thread::sleep(std::time::Duration::from_millis(10));
            post(KEY_CMD, false, 0);

            CFRelease(source as *const _);
        }

        #[cfg(not(target_os = "macos"))]
        {
            use enigo::{Enigo, Key, KeyboardControllable};
            let mut enigo = Enigo::new();
            enigo.key_down(Key::Control);
            enigo.key_click(Key::Layout('v'));
            enigo.key_up(Key::Control);
        }
    });

    Ok(())
}

fn rebuild_tray_menu(app: &AppHandle, pending_version: Option<&str>) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let (Ok(settings_item), Ok(quit_item)) = (
        MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>),
        MenuItem::with_id(app, "quit", "Quit", true, None::<&str>),
    ) else { return };

    if let Some(version) = pending_version {
        if let Ok(update_item) = MenuItem::with_id(app, "restart_update", &format!("Restart to update (v{})", version), true, None::<&str>) {
            if let Ok(menu) = Menu::with_items(app, &[&settings_item, &update_item, &quit_item]) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    } else {
        if let Ok(check_item) = MenuItem::with_id(app, "check_updates", "Check for updates", true, None::<&str>) {
            if let Ok(menu) = Menu::with_items(app, &[&settings_item, &check_item, &quit_item]) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}

fn position_near_cursor(window: &tauri::WebviewWindow) {
    use enigo::{Enigo, MouseControllable};
    let enigo = Enigo::new();
    let (cx, cy) = enigo.mouse_location();
    let positioned = if let Ok(Some(monitor)) = window.current_monitor() {
        let scale = monitor.scale_factor();
        let mpos = monitor.position();
        let msize = monitor.size();
        let wsize = window.outer_size()
            .unwrap_or(tauri::PhysicalSize { width: 360, height: 500 });
        let mon_lx = (mpos.x as f64 / scale) as i32;
        let mon_ly = (mpos.y as f64 / scale) as i32;
        let mon_lw = (msize.width as f64 / scale) as i32;
        let mon_lh = (msize.height as f64 / scale) as i32;
        let win_lw = (wsize.width as f64 / scale) as i32;
        let win_lh = (wsize.height as f64 / scale) as i32;
        let clamped_x = cx.clamp(mon_lx, (mon_lx + mon_lw - win_lw).max(mon_lx));
        let clamped_y = (cy - win_lh).clamp(mon_ly, (mon_ly + mon_lh - win_lh).max(mon_ly));
        let _ = window.set_position(tauri::LogicalPosition::new(clamped_x as f64, clamped_y as f64));
        true
    } else {
        false
    };
    if !positioned {
        let enigo = Enigo::new();
        let (cx, cy) = enigo.mouse_location();
        let _ = window.set_position(tauri::LogicalPosition::new(cx as f64, (cy - 500).max(0) as f64));
    }
}

fn handle_hotkey(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let Some(window) = app.get_webview_window("main") else { return };
    let is_visible = window.is_visible().unwrap_or(false);
    if is_visible {
        let _ = window.hide();
        return;
    }

    #[cfg(target_os = "macos")]
    let _ = app.show();

    let saved_pos: Option<(f64, f64)> = get_config_path(app)
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Config>(&s).ok())
        .and_then(|c| c.window_x.zip(c.window_y));

    if let Some((sx, sy)) = saved_pos {
        let fits = window.available_monitors()
            .ok()
            .unwrap_or_default()
            .iter()
            .any(|m| {
                let scale = m.scale_factor();
                let mpos = m.position();
                let msize = m.size();
                let wsize = window.outer_size()
                    .unwrap_or(tauri::PhysicalSize { width: 360, height: 500 });
                let mon_lx = (mpos.x as f64 / scale) as i32;
                let mon_ly = (mpos.y as f64 / scale) as i32;
                let mon_lw = (msize.width as f64 / scale) as i32;
                let mon_lh = (msize.height as f64 / scale) as i32;
                let win_lw = (wsize.width as f64 / scale) as i32;
                let win_lh = (wsize.height as f64 / scale) as i32;
                let sx_i = sx as i32;
                let sy_i = sy as i32;
                sx_i >= mon_lx && sy_i >= mon_ly
                    && sx_i + win_lw <= mon_lx + mon_lw
                    && sy_i + win_lh <= mon_ly + mon_lh
            });
        if fits {
            let _ = window.set_position(tauri::LogicalPosition::new(sx, sy));
        } else {
            position_near_cursor(&window);
        }
    } else {
        position_near_cursor(&window);
    }

    let _ = window.show();
    let _ = window.set_focus();
}

#[derive(Serialize)]
struct Settings {
    server_url: Option<String>,
    hotkey: String,
    autostart: bool,
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Result<Settings, String> {
    use tauri_plugin_autostart::ManagerExt;
    let path = get_config_path(&app)?;
    let config: Config = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };
    Ok(Settings {
        server_url: config.server_url,
        hotkey: config.hotkey.unwrap_or_else(|| "CmdOrCtrl+Shift+M".to_string()),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    })
}

#[tauri::command]
fn save_settings(app: AppHandle, server_url: String, hotkey: String, autostart: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    // Validate new shortcut before touching anything
    let new_shortcut: Shortcut = hotkey
        .parse()
        .map_err(|_| format!("Invalid hotkey: {}", hotkey))?;

    // Read current config
    let path = get_config_path(&app)?;
    let mut config: Config = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Config::default()
    };
    let old_hotkey = config.hotkey.clone().unwrap_or_else(|| "CmdOrCtrl+Shift+M".to_string());

    // Swap global shortcut; roll back if the new one can't be registered
    if let Ok(old_shortcut) = old_hotkey.parse::<Shortcut>() {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }
    if let Err(e) = app.global_shortcut().on_shortcut(new_shortcut, |app, shortcut, event| handle_hotkey(app, shortcut, event)) {
        // Re-register the old shortcut so the app isn't left with no hotkey
        if let Ok(old_shortcut) = old_hotkey.parse::<Shortcut>() {
            let _ = app.global_shortcut().on_shortcut(old_shortcut, |app, shortcut, event| handle_hotkey(app, shortcut, event));
        }
        return Err(format!("Could not register shortcut (already in use?): {}", e));
    }

    // Toggle autostart
    let autolaunch = app.autolaunch();
    let currently_enabled = autolaunch.is_enabled().unwrap_or(false);
    match (autostart, currently_enabled) {
        (true, false) => { autolaunch.enable().map_err(|e| e.to_string())?; }
        (false, true) => { autolaunch.disable().map_err(|e| e.to_string())?; }
        _ => {}
    }

    // Persist config
    let previous_server_url = config.server_url.clone();
    let normalized_url = if server_url.trim().is_empty() {
        None
    } else {
        Some(normalize_server_url(&server_url))
    };
    config.hotkey = Some(hotkey);
    config.server_url = normalized_url.clone();
    let content = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;

    match &normalized_url {
        Some(url) if normalized_url != previous_server_url => {
            // The server URL changed: grant the new origin's capability, then
            // restart. Dynamic-ACL grants can't be revoked, so simply
            // navigating would leave the previous origin's IPC access active
            // for the rest of this run — restarting rebuilds the runtime
            // authority from scratch so only the new URL is ever granted.
            grant_remote_capability_for_server_url(&app, url);
            app.restart();
        }
        Some(url) => {
            // Unchanged URL: nothing to (re-)grant, just make sure the window
            // reflects the current picker route.
            if let Some(main_window) = app.get_webview_window("main") {
                if let Ok(parsed) = format!("{}/picker", url).parse() {
                    let _ = main_window.navigate(parsed);
                }
            }
        }
        None => {}
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_server_url,
            set_server_url,
            navigate_to,
            hide_window,
            copy_and_paste_link,
            save_window_position,
            get_settings,
            save_settings,
        ])
        .setup(|app| {
            // Hide from dock on macOS; the tray icon is the only entry point.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Register managed state for a pending update the user can trigger via tray.
            app.manage(PendingUpdate(Mutex::new(None)));

            let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let check_updates_item = MenuItem::with_id(app, "check_updates", "Check for updates", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &check_updates_item, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("memeBucket Picker")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("settings") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "check_updates" => {
                            let app2 = app.clone();
                            tauri::async_runtime::spawn(async move {
                                use tauri_plugin_updater::UpdaterExt;
                                let updater = match app2.updater() {
                                    Ok(u) => u,
                                    Err(_) => return,
                                };
                                if let Ok(Some(update)) = updater.check().await {
                                    let version = update.version.clone();
                                    if let Some(state) = app2.try_state::<PendingUpdate>() {
                                        *state.0.lock().unwrap() = Some(update);
                                    }
                                    rebuild_tray_menu(&app2, Some(&version));
                                }
                            });
                        }
                        "restart_update" => {
                            let app2 = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Some(state) = app2.try_state::<PendingUpdate>() {
                                    let update = state.0.lock().unwrap().take();
                                    if let Some(update) = update {
                                        match update.download_and_install(|_, _| {}, || {}).await {
                                            Ok(_) => app2.restart(),
                                            Err(e) => eprintln!("Update install failed: {e}"),
                                        }
                                    }
                                }
                            });
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                #[cfg(target_os = "macos")]
                                let _ = app.hide();
                                #[cfg(not(target_os = "macos"))]
                                let _ = window.hide();
                            } else {
                                #[cfg(target_os = "macos")]
                                let _ = app.show();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(settings_window) = app.get_webview_window("settings") {
                let win = settings_window.clone();
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            // Check for updates in the background on startup.
            // If an update is found: store it in managed state and add a tray item so
            // the user can choose when to install (rather than auto-restarting silently).
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_updater::UpdaterExt;
                if let Ok(Some(update)) = app_handle.updater().unwrap().check().await {
                    let version = update.version.clone();
                    println!(
                        "Update available: {} → {}",
                        update.current_version,
                        update.version
                    );
                    // Store the pending update for user-triggered install.
                    if let Some(state) = app_handle.try_state::<PendingUpdate>() {
                        *state.0.lock().unwrap() = Some(update);
                    }
                    // Rebuild tray menu to surface a "Restart to update" item.
                    rebuild_tray_menu(&app_handle, Some(&version));
                }
            });

            let initial_config: Config = get_config_path(app.handle())
                .ok()
                .and_then(|p| fs::read_to_string(p).ok())
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            // Re-grant the previously configured server's origin before any
            // window content loads and tries to `navigate_to` it — the grant
            // doesn't persist across restarts, only the config does.
            if let Some(url) = &initial_config.server_url {
                grant_remote_capability_for_server_url(app.handle(), url);
            }

            // Load saved hotkey or fall back to default
            let initial_hotkey = initial_config
                .hotkey
                .unwrap_or_else(|| "CmdOrCtrl+Shift+M".to_string());
            let shortcut: Shortcut = initial_hotkey
                .parse()
                .unwrap_or_else(|_| "CmdOrCtrl+Shift+M".parse().unwrap());
            let register_result = app.global_shortcut()
                .on_shortcut(shortcut, |app, s, e| handle_hotkey(app, s, e));
            if register_result.is_err() && initial_hotkey != "CmdOrCtrl+Shift+M" {
                // Saved custom shortcut failed to register — fall back to the default.
                if let Ok(fallback) = "CmdOrCtrl+Shift+M".parse::<Shortcut>() {
                    let _ = app.global_shortcut()
                        .on_shortcut(fallback, |app, s, e| handle_hotkey(app, s, e));
                }
            }
            // Do not propagate — app must start even if the hotkey can't be registered.

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
