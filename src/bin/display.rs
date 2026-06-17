// DISPLAY APPLICATION

use iced::{Result, Task};
use reflection::{backend::directories::storage::Storage, display::application::Application};

fn main() -> Result {
    Storage::initialise().expect("Cannot initialise storage.");
    iced::application(
        || (Application::new(), Task::done(reflection::display::application::Message::Initialise)),
        Application::update, Application::view
    ).run()
}
