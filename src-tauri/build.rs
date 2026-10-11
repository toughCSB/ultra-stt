fn main() {
    if std::env::var("PROFILE").as_deref() == Ok("release") && tauri_build::is_dev() {
        panic!("Release builds must embed the frontend. Use npm run tauri -- build --no-bundle or cargo build --release --features tauri/custom-protocol.");
    }
    println!("cargo:rerun-if-changed=icons");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=Speech");

    #[cfg(target_os = "windows")]
    println!(
        "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
    );

    tauri_build::build()
}
