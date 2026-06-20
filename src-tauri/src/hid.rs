#![allow(dead_code)] // FFI functions and keycode constants exist for completeness

/// IOKit HID device management for macOS.
///
/// Opens the Brook Gen-5X (or compatible) controller via IOKit,
/// reads raw 8-byte HID reports, and emits Tauri events for the frontend.
/// Also feeds reports into the relay engine for keyboard injection.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::config::{Profile, resolve_keycodes};
use crate::relay;

// ── IOKit FFI bindings ───────────────────────────────────────────────
// We use raw FFI because the hidapi crate doesn't support kIOHIDOptionsTypeSeizeDevice.

#[allow(non_camel_case_types)]
type IOHIDManagerRef = *mut c_void;
#[allow(non_camel_case_types)]
type IOHIDDeviceRef = *mut c_void;
#[allow(non_camel_case_types)]
type IOReturn = i32;
#[allow(non_camel_case_types)]
type CFIndex = isize;
#[allow(non_camel_case_types)]
type CFRunLoopRef = *mut c_void;
#[allow(non_camel_case_types)]
type CFStringRef = *const c_void;
#[allow(non_camel_case_types)]
type CFAllocatorRef = *const c_void;
#[allow(non_camel_case_types)]
type CFDictionaryRef = *mut c_void;
#[allow(non_camel_case_types)]
type CFMutableDictionaryRef = *mut c_void;
#[allow(non_camel_case_types)]
type CFNumberRef = *const c_void;
#[allow(non_camel_case_types)]
type CFRunLoopMode = CFStringRef;
#[allow(non_camel_case_types)]
type IOHIDReportType = u32;
#[allow(non_camel_case_types)]
type Boolean = u8;

const K_CF_ALLOCATOR_DEFAULT: CFAllocatorRef = std::ptr::null();
const K_IOHID_OPTIONS_TYPE_NONE: u32 = 0x00;
const K_IOHID_OPTIONS_TYPE_SEIZE_DEVICE: u32 = 0x01;
const K_IO_RETURN_SUCCESS: IOReturn = 0;
const K_CF_NUMBER_INT_TYPE: u32 = 9; // kCFNumberIntType

type IOHIDDeviceCallback = extern "C" fn(
    context: *mut c_void,
    result: IOReturn,
    sender: *mut c_void,
    device: IOHIDDeviceRef,
);

type IOHIDReportCallback = extern "C" fn(
    context: *mut c_void,
    result: IOReturn,
    sender: *mut c_void,
    report_type: IOHIDReportType,
    report_id: u32,
    report: *mut u8,
    report_length: CFIndex,
);

extern "C" {
    fn IOHIDManagerCreate(allocator: CFAllocatorRef, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(manager: IOHIDManagerRef, matching: CFDictionaryRef);
    fn IOHIDManagerRegisterDeviceMatchingCallback(
        manager: IOHIDManagerRef,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    );
    fn IOHIDManagerRegisterDeviceRemovalCallback(
        manager: IOHIDManagerRef,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    );
    fn IOHIDManagerScheduleWithRunLoop(
        manager: IOHIDManagerRef,
        run_loop: CFRunLoopRef,
        mode: CFRunLoopMode,
    );
    fn IOHIDManagerUnscheduleFromRunLoop(
        manager: IOHIDManagerRef,
        run_loop: CFRunLoopRef,
        mode: CFRunLoopMode,
    );
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> IOReturn;
    fn IOHIDManagerClose(manager: IOHIDManagerRef, options: u32) -> IOReturn;

    fn IOHIDDeviceRegisterInputReportCallback(
        device: IOHIDDeviceRef,
        report: *mut u8,
        report_length: CFIndex,
        callback: IOHIDReportCallback,
        context: *mut c_void,
    );

    fn IOHIDDeviceGetProperty(device: IOHIDDeviceRef, key: CFStringRef) -> *const c_void;

    fn CFDictionaryCreateMutable(
        allocator: CFAllocatorRef,
        capacity: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFMutableDictionaryRef;
    fn CFDictionarySetValue(
        dict: CFMutableDictionaryRef,
        key: *const c_void,
        value: *const c_void,
    );

    fn CFNumberCreate(
        allocator: CFAllocatorRef,
        the_type: u32,
        value_ptr: *const c_void,
    ) -> CFNumberRef;

    fn CFRelease(cf: *const c_void);

    fn CFRunLoopGetMain() -> CFRunLoopRef;
    fn CFRunLoopRunInMode(mode: CFRunLoopMode, seconds: f64, return_after_source_handled: Boolean) -> i32;
    fn CFRunLoopStop(rl: CFRunLoopRef);

    fn CFStringGetCStringPtr(string: CFStringRef, encoding: u32) -> *const u8;

    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    static kCFRunLoopDefaultMode: CFStringRef;
}

// IOKit HID property keys — we create these as CFStrings
fn cfstr(s: &str) -> CFStringRef {
    // Use the CoreFoundation function to create CFString from C string
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            c_str: *const u8,
            encoding: u32,
        ) -> CFStringRef;
    }
    let c_string = std::ffi::CString::new(s).unwrap();
    unsafe {
        CFStringCreateWithCString(K_CF_ALLOCATOR_DEFAULT, c_string.as_ptr() as *const u8, 0x08000100) // UTF8
    }
}

