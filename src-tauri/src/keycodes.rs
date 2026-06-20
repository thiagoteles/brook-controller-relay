#![allow(dead_code)]
/// macOS Virtual Key Code definitions.
///
/// Maps physical key positions to their macOS CGKeyCode values.
/// These are hardware key codes, not character codes — they represent
/// the physical key regardless of keyboard layout.

use serde::{Deserialize, Serialize};

/// A named keycode entry with its macOS virtual key code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    /// Human-readable name (e.g., "A", "Space", "Escape")
    pub name: String,
    /// macOS CGKeyCode value
    pub code: u16,
    /// Display label for the UI (e.g., "⎋" for Escape, "⇥" for Tab)
    pub display: String,
}

/// Sentinel value meaning "no key assigned"
pub const KEY_NONE: u16 = 0xFF;

// ── Individual key code constants ────────────────────────────────────
pub const KEY_A: u16 = 0x00;
pub const KEY_S: u16 = 0x01;
pub const KEY_D: u16 = 0x02;
pub const KEY_F: u16 = 0x03;
pub const KEY_H: u16 = 0x04;
pub const KEY_G: u16 = 0x05;
pub const KEY_Z: u16 = 0x06;
pub const KEY_X: u16 = 0x07;
pub const KEY_C: u16 = 0x08;
pub const KEY_V: u16 = 0x09;
pub const KEY_B: u16 = 0x0B;
pub const KEY_Q: u16 = 0x0C;
pub const KEY_W: u16 = 0x0D;
pub const KEY_E: u16 = 0x0E;
pub const KEY_R: u16 = 0x0F;
pub const KEY_Y: u16 = 0x10;
pub const KEY_T: u16 = 0x11;
pub const KEY_1: u16 = 0x12;
pub const KEY_2: u16 = 0x13;
pub const KEY_3: u16 = 0x14;
pub const KEY_4: u16 = 0x15;
pub const KEY_6: u16 = 0x16;
pub const KEY_5: u16 = 0x17;
pub const KEY_EQUAL: u16 = 0x18;
pub const KEY_9: u16 = 0x19;
pub const KEY_7: u16 = 0x1A;
pub const KEY_MINUS: u16 = 0x1B;
pub const KEY_8: u16 = 0x1C;
pub const KEY_0: u16 = 0x1D;
pub const KEY_RIGHT_BRACKET: u16 = 0x1E;
pub const KEY_O: u16 = 0x1F;
pub const KEY_U: u16 = 0x20;
pub const KEY_LEFT_BRACKET: u16 = 0x21;
pub const KEY_I: u16 = 0x22;
pub const KEY_P: u16 = 0x23;
pub const KEY_RETURN: u16 = 0x24;
pub const KEY_L: u16 = 0x25;
pub const KEY_J: u16 = 0x26;
pub const KEY_QUOTE: u16 = 0x27;
pub const KEY_K: u16 = 0x28;
pub const KEY_SEMICOLON: u16 = 0x29;
pub const KEY_BACKSLASH: u16 = 0x2A;
pub const KEY_COMMA: u16 = 0x2B;
pub const KEY_SLASH: u16 = 0x2C;
pub const KEY_N: u16 = 0x2D;
pub const KEY_M: u16 = 0x2E;
pub const KEY_PERIOD: u16 = 0x2F;
pub const KEY_TAB: u16 = 0x30;
pub const KEY_SPACE: u16 = 0x31;
pub const KEY_GRAVE: u16 = 0x32;
pub const KEY_DELETE: u16 = 0x33;
pub const KEY_ESCAPE: u16 = 0x35;
pub const KEY_F5: u16 = 0x60;
pub const KEY_F6: u16 = 0x61;
pub const KEY_F7: u16 = 0x62;
pub const KEY_F3: u16 = 0x63;
pub const KEY_F8: u16 = 0x64;
pub const KEY_F9: u16 = 0x65;
pub const KEY_F11: u16 = 0x67;
pub const KEY_F13: u16 = 0x69;
pub const KEY_F14: u16 = 0x6B;
pub const KEY_F10: u16 = 0x6D;
pub const KEY_F12: u16 = 0x6F;
pub const KEY_F15: u16 = 0x71;
pub const KEY_F4: u16 = 0x76;
pub const KEY_F2: u16 = 0x78;
pub const KEY_F1: u16 = 0x7A;
pub const KEY_LEFT_ARROW: u16 = 0x7B;
pub const KEY_RIGHT_ARROW: u16 = 0x7C;
pub const KEY_DOWN_ARROW: u16 = 0x7D;
pub const KEY_UP_ARROW: u16 = 0x7E;

