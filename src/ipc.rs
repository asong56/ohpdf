use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use wry::WebView;

use crate::pdf;

/// Every message from JS is wrapped with a client-generated `id` so that
/// responses can be matched to the right pending Promise on the JS side,
/// even if two IPC calls happen to overlap. `#[serde(flatten)]` merges the
/// rest of the JSON object's keys into `request` based on its own `action`
/// tag.
#[derive(Debug, Deserialize)]
struct Envelope {
    id: String,
    #[serde(flatten)]
    request: IpcRequest,
}

#[derive(Debug, Serialize)]
struct ResponseEnvelope<'a> {
    id: &'a str,
    #[serde(flatten)]
    response: IpcResponse,
}

/// Messages sent from the UI (JS) to Rust.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum IpcRequest {
    Merge {
        paths: Vec<String>,
    },
    Split {
        path: String,
        ranges: Vec<PageRange>,
    },
    Compress {
        path: String,
        quality: Option<u8>,
    },
    Encrypt {
        path: String,
        password: String,
    },
    Decrypt {
        path: String,
        password: String,
    },
    DeletePages {
        path: String,
        pages: Vec<u32>,
    },
    ExtractPages {
        path: String,
        pages: Vec<u32>,
    },
    RotatePages {
        path: String,
        pages: Vec<u32>,
        degrees: i32,
    },
    ReorderPages {
        path: String,
        order: Vec<u32>,
    },
    PdfToImages {
        path: String,
        dpi: Option<u32>,
    },
    ImagesToPdf {
        paths: Vec<String>,
        output_name: Option<String>,
    },
    AddWatermark {
        path: String,
        text: String,
        font_size: Option<f32>,
        opacity: Option<f32>,
        rotation: Option<f32>,
    },
    GetPageCount {
        path: String,
    },
    RevealInFinder {
        path: String,
    },
    /// Opens a native OS file picker and returns the chosen absolute paths.
    /// This replaces relying on the browser `File.path` property, which no
    /// webview actually exposes.
    PickFiles {
        /// "pdf" or "image"
        kind: String,
        multiple: bool,
    },
}

#[derive(Debug, Deserialize)]
pub struct PageRange {
    pub start: u32,
    pub end: u32,
}

/// Responses sent from Rust back to the UI.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok {
        output: String,
        message: Option<String>,
    },
    Error {
        message: String,
    },
    PageCount {
        count: u32,
    },
    Paths {
        paths: Vec<String>,
    },
}

/// Builds the IPC handler wry calls for every `window.ipc.postMessage(...)`
/// from the page. `webview` is filled in by `main.rs` right after the
/// webview is constructed; the handler uses it to call `evaluate_script`
/// with the JSON response, which is how the earlier version's responses
/// were supposed to reach the frontend but never actually did (it only
/// logged them).
pub fn make_handler(
    webview: Arc<Mutex<Option<WebView>>>,
) -> impl Fn(wry::http::Request<String>) + Send + Sync + 'static {
    move |req: wry::http::Request<String>| {
        let body = req.body();

        let (id, response) = match serde_json::from_str::<Envelope>(body) {
            Ok(env) => {
                let id = env.id;
                let response = handle_request(env.request);
                (id, response)
            }
            Err(e) => (
                "unknown".to_string(),
                IpcResponse::Error {
                    message: format!("Invalid request: {}", e),
                },
            ),
        };

        let wrapped = ResponseEnvelope {
            id: &id,
            response,
        };
        let json = serde_json::to_string(&wrapped).unwrap_or_else(|e| {
            format!(
                r#"{{"id":"{}","status":"error","message":"serialization failed: {}"}}"#,
                id, e
            )
        });

        let script = format!("window.__ipc_cb && window.__ipc_cb({});", json);

        match webview.lock() {
            Ok(guard) => {
                if let Some(wv) = guard.as_ref() {
                    if let Err(e) = wv.evaluate_script(&script) {
                        log::error!("Failed to deliver IPC response to webview: {}", e);
                    }
                } else {
                    log::error!("IPC response dropped: webview not yet initialised");
                }
            }
            Err(e) => log::error!("Webview lock poisoned: {}", e),
        }
    }
}

