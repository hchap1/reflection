use iced::{Background, Length, Task};
use iced::widget::{Column, Container, MouseArea, Row, Scrollable, Space, Stack, button, image, stack};
use iced::advanced::image::Handle;
use iced::widget::text;

use crate::communication::client::Client;
use crate::communication::NetworkMessage;
use crate::frontend::colour::Colour;
use crate::frontend::control_application::message::Message;
use crate::onedrive::get_album_children::Album;

#[derive(Default)]
pub struct Application {
    remote_connection: Option<Client>,
    albums: Vec<(Album, Option<Handle>)>,
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
                            .map(|(idx, (album, handle))|
                                MouseArea::new(
                                    Row::new()
                                        .spacing(10)
                                        .padding(10)
                                        .push(
                                            Stack::new()
                                                .push(match handle {
                                                    Some(handle) => Container::new(image(handle).width(128).height(128)),
                                                    None => Container::new(Space::new().width(Length::Fixed(128f32)).height(Length::Fixed(128f32)))
                                                        .style(|_| iced::widget::container::Style::default().background(Background::Color(Colour::gray())))
                                                })
                                        ).push(
                                            Column::new()
                                                .spacing(10)
                                                .push(text(&album.name))
                                                .push(text(&album.onedrive_id))
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
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

        match message {

            // Establish connection to the display server and initialise other asynchronous items.
            Message::Connect => {
                let (client, receiver) = Client::spawn();
                self.client = Some(client);

                // TODO relay NetworkMessage
            }
        }

    }
}