/// Returns the full table of available key mappings for the UI.
pub fn all_keys() -> Vec<KeyEntry> {
    vec![
        // Letters
        KeyEntry { name: "A".into(), code: KEY_A, display: "A".into() },
        KeyEntry { name: "B".into(), code: KEY_B, display: "B".into() },
        KeyEntry { name: "C".into(), code: KEY_C, display: "C".into() },
        KeyEntry { name: "D".into(), code: KEY_D, display: "D".into() },
        KeyEntry { name: "E".into(), code: KEY_E, display: "E".into() },
        KeyEntry { name: "F".into(), code: KEY_F, display: "F".into() },
        KeyEntry { name: "G".into(), code: KEY_G, display: "G".into() },
        KeyEntry { name: "H".into(), code: KEY_H, display: "H".into() },
        KeyEntry { name: "I".into(), code: KEY_I, display: "I".into() },
        KeyEntry { name: "J".into(), code: KEY_J, display: "J".into() },
        KeyEntry { name: "K".into(), code: KEY_K, display: "K".into() },
        KeyEntry { name: "L".into(), code: KEY_L, display: "L".into() },
        KeyEntry { name: "M".into(), code: KEY_M, display: "M".into() },
        KeyEntry { name: "N".into(), code: KEY_N, display: "N".into() },
        KeyEntry { name: "O".into(), code: KEY_O, display: "O".into() },
        KeyEntry { name: "P".into(), code: KEY_P, display: "P".into() },
        KeyEntry { name: "Q".into(), code: KEY_Q, display: "Q".into() },
        KeyEntry { name: "R".into(), code: KEY_R, display: "R".into() },
        KeyEntry { name: "S".into(), code: KEY_S, display: "S".into() },
        KeyEntry { name: "T".into(), code: KEY_T, display: "T".into() },
        KeyEntry { name: "U".into(), code: KEY_U, display: "U".into() },
        KeyEntry { name: "V".into(), code: KEY_V, display: "V".into() },
        KeyEntry { name: "W".into(), code: KEY_W, display: "W".into() },
        KeyEntry { name: "X".into(), code: KEY_X, display: "X".into() },
        KeyEntry { name: "Y".into(), code: KEY_Y, display: "Y".into() },
        KeyEntry { name: "Z".into(), code: KEY_Z, display: "Z".into() },
        // Numbers
        KeyEntry { name: "0".into(), code: KEY_0, display: "0".into() },
        KeyEntry { name: "1".into(), code: KEY_1, display: "1".into() },
        KeyEntry { name: "2".into(), code: KEY_2, display: "2".into() },
        KeyEntry { name: "3".into(), code: KEY_3, display: "3".into() },
        KeyEntry { name: "4".into(), code: KEY_4, display: "4".into() },
        KeyEntry { name: "5".into(), code: KEY_5, display: "5".into() },
        KeyEntry { name: "6".into(), code: KEY_6, display: "6".into() },
        KeyEntry { name: "7".into(), code: KEY_7, display: "7".into() },
        KeyEntry { name: "8".into(), code: KEY_8, display: "8".into() },
        KeyEntry { name: "9".into(), code: KEY_9, display: "9".into() },
        // Special keys
        KeyEntry { name: "Space".into(), code: KEY_SPACE, display: "␣".into() },
        KeyEntry { name: "Return".into(), code: KEY_RETURN, display: "⏎".into() },
        KeyEntry { name: "Tab".into(), code: KEY_TAB, display: "⇥".into() },
        KeyEntry { name: "Escape".into(), code: KEY_ESCAPE, display: "⎋".into() },
        KeyEntry { name: "Delete".into(), code: KEY_DELETE, display: "⌫".into() },
        // Symbols
        KeyEntry { name: "Minus".into(), code: KEY_MINUS, display: "-".into() },
        KeyEntry { name: "Equal".into(), code: KEY_EQUAL, display: "=".into() },
        KeyEntry { name: "LeftBracket".into(), code: KEY_LEFT_BRACKET, display: "[".into() },
        KeyEntry { name: "RightBracket".into(), code: KEY_RIGHT_BRACKET, display: "]".into() },
        KeyEntry { name: "Semicolon".into(), code: KEY_SEMICOLON, display: ";".into() },
        KeyEntry { name: "Quote".into(), code: KEY_QUOTE, display: "'".into() },
        KeyEntry { name: "Comma".into(), code: KEY_COMMA, display: ",".into() },
        KeyEntry { name: "Period".into(), code: KEY_PERIOD, display: ".".into() },
        KeyEntry { name: "Slash".into(), code: KEY_SLASH, display: "/".into() },
        KeyEntry { name: "Backslash".into(), code: KEY_BACKSLASH, display: "\\".into() },
        KeyEntry { name: "Grave".into(), code: KEY_GRAVE, display: "`".into() },
        // Arrow keys
        KeyEntry { name: "UpArrow".into(), code: KEY_UP_ARROW, display: "↑".into() },
        KeyEntry { name: "DownArrow".into(), code: KEY_DOWN_ARROW, display: "↓".into() },
        KeyEntry { name: "LeftArrow".into(), code: KEY_LEFT_ARROW, display: "←".into() },
        KeyEntry { name: "RightArrow".into(), code: KEY_RIGHT_ARROW, display: "→".into() },
        // Function keys
        KeyEntry { name: "F1".into(), code: KEY_F1, display: "F1".into() },
        KeyEntry { name: "F2".into(), code: KEY_F2, display: "F2".into() },
        KeyEntry { name: "F3".into(), code: KEY_F3, display: "F3".into() },
        KeyEntry { name: "F4".into(), code: KEY_F4, display: "F4".into() },
        KeyEntry { name: "F5".into(), code: KEY_F5, display: "F5".into() },
        KeyEntry { name: "F6".into(), code: KEY_F6, display: "F6".into() },
        KeyEntry { name: "F7".into(), code: KEY_F7, display: "F7".into() },
        KeyEntry { name: "F8".into(), code: KEY_F8, display: "F8".into() },
        KeyEntry { name: "F9".into(), code: KEY_F9, display: "F9".into() },
        KeyEntry { name: "F10".into(), code: KEY_F10, display: "F10".into() },
        KeyEntry { name: "F11".into(), code: KEY_F11, display: "F11".into() },
        KeyEntry { name: "F12".into(), code: KEY_F12, display: "F12".into() },
        // None
        KeyEntry { name: "None".into(), code: KEY_NONE, display: "—".into() },
    ]
}

/// Look up a key name by its macOS virtual key code.
pub fn key_name(code: u16) -> String {
    if code == KEY_NONE {
        return "None".into();
    }
    all_keys()
        .iter()
        .find(|k| k.code == code)
        .map(|k| k.name.clone())
        .unwrap_or_else(|| format!("0x{:02X}", code))
}

/// Look up a key code by its name. Returns KEY_NONE if not found.
pub fn key_code(name: &str) -> u16 {
    if name == "None" || name.is_empty() {
        return KEY_NONE;
    }
    all_keys()
        .iter()
        .find(|k| k.name.eq_ignore_ascii_case(name))
        .map(|k| k.code)
        .unwrap_or(KEY_NONE)
}
