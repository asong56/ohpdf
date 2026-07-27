// Suppresses the console/terminal window that Windows otherwise pops up
// briefly before the real GUI window appears. This only applies in release
// builds — in debug builds we keep the console so `log`/`println!` output
// (and any panics) are still visible while developing.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::mpsc;

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

mod ipc;
mod pdf;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("OhPDF")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_min_inner_size(LogicalSize::new(640, 480))
        .build(&event_loop)?;

    let html = include_str!("../ui/index.html");

    // `WebView` isn't `Send`/`Sync` (it wraps platform-specific webview
    // handles, e.g. WKWebView on macOS), so it can never be stashed inside an
    // `Arc<Mutex<...>>` and reached from the IPC handler closure directly.
    // Instead, the IPC handler (which runs on the same thread as the event
    // loop) just pushes the finished response script onto a plain channel;
    // the event loop drains that channel every iteration and calls
    // `evaluate_script` on the webview itself, which it *does* own directly.
    let (tx, rx) = mpsc::channel::<String>();
    let handler = ipc::make_handler(tx);

    // wry 0.46: `WebViewBuilder::new()` takes no arguments, and the window is
    // supplied to `.build(&window)` instead.
    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(handler)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Drain any IPC responses queued since the last tick and hand them
        // to the webview, which is safe here because we're on the same
        // thread the webview was created on.
        while let Ok(script) = rx.try_recv() {
            if let Err(e) = webview.evaluate_script(&script) {
                log::error!("Failed to deliver IPC response to webview: {}", e);
            }
        }

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
