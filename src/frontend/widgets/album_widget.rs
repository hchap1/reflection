use iced::widget::{Column, Container, Row};
use iced::widget::text;

use crate::{frontend::message::Message, onedrive::get_album_children::Album};

pub struct AlbumWidget;

impl AlbumWidget {
    pub fn list(album: &Album, number_of_items: Option<usize>) -> Container<Message> {
        Container::new(
            Column::new()
                .spacing(10)
                .padding(10)
                .push(
                    text(&album.name)
                ).push(
                    Row::new()
                        .spacing(10)
                        .padding(10)
                        .push(
                            text(&album.onedrive_id)
                        ).push(
                            number_of_items.map(|value| text(value))
                        )
                )
        )
    }
}