// ── Shared state ─────────────────────────────────────────────────────

/// State shared between the HID callbacks and the main thread.
struct HidState {
    app_handle: AppHandle,
    prev_report: [u8; 8],
    prev_wasd: [bool; 4],
    button_codes: Vec<(u16, u16)>,
    dpad_codes: [u16; 4],
    relay_active: bool,
}

/// Wrapper around a raw pointer so it can be sent across threads.
/// Safety: We ensure the pointer is only dereferenced while the HID thread
/// is alive and holds the owning allocation.
struct SendPtr(*mut HidState);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

// Global mutable state for IOKit callbacks (which require C function pointers)
static mut GLOBAL_HID_STATE: Option<*mut HidState> = None;
static mut GLOBAL_REPORT_BUF: [u8; 64] = [0u8; 64];

/// Event payloads sent to the frontend
#[derive(Clone, serde::Serialize)]
pub struct ButtonEvent {
    pub button: usize,
    pub pressed: bool,
    pub label: String,
}

#[derive(Clone, serde::Serialize)]
pub struct HatEvent {
    pub direction: u8,
    pub keys: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct DeviceEvent {
    pub name: String,
    pub connected: bool,
}

// ── Hat switch → direction keys ──────────────────────────────────────
struct HatDir {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
}

const HAT_WASD: [HatDir; 9] = [
    HatDir { w: true,  a: false, s: false, d: false }, // 0: Up
    HatDir { w: true,  a: false, s: false, d: true  }, // 1: Up-Right
    HatDir { w: false, a: false, s: false, d: true  }, // 2: Right
    HatDir { w: false, a: false, s: true,  d: true  }, // 3: Down-Right
    HatDir { w: false, a: false, s: true,  d: false }, // 4: Down
    HatDir { w: false, a: true,  s: true,  d: false }, // 5: Down-Left
    HatDir { w: false, a: true,  s: false, d: false }, // 6: Left
    HatDir { w: true,  a: true,  s: false, d: false }, // 7: Up-Left
    HatDir { w: false, a: false, s: false, d: false }, // 8: Neutral
];

// ── HID report processing ────────────────────────────────────────────

fn process_report(state: &mut HidState, report: &[u8]) {
    if report.len() < 8 {
        return;
    }

    // Parse current buttons
    let mut buttons = [false; 15];
    for i in 0..8 {
        buttons[i] = (report[0] >> i) & 1 == 1;
    }
    for i in 0..6 {
        buttons[8 + i] = (report[1] >> i) & 1 == 1;
    }
    buttons[14] = (report[2] >> 7) & 1 == 1;

    // Parse previous buttons
    let mut prev_buttons = [false; 15];
    for i in 0..8 {
        prev_buttons[i] = (state.prev_report[0] >> i) & 1 == 1;
    }
    for i in 0..6 {
        prev_buttons[8 + i] = (state.prev_report[1] >> i) & 1 == 1;
    }
    prev_buttons[14] = (state.prev_report[2] >> 7) & 1 == 1;

    // Parse hat
    let hat = std::cmp::min((report[2] & 0x0F) as usize, 8);

    // Emit button events on change
    for i in 0..15 {
        if buttons[i] != prev_buttons[i] {
            // Send keyboard events if relay is active
            if state.relay_active && i < state.button_codes.len() {
                let (primary, secondary) = state.button_codes[i];
                relay::post_key(primary, buttons[i]);
                relay::post_key(secondary, buttons[i]);
            }

            // Always send UI event
            let _ = state.app_handle.emit("button-pressed", ButtonEvent {
                button: i + 1,
                pressed: buttons[i],
                label: format!("btn{}", i + 1),
            });
        }
    }

    // Emit hat/WASD events on change
    let new_wasd = [
        HAT_WASD[hat].w,
        HAT_WASD[hat].a,
        HAT_WASD[hat].s,
        HAT_WASD[hat].d,
    ];

    let dpad_names = ["up", "left", "down", "right"];

    for i in 0..4 {
        if new_wasd[i] != state.prev_wasd[i] {
            if state.relay_active {
                relay::post_key(state.dpad_codes[i], new_wasd[i]);
            }
        }
    }

    // Send hat event to frontend
    if hat != std::cmp::min((state.prev_report[2] & 0x0F) as usize, 8) {
        let mut active_dirs = Vec::new();
        for (i, name) in dpad_names.iter().enumerate() {
            if new_wasd[i] {
                active_dirs.push(name.to_string());
            }
        }
        let _ = state.app_handle.emit("hat-changed", HatEvent {
            direction: hat as u8,
            keys: active_dirs,
        });
    }

    state.prev_wasd = new_wasd;
    state.prev_report.copy_from_slice(&report[..8]);
}

// ── IOKit C callbacks ────────────────────────────────────────────────

extern "C" fn input_report_callback(
    _context: *mut c_void,
    result: IOReturn,
    _sender: *mut c_void,
    _report_type: IOHIDReportType,
    _report_id: u32,
    report: *mut u8,
    report_length: CFIndex,
) {
    if result != K_IO_RETURN_SUCCESS || report_length < 8 {
        return;
    }

    unsafe {
        let buf = std::slice::from_raw_parts(report, report_length as usize);

        if let Some(state_ptr) = GLOBAL_HID_STATE {
            let state = &mut *state_ptr;
            // Skip if report unchanged
            if buf[..8] == state.prev_report[..] {
                return;
            }
            process_report(state, buf);
        }
    }
}

extern "C" fn device_matched_callback(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    device: IOHIDDeviceRef,
) {
    let name = unsafe {
        let product_key = cfstr("Product");
        let prop = IOHIDDeviceGetProperty(device, product_key);
        CFRelease(product_key);

        if !prop.is_null() {
            let cstr = CFStringGetCStringPtr(prop as CFStringRef, 0x08000100);
            if !cstr.is_null() {
                std::ffi::CStr::from_ptr(cstr as *const i8)
                    .to_string_lossy()
                    .to_string()
            } else {
                "Unknown Device".to_string()
            }
        } else {
            "Unknown Device".to_string()
        }
    };

    println!("✅ Device connected: {}", name);

    unsafe {
        #[allow(static_mut_refs)]
        let buf_ptr = GLOBAL_REPORT_BUF.as_mut_ptr();
        #[allow(static_mut_refs)]
        let buf_len = GLOBAL_REPORT_BUF.len() as CFIndex;
        IOHIDDeviceRegisterInputReportCallback(
            device,
            buf_ptr,
            buf_len,
            input_report_callback,
            std::ptr::null_mut(),
        );

        if let Some(state_ptr) = GLOBAL_HID_STATE {
            let state = &*state_ptr;
            let _ = state.app_handle.emit("device-status", DeviceEvent {
                name: name.clone(),
                connected: true,
            });
        }
    }
}

extern "C" fn device_removed_callback(
    _context: *mut c_void,
    _result: IOReturn,
    _sender: *mut c_void,
    _device: IOHIDDeviceRef,
) {
    println!("⚠️  Device disconnected.");

    unsafe {
        if let Some(state_ptr) = GLOBAL_HID_STATE {
            let state = &*state_ptr;
            let _ = state.app_handle.emit("device-status", DeviceEvent {
                name: String::new(),
                connected: false,
            });
        }
    }
}

// ── Device Enumeration ───────────────────────────────────────────────

// Additional FFI for device enumeration
#[allow(non_camel_case_types)]
type CFSetRef = *const c_void;
#[allow(non_camel_case_types)]
type CFTypeRef = *const c_void;

extern "C" {
    fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> CFSetRef;
    fn CFSetGetCount(set: CFSetRef) -> CFIndex;
    fn CFSetGetValues(set: CFSetRef, values: *mut CFTypeRef);
    fn CFNumberGetValue(number: CFNumberRef, the_type: u32, value_ptr: *mut c_void) -> Boolean;
}

/// Info about a connected HID device, for the frontend picker.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HidDeviceInfo {
    pub name: String,
    pub vid: String,
    pub pid: String,
}

