// Prevents additional console window on Windows in release, do not remove!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    server_url: Option<String>,
}

// Helper to get the path to the config file
fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;

    // Ensure the directory exists
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    Ok(config_dir.join("config.json"))
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
    let mut normalized_url = url.trim().to_string();

    // Normalize protocol (case-insensitive check so "Https://" doesn't get double-prefixed)
    let lower = normalized_url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        normalized_url = format!("https://{}", normalized_url);
    }
    // Strip trailing slash
    if normalized_url.ends_with('/') {
        normalized_url.pop();
    }
    // Rewrite localhost → 127.0.0.1 to avoid session cookie origin mismatch
    // (browsers treat them as different origins, so OAuth cookies won't carry over)
    normalized_url = normalized_url
        .replace("://localhost:", "://127.0.0.1:")
        .replace("://localhost/", "://127.0.0.1/");
    if normalized_url.ends_with("://localhost") {
        normalized_url = normalized_url.replace("://localhost", "://127.0.0.1:3000");
    }

    let config = Config {
        server_url: Some(normalized_url.clone()),
    };

    let content = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;

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
            copy_and_paste_link
        ])
        .setup(|app| {
            // Hide from dock on macOS; the tray icon is the only entry point.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("memeBucket Picker")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
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

            let _window = app.get_webview_window("main").unwrap();

            // Register global hotkey: CmdOrCtrl+Shift+M
            let shortcut: Shortcut = "CmdOrCtrl+Shift+M".parse().unwrap();
            app.global_shortcut()
                .on_shortcut(shortcut, move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                // The paste flow hides the whole app (app.hide()), so unhide
                                // the application before showing the window or it stays hidden.
                                #[cfg(target_os = "macos")]
                                let _ = app.show();

                                // Position window so its bottom-left corner is at the cursor
                                use enigo::{Enigo, MouseControllable};
                                let enigo = Enigo::new();
                                let (cx, cy) = enigo.mouse_location();
                                let _ = window.set_position(tauri::LogicalPosition::new(
                                    cx as f64,
                                    (cy - 500).max(0) as f64,
                                ));
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
