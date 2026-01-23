#![allow(clippy::enum_variant_names)]

use iced::Task;
use frontend::application::Application;
use rusqlite_async::database::Database;
use directories::create::Directories;

use crate::frontend::message::Message;
use crate::frontend::message::Global;

mod error;
mod util;
mod database;
mod directories;
mod authentication;
mod onedrive;
mod frontend;

fn main() -> iced::Result {
    let directories = Directories::create_or_load().expect("[CRITICAL ERROR] Unable to find suitable directories location.");
    let (database, error_handle) = Database::new(directories.root);

    iced::application(|| (Application::default(), Task::done(Message::Global(Global::Authenticate))),
        Application::update(),
        Application::view()
    ).title("Reflection").run()
}

// TODO Test if album updating works
