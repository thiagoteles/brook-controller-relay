/// Configuration management for button mappings and profiles.
///
/// Persists user settings to ~/Library/Application Support/brook-controller-relay/config.json

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::keycodes;

/// A single button mapping: which keyboard keys to send when this HID button is pressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonMapping {
    /// Primary keyboard key name (e.g., "U", "J", "Escape")
    pub primary: String,
    /// Optional secondary key (for dual-key buttons like L+M + Tab)
    pub secondary: Option<String>,
    /// User-visible label (e.g., "Light Attack", "Parry")
    pub label: String,
}

/// D-pad key assignments (normally WASD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpadMapping {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
}

/// A complete mapping profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub buttons: HashMap<String, ButtonMapping>,
    pub dpad: DpadMapping,
}

/// Device identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub vid: String,
    pub pid: String,
    pub seize: bool,
}

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub active_profile: String,
    pub device: DeviceConfig,
    pub profiles: HashMap<String, Profile>,
    /// Start the relay automatically when the app opens.
    #[serde(default)]
    pub auto_start: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("SF6 Modern".to_string(), default_sf6_profile());

        AppConfig {
            active_profile: "SF6 Modern".to_string(),
            device: DeviceConfig {
                vid: "0x0f0d".to_string(),
                pid: "0x0202".to_string(),
                seize: true,
            },
            profiles,
            auto_start: false,
        }
    }
}

/// The SF6 Modern Controls default mapping that matches the original C relay.
pub fn default_sf6_profile() -> Profile {
    let mut buttons = HashMap::new();

    buttons.insert("btn1".into(), ButtonMapping {
        primary: "U".into(), secondary: None, label: "Light".into(),
    });
    buttons.insert("btn2".into(), ButtonMapping {
        primary: "J".into(), secondary: None, label: "Medium".into(),
    });
    buttons.insert("btn3".into(), ButtonMapping {
        primary: "K".into(), secondary: None, label: "Heavy".into(),
    });
    buttons.insert("btn4".into(), ButtonMapping {
        primary: "I".into(), secondary: None, label: "Special".into(),
    });
    buttons.insert("btn5".into(), ButtonMapping {
        primary: "Y".into(), secondary: Some("E".into()), label: "Impact / Tab→".into(),
    });
    buttons.insert("btn6".into(), ButtonMapping {
        primary: "H".into(), secondary: Some("Q".into()), label: "L+M / Tab←".into(),
    });
    buttons.insert("btn7".into(), ButtonMapping {
        primary: "O".into(), secondary: None, label: "Parry".into(),
    });
    buttons.insert("btn8".into(), ButtonMapping {
        primary: "L".into(), secondary: None, label: "Assist".into(),
    });
    buttons.insert("btn9".into(), ButtonMapping {
        primary: "None".into(), secondary: None, label: "Touch".into(),
    });
    buttons.insert("btn10".into(), ButtonMapping {
        primary: "F".into(), secondary: None, label: "Start".into(),
    });
    buttons.insert("btn11".into(), ButtonMapping {
        primary: "None".into(), secondary: None, label: "L3".into(),
    });
    buttons.insert("btn12".into(), ButtonMapping {
        primary: "None".into(), secondary: None, label: "R3".into(),
    });
    buttons.insert("btn13".into(), ButtonMapping {
        primary: "Escape".into(), secondary: None, label: "Home".into(),
    });
    buttons.insert("btn14".into(), ButtonMapping {
        primary: "Tab".into(), secondary: None, label: "Select".into(),
    });
    buttons.insert("btn15".into(), ButtonMapping {
        primary: "None".into(), secondary: None, label: "Extra".into(),
    });

    Profile {
        buttons,
        dpad: DpadMapping {
            up: "W".into(),
            down: "S".into(),
            left: "A".into(),
            right: "D".into(),
        },
    }
}

/// Get the config directory path.
fn config_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    home.join("Library")
        .join("Application Support")
        .join("brook-controller-relay")
}

/// Get the config file path.
fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Load config from disk, or return defaults.
pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(contents) => {
                match serde_json::from_str::<AppConfig>(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("⚠️  Config parse error, using defaults: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  Config read error, using defaults: {}", e);
            }
        }
    }
    AppConfig::default()
}

/// Save config to disk.
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create config dir: {}", e))?;

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;

    fs::write(config_path(), json)
        .map_err(|e| format!("Cannot write config: {}", e))?;

    Ok(())
}

/// Resolve a profile's button mapping to actual keycodes for the relay engine.
/// Returns a vec of (primary_keycode, secondary_keycode) for buttons 0..14,
/// plus a (up, down, left, right) tuple for the d-pad.
pub fn resolve_keycodes(profile: &Profile) -> (Vec<(u16, u16)>, [u16; 4]) {
    let mut button_codes = Vec::with_capacity(15);
    for i in 1..=15 {
        let key = format!("btn{}", i);
        if let Some(mapping) = profile.buttons.get(&key) {
            let primary = keycodes::key_code(&mapping.primary);
            let secondary = mapping.secondary.as_deref()
                .map(keycodes::key_code)
                .unwrap_or(keycodes::KEY_NONE);
            button_codes.push((primary, secondary));
        } else {
            button_codes.push((keycodes::KEY_NONE, keycodes::KEY_NONE));
        }
    }

    let dpad = [
        keycodes::key_code(&profile.dpad.up),
        keycodes::key_code(&profile.dpad.left),
        keycodes::key_code(&profile.dpad.down),
        keycodes::key_code(&profile.dpad.right),
    ];

    (button_codes, dpad)
}
