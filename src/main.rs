use std::sync::{Arc, Mutex};

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::{WebView, WebViewBuilder};

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

    // The ipc handler needs to be able to call `evaluate_script` on the
    // webview once a response is ready, but `WebViewBuilder::with_ipc_handler`
    // requires the handler *before* the webview itself exists. We solve the
    // chicken-and-egg problem with a shared slot that gets filled in right
    // after `build()` returns, before the event loop starts running (so
    // there's no window where a message could arrive with the slot empty).
    let webview_slot: Arc<Mutex<Option<WebView>>> = Arc::new(Mutex::new(None));
    let handler = ipc::make_handler(webview_slot.clone());

    let webview = WebViewBuilder::new(&window)
        .with_html(html)
        .with_ipc_handler(handler)
        .with_devtools(cfg!(debug_assertions))
        .build()?;

    *webview_slot.lock().unwrap() = Some(webview);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
