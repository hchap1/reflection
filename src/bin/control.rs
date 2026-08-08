// CONTROL APPLICATION

use iced::{Result, Task};
use reflection::control::application::Application;

fn main() -> Result {
    iced::application(
        || (Application::default(), Task::done(reflection::control::application::Message::Initialise)),
        Application::update, Application::view
    )
    .subscription(Application::subscription)
    .run()
}
