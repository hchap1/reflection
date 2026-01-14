use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}};
use wry::{WebView, WebViewBuilder};

use crate::error::Res;

#[derive(Default)]
pub struct OAuthWindow {
    window: Option<Window>,
    webview: Option<WebView>
}

impl ApplicationHandler for OAuthWindow {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        let webview = WebViewBuilder::new()
            .with_url("https://youtube.com")
            .build(&window)
            .unwrap();

        self.window = Some(window);
        self.webview = Some(webview);
    }

    // Ignore window events.
    fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: WindowEvent
        ) {}
}

pub fn launch_oauth_window() -> Res<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut window = OAuthWindow::default();
    event_loop.run_app(&mut window)?;

    Ok(())
}
