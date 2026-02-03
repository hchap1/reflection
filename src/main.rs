#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::enum_variant_names)]

use iced::Task;
use frontend::application::Application;

use crate::frontend::message::Message;
use crate::frontend::message::Global;

mod error;
mod util;
mod database;
mod directories;
mod authentication;
mod onedrive;
mod frontend;
mod communication;

fn main() -> iced::Result {
    iced::application(||
        (
            Application::new(),
            Task::done(Message::Global(Global::Authenticate))
        ),
        Application::update,
        Application::view
    ).title("Reflection").run()
}
