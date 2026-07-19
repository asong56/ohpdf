// build.rs
// Helps locate MuPDF on platforms where it isn't in the default search path.

fn main() {
    // Allow overriding MuPDF lib/include paths via environment variables.
    if let Ok(lib_dir) = std::env::var("MUPDF_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    }

    // On macOS with Homebrew, MuPDF is typically in /opt/homebrew or /usr/local.
    #[cfg(target_os = "macos")]
    {
        for prefix in &["/opt/homebrew", "/usr/local"] {
            let lib = format!("{}/lib", prefix);
            if std::path::Path::new(&lib).exists() {
                println!("cargo:rustc-link-search=native={}", lib);
            }
        }
    }

    println!("cargo:rerun-if-env-changed=MUPDF_LIB_DIR");
    println!("cargo:rerun-if-env-changed=MUPDF_INCLUDE_DIR");
}
