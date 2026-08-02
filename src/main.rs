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
use wry::{WebContext, WebViewBuilder};

mod ipc;
mod pdf;

/// A no-op event we send through the event loop's proxy purely to wake it
/// back up — see the comment on `event_loop_proxy` below for why this is
/// needed at all.
#[derive(Debug)]
pub struct WakeUp;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let event_loop = EventLoop::<WakeUp>::with_user_event();

    let window = WindowBuilder::new()
        .with_title("OhPDF")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_min_inner_size(LogicalSize::new(640, 480))
        .build(&event_loop)?;

    let html = include_str!("../ui/index.html");

    // On Windows, WebView2 defaults to creating its user-data folder (cache,
    // cookies, IndexedDB, etc.) as a sibling of the .exe — the
    // "<exe-name>.WebView2" folder the user was seeing appear next to
    // ohpdf.exe on every launch. Microsoft's own guidance is that production
    // apps should always point this somewhere else (the exe's directory may
    // not even be writable, e.g. under Program Files), so we redirect it into
    // the OS's proper per-user app-data directory instead:
    //   Windows: %LOCALAPPDATA%\OhPDF\WebView2
    //   macOS:   ~/Library/Application Support/OhPDF/WebView2
    //   Linux:   ~/.local/share/OhPDF/WebView2 (WebKitGTK profile dir; mostly
    //            a no-op there, but harmless and keeps behavior consistent)
    //
    // wry has no `WebViewBuilder::with_data_directory` method — the actual,
    // documented way to set this is a `WebContext` constructed with the
    // desired path, passed to `WebViewBuilder::new_with_web_context`. The
    // context has to outlive the builder/webview (the builder only borrows
    // it), so it's declared here rather than as a temporary inline value.
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("OhPDF")
        .join("WebView2");
    let mut web_context = WebContext::new(Some(data_dir));

    // `WebView` isn't `Send`/`Sync` (it wraps platform-specific webview
    // handles, e.g. WKWebView on macOS), so it can never be stashed inside an
    // `Arc<Mutex<...>>` and reached from the IPC handler closure directly.
    // Instead, the IPC handler (which runs on the same thread as the event
    // loop) just pushes the finished response script onto a plain channel;
    // the event loop drains that channel and calls `evaluate_script` on the
    // webview itself, which it *does* own directly.
    //
    // Why a separate `EventLoopProxy` wake-up is needed: the loop below runs
    // with `ControlFlow::Wait`, meaning it only wakes up and re-checks the
    // channel when a *window* event (input, resize, etc.) arrives — pushing
    // to `tx` from the IPC handler does not by itself wake a `Wait`ing loop.
    // Without this, a finished operation's result could sit in the channel
    // indefinitely (the UI stuck showing "Processing…") until the user
    // happened to move the mouse or otherwise generate a window event. The
    // IPC handler explicitly wakes the loop via `proxy.send_event` right
    // after queuing its response, so results are always delivered
    // immediately rather than depending on incidental window activity.
    let (tx, rx) = mpsc::channel::<String>();
    let proxy = event_loop.create_proxy();
    let handler = ipc::make_handler(tx, proxy);

    // wry 0.46: `WebViewBuilder::new()` takes no arguments, and the window is
    // supplied to `.build(&window)` instead.
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
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

        // Drain and deliver any queued IPC responses. This runs for every
        // event the loop wakes up for — most importantly the `UserEvent`
        // fired by `proxy.send_event(WakeUp)` right after a response is
        // queued, which is what guarantees it's delivered promptly instead
        // of waiting on unrelated window activity — but draining on any
        // other event too is a harmless no-op when the channel is empty,
        // and catches the rare case where a response lands in the channel
        // between the wake-up event and now.
        while let Ok(script) = rx.try_recv() {
            if let Err(e) = webview.evaluate_script(&script) {
                log::error!("Failed to deliver IPC response to webview: {}", e);
            }
        }
    });
}