/// Get the name of an HID device from its IOKit properties.
unsafe fn get_device_string_property(device: IOHIDDeviceRef, key: &str) -> String {
    let cf_key = cfstr(key);
    let prop = IOHIDDeviceGetProperty(device, cf_key);
    CFRelease(cf_key);

    if !prop.is_null() {
        let cstr = CFStringGetCStringPtr(prop as CFStringRef, 0x08000100);
        if !cstr.is_null() {
            return std::ffi::CStr::from_ptr(cstr as *const i8)
                .to_string_lossy()
                .to_string();
        }
    }
    String::new()
}

/// Get an integer property (VID or PID) from an HID device.
unsafe fn get_device_int_property(device: IOHIDDeviceRef, key: &str) -> i32 {
    let cf_key = cfstr(key);
    let prop = IOHIDDeviceGetProperty(device, cf_key);
    CFRelease(cf_key);

    if !prop.is_null() {
        let mut value: i32 = 0;
        CFNumberGetValue(
            prop as CFNumberRef,
            K_CF_NUMBER_INT_TYPE,
            &mut value as *mut i32 as *mut c_void,
        );
        return value;
    }
    0
}

/// List all connected HID devices visible to IOKit.
pub fn list_hid_devices() -> Vec<HidDeviceInfo> {
    let mut devices = Vec::new();

    unsafe {
        let mgr = IOHIDManagerCreate(K_CF_ALLOCATOR_DEFAULT, K_IOHID_OPTIONS_TYPE_NONE);
        if mgr.is_null() {
            return devices;
        }

        // Match ALL HID devices (null matching = everything)
        IOHIDManagerSetDeviceMatching(mgr, std::ptr::null_mut());

        // Open manager (read-only, no seize)
        let result = IOHIDManagerOpen(mgr, K_IOHID_OPTIONS_TYPE_NONE);
        if result != K_IO_RETURN_SUCCESS {
            CFRelease(mgr as *const c_void);
            return devices;
        }

        let device_set = IOHIDManagerCopyDevices(mgr);
        if !device_set.is_null() {
            let count = CFSetGetCount(device_set);
            if count > 0 {
                let mut device_ptrs = vec![std::ptr::null() as CFTypeRef; count as usize];
                CFSetGetValues(device_set, device_ptrs.as_mut_ptr());

                for ptr in &device_ptrs {
                    let device = *ptr as IOHIDDeviceRef;
                    let name = get_device_string_property(device, "Product");
                    let vid = get_device_int_property(device, "VendorID");
                    let pid = get_device_int_property(device, "ProductID");

                    // Skip devices with no name or VID/PID
                    if name.is_empty() || (vid == 0 && pid == 0) {
                        continue;
                    }

                    devices.push(HidDeviceInfo {
                        name,
                        vid: format!("0x{:04x}", vid),
                        pid: format!("0x{:04x}", pid),
                    });
                }
            }
            CFRelease(device_set);
        }

        IOHIDManagerClose(mgr, K_IOHID_OPTIONS_TYPE_NONE);
        CFRelease(mgr as *const c_void);
    }

    // Sort by name and deduplicate
    devices.sort_by(|a, b| a.name.cmp(&b.name));
    devices.dedup_by(|a, b| a.vid == b.vid && a.pid == b.pid);

    devices
}