fn handle_request(request: IpcRequest) -> IpcResponse {
    match request {
        IpcRequest::Merge { paths } => {
            let src_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            if src_paths.is_empty() {
                return IpcResponse::Error {
                    message: "At least one input file is required.".into(),
                };
            }
            let output = derive_output(&src_paths[0], "merged");
            match pdf::merge(&src_paths, &output) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Merged successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::Split { path, ranges } => {
            let src = PathBuf::from(&path);
            let outputs: Vec<PathBuf> = ranges
                .iter()
                .enumerate()
                .map(|(i, _)| derive_output(&src, &format!("split_{}", i + 1)))
                .collect();
            let page_ranges: Vec<(u32, u32)> =
                ranges.iter().map(|r| (r.start, r.end)).collect();
            match pdf::split(&src, &page_ranges, &outputs) {
                Ok(_) => IpcResponse::Ok {
                    output: src
                        .parent()
                        .unwrap_or(&src)
                        .to_string_lossy()
                        .into_owned(),
                    message: Some(format!("Split into {} file(s).", outputs.len())),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::Compress { path, quality } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "compressed");
            match pdf::compress(&src, &output, quality) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Compressed successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::Encrypt { path, password } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "encrypted");
            match pdf::encrypt(&src, &output, &password) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Encrypted successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::Decrypt { path, password } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "decrypted");
            match pdf::decrypt(&src, &output, &password) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Decrypted successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::DeletePages { path, pages } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "deleted");
            match pdf::delete_pages(&src, &output, &pages) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some(format!("Deleted {} page(s).", pages.len())),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::ExtractPages { path, pages } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "extracted");
            match pdf::extract_pages(&src, &output, &pages) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some(format!("Extracted {} page(s).", pages.len())),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::RotatePages {
            path,
            pages,
            degrees,
        } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "rotated");
            match pdf::rotate_pages(&src, &output, &pages, degrees) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Rotated successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::ReorderPages { path, order } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "reordered");
            match pdf::reorder_pages(&src, &output, &order) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Pages reordered successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::PdfToImages { path, dpi } => {
            let src = PathBuf::from(&path);
            let dpi = dpi.unwrap_or(150).clamp(72, 600);
            match pdf::pdf_to_images(&src, dpi) {
                Ok(paths) => {
                    let first = paths.first().cloned().unwrap_or(src);
                    let dir = first.parent().unwrap_or(&first).to_string_lossy().into_owned();
                    IpcResponse::Ok {
                        output: dir,
                        message: Some(format!("Exported {} image(s).", paths.len())),
                    }
                }
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::ImagesToPdf { paths, output_name } => {
            let img_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
            if img_paths.is_empty() {
                return IpcResponse::Error {
                    message: "At least one image is required.".into(),
                };
            }
            let first = img_paths[0].clone();
            let parent = first.parent().unwrap_or_else(|| std::path::Path::new("."));
            let name = output_name.unwrap_or_else(|| "images_combined.pdf".into());
            let output = parent.join(&name);
            match pdf::images_to_pdf(&img_paths, &output) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some(format!("Combined {} image(s) into a PDF.", img_paths.len())),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::AddWatermark { path, text, font_size, opacity, rotation } => {
            let src = PathBuf::from(&path);
            let output = derive_output(&src, "watermarked");
            let opts = pdf::WatermarkOptions {
                text: &text,
                font_size: font_size.unwrap_or(60.0),
                opacity:   opacity.unwrap_or(0.15),
                rotation:  rotation.unwrap_or(45.0),
                color:     (0.5, 0.5, 0.5),
            };
            match pdf::add_watermark(&src, &output, &opts) {
                Ok(_) => IpcResponse::Ok {
                    output: output.to_string_lossy().into_owned(),
                    message: Some("Watermark added successfully.".into()),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::GetPageCount { path } => {
            match pdf::page_count(&PathBuf::from(&path)) {
                Ok(count) => IpcResponse::PageCount { count },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::RevealInFinder { path } => {
            reveal_in_finder(&path);
            IpcResponse::Ok {
                output: path,
                message: None,
            }
        }

        IpcRequest::PickFiles { kind, multiple } => {
            let mut dialog = rfd::FileDialog::new();
            dialog = match kind.as_str() {
                "image" => dialog.add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"]),
                _ => dialog.add_filter("PDF", &["pdf"]),
            };

            let chosen = if multiple {
                dialog.pick_files()
            } else {
                dialog.pick_file().map(|p| vec![p])
            };

            match chosen {
                Some(files) => IpcResponse::Paths {
                    paths: files
                        .into_iter()
                        .map(|p| p.to_string_lossy().into_owned())
                        .collect(),
                },
                None => IpcResponse::Paths { paths: vec![] },
            }
        }
    }
}

/// Build output path: same directory, stem + suffix + ".pdf"
fn derive_output(src: &PathBuf, suffix: &str) -> PathBuf {
    let parent = src.parent().unwrap_or_else(|| std::path::Path::new("."));
    let stem = src
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    parent.join(format!("{}_{}.pdf", stem, suffix))
}

fn reveal_in_finder(path: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .args(["-R", path])
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .args(["/select,", path])
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        // Best-effort: open the parent directory
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn();
        }
    }
}
