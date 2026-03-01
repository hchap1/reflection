#![allow(mismatched_lifetime_syntaxes)]
#![allow(clippy::enum_variant_names)]

use iced::Task;
use crate::util::relay::Relay;

use std::env::args;

mod error;
mod util;
mod database;
mod directories;
mod authentication;
mod onedrive;
mod frontend;
mod communication;

fn main() -> iced::Result {

    let args = args().nth(1).unwrap().to_string();

    match args.as_str() {
        "display" => {
            iced::application(||
                {
                    let (application, error_handle, network_receiver) = crate::frontend::display_application::application::Application::new();
                    (
                        application,
                        Task::batch(vec![
                            Task::stream(Relay::consume_receiver(error_handle, |e| Some(crate::frontend::display_application::message::Message::Error(e.into())))),
                            Task::stream(Relay::consume_receiver(network_receiver, |nm| Some(crate::frontend::display_application::message::Message::IncomingNetworkMessage(nm))))
                        ])
                    )
                },
                crate::frontend::display_application::application::Application::update,
                crate::frontend::display_application::application::Application::view,
            ).title("Display").run()
        },

        "control" => {
            iced::application(||
                (
                    crate::frontend::control_application::application::Application::default(),
                    Task::none()
                ),
                crate::frontend::control_application::application::Application::update,
                crate::frontend::control_application::application::Application::view,
            ).title("Control").run()
        }

        _ => Ok(())
    }
}