// ── Public API ───────────────────────────────────────────────────────

/// Manages the IOKit HID manager lifecycle on a background thread.
pub struct HidManager {
    running: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    state_ptr: Arc<Mutex<Option<SendPtr>>>,
}

// HidManager is safe to Send because all raw pointer access goes through
// the SendPtr wrapper which is guarded by a Mutex.
unsafe impl Send for HidManager {}
unsafe impl Sync for HidManager {}

impl HidManager {
    /// Start the HID manager on a background thread.
    pub fn start(
        app_handle: AppHandle,
        vid: i32,
        pid: i32,
        seize: bool,
        profile: &Profile,
        relay_active: bool,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let state_ptr: Arc<Mutex<Option<SendPtr>>> = Arc::new(Mutex::new(None));
        let state_ptr_clone = state_ptr.clone();

        let (button_codes, dpad_codes) = resolve_keycodes(profile);

        let thread = std::thread::spawn(move || {
            // Create state on the heap
            let state = Box::new(HidState {
                app_handle,
                prev_report: [0, 0, 8, 128, 128, 128, 128, 0],
                prev_wasd: [false; 4],
                button_codes,
                dpad_codes,
                relay_active,
            });
            let state_raw = Box::into_raw(state);

            // Store globally for IOKit callbacks
            unsafe {
                GLOBAL_HID_STATE = Some(state_raw);
            }
            if let Ok(mut guard) = state_ptr_clone.lock() {
                *guard = Some(SendPtr(state_raw));
            }

            unsafe {
                let mgr = IOHIDManagerCreate(K_CF_ALLOCATOR_DEFAULT, K_IOHID_OPTIONS_TYPE_NONE);
                if mgr.is_null() {
                    eprintln!("Failed to create HID Manager");
                    return;
                }

                // Build matching dictionary for VID/PID
                let match_dict = CFDictionaryCreateMutable(
                    K_CF_ALLOCATOR_DEFAULT,
                    2,
                    &kCFTypeDictionaryKeyCallBacks as *const c_void,
                    &kCFTypeDictionaryValueCallBacks as *const c_void,
                );

                let vid_key = cfstr("VendorID");
                let pid_key = cfstr("ProductID");
                let vid_num = CFNumberCreate(
                    K_CF_ALLOCATOR_DEFAULT,
                    K_CF_NUMBER_INT_TYPE,
                    &vid as *const i32 as *const c_void,
                );
                let pid_num = CFNumberCreate(
                    K_CF_ALLOCATOR_DEFAULT,
                    K_CF_NUMBER_INT_TYPE,
                    &pid as *const i32 as *const c_void,
                );

                CFDictionarySetValue(match_dict, vid_key, vid_num);
                CFDictionarySetValue(match_dict, pid_key, pid_num);

                IOHIDManagerSetDeviceMatching(mgr, match_dict);

                CFRelease(vid_key);
                CFRelease(pid_key);
                CFRelease(vid_num);
                CFRelease(pid_num);
                CFRelease(match_dict as *const c_void);

                // Register callbacks
                IOHIDManagerRegisterDeviceMatchingCallback(
                    mgr,
                    device_matched_callback,
                    std::ptr::null_mut(),
                );
                IOHIDManagerRegisterDeviceRemovalCallback(
                    mgr,
                    device_removed_callback,
                    std::ptr::null_mut(),
                );

                // Schedule with run loop
                let run_loop = CFRunLoopGetMain();
                IOHIDManagerScheduleWithRunLoop(mgr, run_loop, kCFRunLoopDefaultMode);

                // Open with seize option
                let options = if seize {
                    K_IOHID_OPTIONS_TYPE_SEIZE_DEVICE
                } else {
                    K_IOHID_OPTIONS_TYPE_NONE
                };
                let result = IOHIDManagerOpen(mgr, options);
                if result != K_IO_RETURN_SUCCESS {
                    eprintln!("Failed to open HID Manager (0x{:08x}), trying without seize...", result);
                    let result2 = IOHIDManagerOpen(mgr, K_IOHID_OPTIONS_TYPE_NONE);
                    if result2 != K_IO_RETURN_SUCCESS {
                        eprintln!("Failed to open HID Manager even without seize (0x{:08x})", result2);
                        CFRelease(mgr as *const c_void);
                        return;
                    }
                }

                println!("⏳ HID Manager started. VID=0x{:04x} PID=0x{:04x} Seize={}", vid, pid, seize);

                // Run loop until stopped
                while running_clone.load(Ordering::Relaxed) {
                    CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.1, 0);
                }

                // Cleanup
                IOHIDManagerUnscheduleFromRunLoop(mgr, run_loop, kCFRunLoopDefaultMode);
                IOHIDManagerClose(mgr, K_IOHID_OPTIONS_TYPE_NONE);
                CFRelease(mgr as *const c_void);

                // Free state
                GLOBAL_HID_STATE = None;
                let _ = Box::from_raw(state_raw);
            }
        });

        HidManager {
            running,
            thread: Some(thread),
            state_ptr,
        }
    }

    /// Stop the HID manager and wait for the thread to finish.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    /// Update the relay active state (start/stop sending keyboard events).
    pub fn set_relay_active(&self, active: bool) {
        if let Ok(guard) = self.state_ptr.lock() {
            if let Some(ref send_ptr) = *guard {
                let ptr = send_ptr.0;
                unsafe {
                    (*ptr).relay_active = active;
                }
            }
        }
    }

    /// Update button mappings live (without restarting HID).
    pub fn update_mappings(&self, profile: &Profile) {
        let (button_codes, dpad_codes) = resolve_keycodes(profile);
        if let Ok(guard) = self.state_ptr.lock() {
            if let Some(ref send_ptr) = *guard {
                let ptr = send_ptr.0;
                unsafe {
                    (*ptr).button_codes = button_codes;
                    (*ptr).dpad_codes = dpad_codes;
                }
            }
        }
    }
}

impl Drop for HidManager {
    fn drop(&mut self) {
        self.stop();
    }
}
