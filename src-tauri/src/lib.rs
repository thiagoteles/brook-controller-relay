/// Brook Controller Relay — Tauri application entry point.
///
/// Exposes Tauri commands for the frontend to:
/// - Start/stop the HID relay
/// - Manage configuration and profiles
/// - Query available keycodes

mod config;
mod hid;
mod keycodes;
mod relay;

use std::sync::Mutex;
use tauri::{Manager, State};

// ── Application State ────────────────────────────────────────────────

struct AppState {
    config: Mutex<config::AppConfig>,
    hid_manager: Mutex<Option<hid::HidManager>>,
}

// ── Tauri Commands ───────────────────────────────────────────────────

/// Get the current configuration.
#[tauri::command]
fn get_config(state: State<AppState>) -> Result<config::AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// Save the full configuration.
#[tauri::command]
fn save_config_cmd(
    state: State<AppState>,
    new_config: config::AppConfig,
) -> Result<(), String> {
    config::save_config(&new_config)?;

    // Update live HID mappings if running
    if let Ok(guard) = state.hid_manager.lock() {
        if let Some(ref mgr) = *guard {
            if let Some(profile) = new_config.profiles.get(&new_config.active_profile) {
                mgr.update_mappings(profile);
            }
        }
    }

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    Ok(())
}

/// List all profile names.
#[tauri::command]
fn list_profiles(state: State<AppState>) -> Result<Vec<String>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.profiles.keys().cloned().collect())
}

/// Load a specific profile by name.
#[tauri::command]
fn load_profile(
    state: State<AppState>,
    name: String,
) -> Result<config::Profile, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    config.profiles.get(&name)
        .cloned()
        .ok_or_else(|| format!("Profile '{}' not found", name))
}

/// Save/create a profile.
#[tauri::command]
fn save_profile(
    state: State<AppState>,
    name: String,
    profile: config::Profile,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.profiles.insert(name, profile);
    config::save_config(&config)?;
    Ok(())
}

/// Delete a profile.
#[tauri::command]
fn delete_profile(
    state: State<AppState>,
    name: String,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    if config.profiles.len() <= 1 {
        return Err("Cannot delete the last profile".into());
    }
    if config.active_profile == name {
        return Err("Cannot delete the active profile. Switch to another profile first.".into());
    }
    config.profiles.remove(&name);
    config::save_config(&config)?;
    Ok(())
}

/// Get all available key entries for the key picker UI.
#[tauri::command]
fn get_keycodes() -> Vec<keycodes::KeyEntry> {
    keycodes::all_keys()
}

/// List all connected HID devices for the device picker.
#[tauri::command]
fn list_devices() -> Vec<hid::HidDeviceInfo> {
    hid::list_hid_devices()
}

/// Start the HID relay. Begins listening for the controller and optionally
/// injecting keyboard events.
#[tauri::command]
fn start_relay(
    app_handle: tauri::AppHandle,
    state: State<AppState>,
    relay_active: bool,
) -> Result<(), String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;

    let vid = i32::from_str_radix(
        config.device.vid.trim_start_matches("0x"),
        16,
    ).map_err(|e| format!("Invalid VID: {}", e))?;

    let pid = i32::from_str_radix(
        config.device.pid.trim_start_matches("0x"),
        16,
    ).map_err(|e| format!("Invalid PID: {}", e))?;

    let profile = config.profiles.get(&config.active_profile)
        .ok_or("Active profile not found")?
        .clone();

    let seize = config.device.seize;

    drop(config); // Release lock before starting HID

    let mut hid_guard = state.hid_manager.lock().map_err(|e| e.to_string())?;

    // Stop existing manager if running
    if let Some(ref mut mgr) = *hid_guard {
        mgr.stop();
    }

    let manager = hid::HidManager::start(
        app_handle,
        vid,
        pid,
        seize,
        &profile,
        relay_active,
    );
    *hid_guard = Some(manager);

    Ok(())
}

/// Stop the HID relay.
#[tauri::command]
fn stop_relay(state: State<AppState>) -> Result<(), String> {
    let mut hid_guard = state.hid_manager.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut mgr) = *hid_guard {
        mgr.stop();
    }
    *hid_guard = None;
    Ok(())
}

/// Toggle keyboard injection on/off (without stopping HID listening).
#[tauri::command]
fn set_relay_active(
    state: State<AppState>,
    active: bool,
) -> Result<(), String> {
    let hid_guard = state.hid_manager.lock().map_err(|e| e.to_string())?;
    if let Some(ref mgr) = *hid_guard {
        mgr.set_relay_active(active);
    }
    Ok(())
}

// ── Tauri App Setup ──────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = config::load_config();

    let auto_start = config.auto_start;

    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(config),
            hid_manager: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config_cmd,
            list_profiles,
            load_profile,
            save_profile,
            delete_profile,
            get_keycodes,
            list_devices,
            start_relay,
            stop_relay,
            set_relay_active,
        ])
        .setup(move |app| {
            if auto_start {
                let handle = app.handle().clone();
                // Spawn so it doesn't block window creation
                std::thread::spawn(move || {
                    // Small delay to let the frontend connect its event listeners
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    let state: State<AppState> = handle.state();
                    let config_guard = state.config.lock().unwrap();

                    let vid = i32::from_str_radix(
                        config_guard.device.vid.trim_start_matches("0x"), 16,
                    ).unwrap_or(0x0f0d);
                    let pid = i32::from_str_radix(
                        config_guard.device.pid.trim_start_matches("0x"), 16,
                    ).unwrap_or(0x0202);
                    let seize = config_guard.device.seize;
                    let profile = config_guard.profiles
                        .get(&config_guard.active_profile)
                        .cloned()
                        .unwrap_or_else(config::default_sf6_profile);

                    drop(config_guard);

                    let manager = hid::HidManager::start(
                        handle.clone(), vid, pid, seize, &profile, true,
                    );

                    let mut hid_guard = state.hid_manager.lock().unwrap();
                    *hid_guard = Some(manager);

                    println!("🚀 Auto-started relay");
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
