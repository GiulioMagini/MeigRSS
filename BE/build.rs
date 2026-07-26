// build.rs
// Embeds the application icon into the Windows executable.
// On other platforms this script does nothing.

fn main() {
    // The frontend is embedded with include_dir! at compile time, so cargo
    // must recompile when the built static files change
    println!("cargo:rerun-if-changed=static");
    
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/icon.ico");
        // Metadata is automatically pulled from Cargo.toml [package] section
        res.compile().unwrap();
    }
}