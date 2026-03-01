use async_channel::Receiver;
use iced::Task;
use iced::widget::{Container, text};
use iced::widget::Column;
use rusqlite_async::database::Database;

use crate::authentication::oauth2::wrapper::first_authentication;
use crate::communication::NetworkMessage;
use crate::communication::server::Server;
use crate::frontend::display_application::message::Message;
use crate::{authentication::oauth2::api::TokenSet, database::interface, directories::create::Directories, onedrive::get_drive::DriveData};

pub struct Application {
    connection: Server,
    database: Database,
    directories: Directories,

    // Authentication
    tokenset: Option<TokenSet>,
    drivedata: Option<DriveData>,
}

impl Application {
    pub fn new() -> (Self, Receiver<rusqlite_async::error::Error>, Receiver<NetworkMessage>) {
        let directories = Directories::create_or_load().expect("[CRITICAL ERROR] Unable to find suitable directories location.");
        let (database, error_receiver) = Database::new(directories.root.clone());
        interface::create_tables(database.derive()).expect("[CRITICAL ERROR] Unable to create database tables.");
        let (server, network_receiver) = Server::spawn();

        (Self {
            connection: server,
            database,
            directories,
            tokenset: None,
            drivedata: None
        }, error_receiver, network_receiver)
    }

    pub fn view(&self) -> Container<Message> {
        Container::new(
            Column::new()
                .push(text("Placeholder..."))
                .push(
                    if self.tokenset.is_none() { Some(text("Not authenticated...").color(Colour::error())) }
                    else { None }
                )
                .push(
                    self.connection.get_active_connection().map(|ip| text(format!("Connected to {ip}!")))
                )
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::None => Task::none(),
            Message::IncomingNetworkMessage(nm) => match nm {
                NetworkMessage::TokenSet(tokenset) => {
                    let datalink = self.database.derive();
                    Task::future(first_authentication(datalink, tokenset))
                        .map(|res| match res {
                            Ok((tokenset, drivedata)) => {
                                Message::AuthenticationComplete(tokenset, drivedata)
                            },
                            Err(e) => Message::Error(e)
                        })
                },

                _ => Task::none()
            }

            Message::OutgoingNetworkMessage(nm) => {
                if let Some(_) = self.connection.get_active_connection() {
                    let sender = self.connection.get_sender();
                    Task::future(Server::send_network_message(sender, nm))
                        .map(|res| match res {
                            Ok(()) => Message::None,
                            Err(e) => Message::Error(e)
                        })
                } else {
                    Task::none()
                }
            }

            Message::AuthenticationComplete(tokenset, drivedata) => {
                self.tokenset = Some(tokenset);
                self.drivedata = Some(drivedata);
                Task::none()
            }

            Message::Error(e) => {
                println!("Error: {e:?}");
                Task::none()
            }
        }
    }
}
