// DISPLAY APPLICATION

use iced::{Result, Task};
use reflection::{backend::directories::storage::Storage, display::application::Application};

fn main() -> Result {
    Storage::initialise().expect("Cannot initialise storage.");
    iced::application(
        || (Application::new(), Task::none()),
        Application::update, Application::view
    ).run()
}
