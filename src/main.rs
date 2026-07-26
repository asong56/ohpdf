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

    let (tx, rx) = mpsc::channel::<String>();
    let handler = ipc::make_handler(tx);

    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(handler)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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
