// build.rs
//
// NOTE: this is a leftover from before this project switched to the real
// published `mupdf` crate. `mupdf` (via `mupdf-sys`) vendors and compiles
// MuPDF from source itself — it does not link against a system-installed
// MuPDF, so there is no "system MuPDF" for this file to help locate. The
// env vars below are simply never read by anything in the build, making
// this whole file a harmless no-op kept only so `MUPDF_LIB_DIR`/
// `MUPDF_INCLUDE_DIR` don't silently do nothing if someone sets them out of
// habit from other MuPDF bindings. It can be deleted entirely with no
// effect on the build.

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
