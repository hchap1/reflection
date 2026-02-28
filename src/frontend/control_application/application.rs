use std::collections::HashMap;

use iced::advanced::image::Handle;
use iced::{Background, Length, Task};
use iced::widget::{Column, Container, MouseArea, Row, Scrollable, Space, Stack, button, image};
use iced::widget::text;

use crate::authentication::oauth2::wrapper::{authenticate, stateless_authentication};
use crate::communication::client::Client;
use crate::communication::NetworkMessage;
use crate::frontend::application::ApplicationError;
use crate::frontend::colour::Colour;
use crate::frontend::control_application::message::Message;
use crate::onedrive::get_album_children::{Album, Photo};
use crate::util::relay;

#[derive(Default)]
pub struct Application {
    remote_connection: Option<Client>,
    albums: Vec<(Album, Vec<Photo>, bool)>,
    thumbnails: HashMap<String, Handle>,
    active_album: Option<Album>,
}

impl Application {
    pub fn view(&self) -> Container<Message> {
        if self.remote_connection.is_none() {
            return Container::new(
                Column::new()
                    .push(text("Could not connect to display server."))
                    .push(
                        button("Retry?")
                            .on_press(Message::Connect)
                    )
            );
        }

        Container::new(
            Column::new()
                .spacing(10)
                .padding(10)
                .push(
                    Scrollable::new(
                        Column::from_iter(self.albums
                            .iter()
                            .enumerate()
                            .map(|(idx, (album, _, hovered))|
                                MouseArea::new(
                                    Container::new(
                                        Row::new()
                                            .spacing(10)
                                            .padding(10)
                                            .push(
                                                Stack::new()

                                            ).push(
                                                Column::new()
                                                    .spacing(10)
                                                    .push(text(&album.name))
                                                    .push(text(&album.onedrive_id))
                                            )
                                    ).style(
                                        |_| iced::widget::container::Style::default()
                                            .background(match hovered {
                                                true => Colour::accent(),
                                                false => Colour::background()
                                            })
                                    )
                                )
                                .on_enter(Message::Hover(idx))
                                .on_exit(Message::Unhover(idx))
                                .on_press(Message::OutgoingNetworkMessage(NetworkMessage::PlayAlbum(album.clone())))
                                .into()
                            )
                        )
                    )
                ).push(
                    Row::new()
                        .spacing(10)
                        .padding(10)
                        .push(
                            button(text("Authenticate"))
                                .on_press(Message::Authenticate)
                        )
                )
        )

        /*
        Code for loading images for later

                                                .push(match handle {
                                                    Some(handle) => Container::new(image(handle).width(128).height(128)),
                                                    None => Container::new(Space::new().width(Length::Fixed(128f32)).height(Length::Fixed(128f32))) .style(|_| iced::widget::container::Style::default().background(Background::Color(Colour::gray())))
                                                })
*/
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

        match message {

            // Establish connection to the display server and initialise other asynchronous items.
            Message::Connect => {
                let (client, receiver) = Client::spawn();
                self.remote_connection = Some(client);
                Task::stream(relay::Relay::consume_receiver(receiver, |nm|
                    Some(Message::IncomingNetworkMessage(nm))
                ))
            },

            Message::IncomingNetworkMessage(nm) => {
                match nm {

                    // IncomingNetworkMessage

                    NetworkMessage::NewAlbum(album) => {
                        self.albums.push((album, vec![]));
                        Task::none()
                    },
                    
                    NetworkMessage::ReturnAllAlbums(albums) => {
                        self.albums.append(&mut albums.into_iter().map(|x| (x, vec![])).collect());
                        Task::none()
                    },

                    NetworkMessage::ReturnPhotosInAlbum(album, mut photos) => {
                        for album_mutref in &mut self.albums {
                            if album_mutref.0.name == album.name {
                                album_mutref.1.append(&mut photos)
                            }
                        }.into()
                    },

                    NetworkMessage::Thumbnail(photo, bytes) => {
                        self.thumbnails.insert(photo.onedrive_id, Handle::from_bytes(bytes));
                        Task::none()
                    }

                    NetworkMessage::ReturnActiveAlbum(album) => {
                        self.active_album = album;
                        Task::none()
                    },
                }
            },

            Message::None => Task::none(),
            Message::Authenticate => {
                Task::future(stateless_authentication()).map(|res| match res {
                    Ok((tokenset, drivedata)) => Message::OutgoingNetworkMessage(NetworkMessage::DispatchAuthentication(tokenset, drivedata)),
                    Err(e) => Message::Error(e)
                })
            },

            Message::Hover(idx) => {
                if let Some(item) = self.albums.get_mut(idx) {
                    item.2 = true;
                }
                Task::none()
            }

            Message::Unhover(idx) => {
                if let Some(item) = self.albums.get_mut(idx) {
                    item.2 = false;
                }
                Task::none()
            }

            Message::OutgoingNetworkMessage(nm) => {
                // OutgoingNetworkMessage
                if let Some(client) = self.remote_connection.as_mut() {
                    Task::future(client.send(nm))
                        .map(|res| match res {
                            Ok(()) => Message::None,
                            Err(e) => Message::Error(e)
                        })
                } else {
                    Task::done(Message::Error(ApplicationError::NotConnected.into()))
                }
            }
        }

    }
}
