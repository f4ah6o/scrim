//! Scrim - Keyboard-driven desktop navigation overlay

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tauri::{Emitter, Manager};

static OVERLAY_ACTIVE: AtomicBool = AtomicBool::new(false);

// Global state for keyboard callback
static KEYBOARD_CALLBACK: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Initialize overlay window with click-through
            setup_overlay_window(app)?;

            // Register global shortcuts
            setup_global_shortcuts(app.handle().clone())?;

            // Setup dock menu for toggling overlay
            setup_dock_menu(app.handle().clone())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_elements,
            commands::perform_action,
            commands::set_click_through,
            commands::check_permissions,
            commands::request_permissions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Setup the overlay window with click-through behavior
fn setup_overlay_window(_app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::{NSWindow, NSWindowStyleMask};
        use cocoa::base::id;
        use objc::{class, msg_send, sel, sel_impl};

        let window = _app
            .get_webview_window("overlay")
            .ok_or("Overlay window not found")?;

        // Get the native NSWindow using the Tauri API
        #[cfg(target_os = "macos")]
        {
            use tauri::utils::config::WindowEffectsConfig;

            // Temporarily disable NSWindow access to test if it's causing the crash
            // The overlay will still have click-through controlled via Tauri API
            log::info!("Overlay window configured (NSWindow access disabled for debugging)");
        }
    }

    Ok(())
}

/// Setup global keyboard shortcuts
fn setup_global_shortcuts(_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Store app handle for keyboard callback
    {
        let mut cb = KEYBOARD_CALLBACK.lock().unwrap();
        *cb = Some(_handle.clone());
    }

    #[cfg(target_os = "macos")]
    {
        use flume::{unbounded, Receiver, Sender};
        use rdev::{listen, Event, EventType, Key};

        // Create a channel for keyboard events
        let (tx, rx): (Sender<String>, Receiver<String>) = unbounded();

        // Spawn keyboard listener thread
        thread::spawn(move || {
            let result = listen(move |event: Event| {
                // Process keyboard events
                if let EventType::KeyPress(key) = event.event_type {
                    // Only process if overlay is active
                    if OVERLAY_ACTIVE.load(Ordering::Relaxed) {
                        match key {
                            Key::KeyA => {
                                let _ = tx.send("A".to_string());
                            }
                            Key::KeyS => {
                                let _ = tx.send("S".to_string());
                            }
                            Key::KeyD => {
                                let _ = tx.send("D".to_string());
                            }
                            Key::KeyF => {
                                let _ = tx.send("F".to_string());
                            }
                            Key::KeyG => {
                                let _ = tx.send("G".to_string());
                            }
                            Key::KeyH => {
                                let _ = tx.send("H".to_string());
                            }
                            Key::KeyJ => {
                                let _ = tx.send("J".to_string());
                            }
                            Key::KeyK => {
                                let _ = tx.send("K".to_string());
                            }
                            Key::KeyL => {
                                let _ = tx.send("L".to_string());
                            }
                            Key::Escape => {
                                let _ = tx.send("Escape".to_string());
                            }
                            _ => {}
                        }
                    }
                }
                // Return empty tuple - rdev 0.5 expects ()
            });

            if let Err(error) = result {
                log::error!("Error listening for keyboard events: {:?}", error);
            }
        });

        // Spawn a thread to forward events to Tauri
        thread::spawn(move || {
            while let Ok(key) = rx.recv() {
                if let Ok(cb) = KEYBOARD_CALLBACK.lock() {
                    if let Some(handle) = cb.as_ref() {
                        let _ = handle.emit("keyboard-input", key);
                    }
                }
            }
        });

        log::info!("Global keyboard event monitoring enabled (A,S,D,F,G,H,J,K,L,Esc)");
    }

    log::info!("Click on scrim-app window to activate overlay");
    Ok(())
}

/// Setup dock menu
fn setup_dock_menu(_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Click on scrim-app window to activate overlay");
    log::info!("Permission prompt: Click 'Retry' button after granting Accessibility permissions");
    Ok(())
}

/// Tauri commands for IPC
mod commands {
    use super::*;
    use scrim_core::{AccessibilityClient, ElementFilter};
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct GetElementsArgs {
        pub filter: String,
    }

    /// Get UI elements from the focused window
    #[tauri::command]
    pub async fn get_elements(args: GetElementsArgs) -> Result<Vec<scrim_core::UIElement>, String> {
        #[cfg(target_os = "macos")]
        {
            use scrim_macos::MacOSAccessibilityClient;

            let client = MacOSAccessibilityClient::new();

            if !client.is_trusted() {
                return Err("Accessibility permissions not granted".to_string());
            }

            let filter = match args.filter.as_str() {
                "all" => ElementFilter::All,
                "interactive" => ElementFilter::HasActions,
                _ => ElementFilter::default(),
            };

            log::info!(
                "Getting interactive elements with filter: {:?}",
                args.filter
            );

            let result = client.get_interactive_elements(filter).await;

            match &result {
                Ok(elements) => log::info!("Found {} interactive elements", elements.len()),
                Err(e) => log::error!("Failed to get interactive elements: {:?}", e),
            }

            result.map_err(|e| e.to_string())
        }

        #[cfg(not(target_os = "macos"))]
        Err("Not implemented on this platform".to_string())
    }

    /// Perform an action on a UI element
    #[tauri::command]
    pub async fn perform_action(element_id: String, action: String) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            use scrim_macos::MacOSAccessibilityClient;

            let client = MacOSAccessibilityClient::new();
            client
                .perform_action(&element_id, &action)
                .await
                .map_err(|e| e.to_string())
        }

        #[cfg(not(target_os = "macos"))]
        Err("Not implemented on this platform".to_string())
    }

    /// Set click-through behavior
    #[tauri::command]
    pub async fn set_click_through(window: tauri::Window, ignore: bool) -> Result<(), String> {
        window
            .set_ignore_cursor_events(ignore)
            .map_err(|e| e.to_string())?;
        log::info!("Click-through set to: {}", ignore);

        // Update overlay active state for global keyboard handling
        OVERLAY_ACTIVE.store(!ignore, Ordering::Relaxed);
        log::info!("Overlay active state: {}", !ignore);

        Ok(())
    }

    /// Check accessibility permissions
    #[tauri::command]
    pub fn check_permissions() -> bool {
        #[cfg(target_os = "macos")]
        {
            use scrim_macos::permissions;
            permissions::is_trusted()
        }

        #[cfg(not(target_os = "macos"))]
        true
    }

    /// Request accessibility permissions (opens system dialog)
    #[tauri::command]
    pub fn request_permissions() -> bool {
        #[cfg(target_os = "macos")]
        {
            use scrim_macos::permissions;
            permissions::request_trust()
        }

        #[cfg(not(target_os = "macos"))]
        true
    }
}
