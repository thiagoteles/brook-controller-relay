fn main() {
    // Link macOS frameworks for HID access and keyboard injection
    println!("cargo:rustc-link-lib=framework=IOKit");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");

    tauri_build::build()
}
