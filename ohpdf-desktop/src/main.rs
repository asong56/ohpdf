#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::mpsc;

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{WebContext, WebViewBuilder};

mod ipc;

#[derive(Debug)]
pub struct WakeUp;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoopBuilder::<WakeUp>::with_user_event().build();

    let window = WindowBuilder::new()
        .with_title("OhPDF")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_min_inner_size(LogicalSize::new(640, 480))
        .build(&event_loop)?;

    let html = include_str!("../ui/index.html");

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("OhPDF")
        .join("WebView2");
    let mut web_context = WebContext::new(Some(data_dir));

    let (tx, rx) = mpsc::channel::<String>();
    let proxy = event_loop.create_proxy();
    let handler = ipc::make_handler(tx, proxy);

    let webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_html(html)
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
