// DISPLAY APPLICATION

use iced::{Result, Task};
use reflection::{backend::directories::storage::Storage, display::application::Application};

fn main() -> Result {
    Storage::initialise().expect("Cannot initialise storage.");
    iced::application(
        || (Application::default(), Task::done(reflection::display::application::Message::Initialise)),
        Application::update, Application::view
    )
    // This runs as a photo frame on a kiosk display: always borderless and
    // fullscreen from the very first frame, with no window chrome to fall back
    // to, so an autostarted instance needs no manual set-up after a reboot.
    .window(iced::window::Settings {
        fullscreen: true,
        decorations: false,
        resizable: false,
        ..iced::window::Settings::default()
    })
    .subscription(Application::subscription)
    .run()
}

// The renderer backend is deliberately left to wgpu's own selection, which picks
// the Raspberry Pi's V3D GPU via Vulkan.
//
// An earlier version forced `WGPU_BACKEND=gl` here to work around photos being
// drawn cut across a clean diagonal, with the blurred backdrop showing through
// the gap. That was the wrong fix twice over: the real cause was handing the
// renderer images larger than a single atlas layer (see `MAX_DISPLAY_WIDTH` and
// `MAX_DISPLAY_HEIGHT`), and wgpu's GL path on this hardware falls back to
// software rasterisation — which rendered correctly but took roughly a second
// per frame, making every crossfade a slideshow of two or three steps.
