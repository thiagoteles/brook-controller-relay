/// CGEvent keyboard injection for macOS.
///
/// Posts synthetic keyboard events via Core Graphics,
/// allowing the controller to act as a virtual keyboard.

use crate::keycodes::KEY_NONE;

// ── Core Graphics FFI ────────────────────────────────────────────────

#[allow(non_camel_case_types)]
type CGEventRef = *mut std::ffi::c_void;
#[allow(non_camel_case_types)]
type CGEventSourceRef = *mut std::ffi::c_void;
#[allow(non_camel_case_types)]
type CGKeyCode = u16;

const CG_SESSION_EVENT_TAP: u32 = 1; // kCGSessionEventTap

extern "C" {
    fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtual_key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;
    fn CGEventPost(tap: u32, event: CGEventRef);
    fn CFRelease(cf: *const std::ffi::c_void);
}

/// Post a single keyboard key press or release event.
///
/// This is the direct Rust equivalent of the C `post_key` function.
/// Requires Accessibility permissions to work.
pub fn post_key(keycode: u16, down: bool) {
    if keycode == KEY_NONE {
        return;
    }

    unsafe {
        let event = CGEventCreateKeyboardEvent(
            std::ptr::null_mut(),
            keycode,
            down,
        );
        if !event.is_null() {
            CGEventPost(CG_SESSION_EVENT_TAP, event);
            CFRelease(event as *const std::ffi::c_void);
        }
    }
}
