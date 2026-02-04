use iced::{Background, Color, Length, Task};
use iced::widget::{Column, Container, MouseArea, Row, Scrollable, Stack, button, image, stack};
use iced::advanced::image::Handle;

use crate::communication::server::Server;
use crate::frontend::control_application::message::Message;
use crate::onedrive::get_album_children::Album;

#[derive(Default)]
pub struct Application {
    remote_connection: Option<Server>,
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
                            .map(|(album, handle)|
                                MouseArea::new(
                                    Row::new()
                                        .spacing(10)
                                        .padding(10)
                                        .push(
                                            Stack::new()
                                                .push(match handle {
                                                    Some(handle) => image(handle).width(128).height(128),
                                                    None => Container::new()
                                                        .width(Length::Fixed(128)).height(Length::Fixed(128))
                                                        .style(iced::widget::container::Style::default().background(Background::Color(Colour::Gray)))
                                                })
                                        )
                                )
                            )
                        )
                    )
                )
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

    }
}
