use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, AppHandle, RunEvent,
};

#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_deep_link::DeepLinkExt;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}


#[tauri::command]
async fn exchange_token(backend_url: String, token: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/auth/desktop-exchange", backend_url);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("ngrok-skip-browser-warning", "true")
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status().as_u16();
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if status != 200 {
        let error = body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("Exchange failed ({}): {}", status, error));
    }

    Ok(body)
}

// Helper function to show the window
fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// Helper function to toggle window visibility
fn toggle_window_visibility(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        match window.is_visible() {
            Ok(true) => {
                let _ = window.hide();
            }
            Ok(false) => {
                show_window(app);
            }
            Err(_) => {
                // If we can't determine visibility, try to show it
                show_window(app);
            }
        }
    } else {
        eprintln!("Could not find window 'main'");
    }
}

// Setup system tray with menu
fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_hide_item = MenuItem::with_id(app, "show_hide", "Show/Hide Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_hide_item, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("AlphaHuman")
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "show_hide" => {
                    toggle_window_visibility(app);
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let app = tray.app_handle();
                    toggle_window_visibility(app);
                }
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => {
                    let app = tray.app_handle();
                    show_window(app);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--flag1", "--flag2"]),
        ))
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                app.deep_link().register_all()?;
            }

            // Setup system tray (desktop only)
            #[cfg(desktop)]
            {
                setup_tray(app.handle())?;
            }

            // macOS-specific: Handle window close event to minimize to tray
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let app_handle = app.handle().clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            // Prevent the window from closing, hide it instead
                            api.prevent_close();
                            if let Some(win) = app_handle.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, exchange_token])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Handle macOS Dock icon click (reopen event)
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen { .. } = event {
                show_window(app_handle);
            }
        });
}
