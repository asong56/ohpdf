use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use tao::event_loop::EventLoopProxy;

use crate::pdf;
use crate::WakeUp;

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
    /// Renders small preview thumbnails for every page, used by the
    /// delete/reorder pickers so the user sees actual page content instead
    /// of plain numbered chips or filename rows.
    GetPageThumbnails {
        path: String,
    },
    /// Renders one page at full reading resolution, for the Read &
    /// Annotate view. `page` is 1-indexed. `dpi` follows the current zoom
    /// level; if omitted, a comfortable on-screen default is used.
    RenderPage {
        path: String,
        page: u32,
        dpi: Option<u32>,
    },
    /// Loads any previously-saved annotations (highlights, notes, ink
    /// strokes) for a document from its sidecar `.ohpdf-annot.json` file.
    /// Returns an empty set if none exist yet — this is not an error.
    LoadAnnotations {
        path: String,
    },
    /// Saves the full set of per-page annotations for a document to its
    /// sidecar file. Never touches the PDF itself.
    SaveAnnotations {
        path: String,
        annotations: pdf::PageAnnotations,
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
    Thumbnails {
        pages: Vec<ThumbnailData>,
    },
    Page {
        data_url: String,
    },
    Annotations {
        pages: pdf::PageAnnotations,
    },
    Paths {
        paths: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct ThumbnailData {
    pub page: u32,
    pub data_url: String,
}

/// Builds the IPC handler wry calls for every `window.ipc.postMessage(...)`
/// from the page. `WebView` itself is not `Send`/`Sync` (it wraps
/// platform-specific handles like WKWebView), so this handler can't hold a
/// reference to it directly — instead the actual work runs on a spawned
/// background thread, which sends the finished response script down `tx` (a
/// plain `mpsc::Sender<String>`) and then wakes the event loop via `proxy` so
/// `main.rs` (which owns the matching receiver, on the same thread the
/// webview lives on) drains it and calls `evaluate_script` immediately.
///
/// Running the PDF work on a background thread — rather than directly
/// inline in this handler — matters because this handler itself executes on
/// the webview's own message-handling thread. A large Compress/Merge/etc.
/// call can take a real amount of time; running it inline would block that
/// thread for the whole duration, freezing the entire window (no repaints,
/// no input) until it finished. Spawning it means the UI stays responsive
/// (the "Processing…" spinner keeps animating) the whole time, and the
/// result still gets delivered the moment it's ready via the same
/// wake-up-the-event-loop mechanism.
///
/// The wake-up itself is not optional either: the event loop runs with
/// `ControlFlow::Wait`, which only re-checks the channel when a *window*
/// event (input, resize, etc.) shows up — merely pushing to `tx` does not
/// wake a `Wait`ing loop on its own. Without explicitly waking it, a
/// finished operation's result could sit in the channel indefinitely (the UI
/// stuck showing "Processing…") until the user happened to move the mouse or
/// otherwise generate a window event.
pub fn make_handler(
    tx: Sender<String>,
    proxy: EventLoopProxy<WakeUp>,
) -> impl Fn(wry::http::Request<String>) + Send + Sync + 'static {
    move |req: wry::http::Request<String>| {
        let body = req.body();

        // Parse just enough up front to special-case native file-dialog
        // requests, which — unlike every other action here — must run
        // synchronously on this same thread. On macOS in particular, Cocoa
        // requires all NSOpenPanel/NSSavePanel interaction to happen on the
        // main thread; calling it from a spawned background thread panics
        // outright ("Fallback Sync Dialog Must Be Spawned On Main Thread").
        // This handler already runs on the webview's own thread, which for
        // a single-window desktop app *is* the main thread, so keeping
        // dialog calls right here (rather than moving them to the
        // background pool below) is both correct and effectively free —
        // they're instant to dispatch and only return once the user closes
        // the dialog.
        let envelope = match serde_json::from_str::<Envelope>(body) {
            Ok(env) => env,
            Err(e) => {
                let json = serde_json::to_string(&ResponseEnvelope {
                    id: "unknown",
                    response: IpcResponse::Error {
                        message: format!("Invalid request: {}", e),
                    },
                })
                .unwrap_or_default();
                let script = format!("window.__ipc_cb && window.__ipc_cb({});", json);
                let _ = tx.send(script);
                let _ = proxy.send_event(WakeUp);
                return;
            }
        };

        if matches!(envelope.request, IpcRequest::PickFiles { .. }) {
            let id = envelope.id;
            let response = handle_request(envelope.request);
            let json = serde_json::to_string(&ResponseEnvelope {
                id: &id,
                response,
            })
            .unwrap_or_else(|e| {
                format!(
                    r#"{{"id":"{}","status":"error","message":"serialization failed: {}"}}"#,
                    id, e
                )
            });
            let script = format!("window.__ipc_cb && window.__ipc_cb({});", json);
            if let Err(e) = tx.send(script) {
                log::error!("Failed to queue IPC response (event loop gone?): {}", e);
                return;
            }
            if let Err(e) = proxy.send_event(WakeUp) {
                log::error!("Failed to wake event loop for IPC response: {}", e);
            }
            return;
        }

        // Everything else (PDF processing, thumbnail rendering, etc.) can
        // genuinely take a while for large files, so it runs on a spawned
        // background thread instead — keeping it here would freeze the
        // whole window (no repaints, no input) for the entire duration of
        // e.g. a large Compress or Merge job.
        let tx = tx.clone();
        let proxy = proxy.clone();
        std::thread::spawn(move || {
            let id = envelope.id;
            let request = envelope.request;

            // Guard against an unexpected panic inside mupdf/our own code
            // taking down just this thread silently, which — since nothing
            // would ever send a response back — would leave the UI stuck
            // showing "Processing…" forever with no way to know something
            // went wrong. Catching it here turns that into a normal error
            // response instead. `AssertUnwindSafe` is fine: `request` isn't
            // shared with anything else that could observe it half-mutated
            // after an unwind.
            let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handle_request(request)
            }))
            .unwrap_or_else(|panic_payload| {
                let msg = panic_payload
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown panic".to_string());
                log::error!("PDF operation panicked: {}", msg);
                IpcResponse::Error {
                    message: format!("Internal error: {}", msg),
                }
            });

            let json = serde_json::to_string(&ResponseEnvelope {
                id: &id,
                response,
            })
            .unwrap_or_else(|e| {
                format!(
                    r#"{{"id":"{}","status":"error","message":"serialization failed: {}"}}"#,
                    id, e
                )
            });

            let script = format!("window.__ipc_cb && window.__ipc_cb({});", json);

            if let Err(e) = tx.send(script) {
                log::error!("Failed to queue IPC response (event loop gone?): {}", e);
                return;
            }
            // Wake the event loop so it delivers this response right away
            // instead of waiting on unrelated window activity.
            if let Err(e) = proxy.send_event(WakeUp) {
                log::error!("Failed to wake event loop for IPC response: {}", e);
            }
        });
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

        IpcRequest::GetPageThumbnails { path } => {
            match pdf::pdf_page_thumbnails(&PathBuf::from(&path)) {
                Ok(thumbs) => IpcResponse::Thumbnails {
                    pages: thumbs
                        .into_iter()
                        .map(|t| ThumbnailData {
                            page: t.page,
                            data_url: t.data_url,
                        })
                        .collect(),
                },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::RenderPage { path, page, dpi } => {
            let src = PathBuf::from(&path);
            // 96 matches the reader's own "100% zoom" baseline on the JS
            // side; this fallback only matters if a request is ever sent
            // without an explicit dpi, which the reader itself never does.
            let dpi = dpi.unwrap_or(96).clamp(36, 600);
            match pdf::render_page(&src, page, dpi) {
                Ok(data_url) => IpcResponse::Page { data_url },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::LoadAnnotations { path } => {
            match pdf::load_annotations(&PathBuf::from(&path)) {
                Ok(pages) => IpcResponse::Annotations { pages },
                Err(e) => IpcResponse::Error {
                    message: e.to_string(),
                },
            }
        }

        IpcRequest::SaveAnnotations { path, annotations } => {
            match pdf::save_annotations(&PathBuf::from(&path), &annotations) {
                Ok(_) => IpcResponse::Ok {
                    output: path,
                    message: Some("Annotations saved.".into()),
                },
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
