#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::sync::mpsc;

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    WebContext, WebViewBuilder,
};

mod ipc;

#[derive(Debug)]
pub struct WakeUp;

// Assets are embedded at compile time so the binary stays self-contained.
// Adding a new UI file only requires an entry here + a <link>/<script> in index.html.
static INDEX_HTML: &[u8] = include_bytes!("../ui/index.html");
static STYLES_CSS: &[u8] = include_bytes!("../ui/styles.css");
static SCRIPTS_JS: &[u8] = include_bytes!("../ui/scripts.js");

fn serve_asset(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path().trim_start_matches('/');

    let (body, mime): (&'static [u8], &str) = match path {
        "" | "index.html" => (INDEX_HTML, "text/html; charset=utf-8"),
        "styles.css"      => (STYLES_CSS, "text/css; charset=utf-8"),
        "scripts.js"      => (SCRIPTS_JS, "application/javascript; charset=utf-8"),
        _ => {
            return Response::builder()
                .status(404)
                .header(CONTENT_TYPE, "text/plain")
                .body(Cow::Borrowed(b"Not found" as &[u8]))
                .unwrap();
        }
    };

    Response::builder()
        .status(200)
        .header(CONTENT_TYPE, mime)
        .body(Cow::Borrowed(body))
        .unwrap()
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoopBuilder::<WakeUp>::with_user_event().build();

    let window = WindowBuilder::new()
        .with_title("OhPDF")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_min_inner_size(LogicalSize::new(640, 480))
        .build(&event_loop)?;

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("OhPDF")
        .join("WebView2");
    let mut web_context = WebContext::new(Some(data_dir));

    let (tx, rx) = mpsc::channel::<String>();
    let proxy = event_loop.create_proxy();
    let handler = ipc::make_handler(tx, proxy);

    // ohpdf:// is the virtual scheme. On Windows it resolves to http://ohpdf.localhost/
    // so relative <link href="styles.css"> and <script src="scripts.js"> work on all platforms.
    let webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_custom_protocol("ohpdf".into(), |_id, req| serve_asset(req))
        .with_url("ohpdf://localhost/")
        .with_ipc_handler(handler)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
            return;
        }

        while let Ok(script) = rx.try_recv() {
            if let Err(e) = webview.evaluate_script(&script) {
                log::error!("Failed to deliver IPC response to webview: {}", e);
            }
        }
    })
}
