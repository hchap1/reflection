use iced::widget::{Button, Column, Row};
use iced::widget::text;

use crate::frontend::message::Global;
use crate::{frontend::message::Message, onedrive::get_album_children::Album};

pub struct AlbumWidget;

impl AlbumWidget {
    pub fn list(album: &Album, number_of_items: Option<usize>) -> Button<Message> {
        Button::new(
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
                            number_of_items.map(text)
                        )
                )
        ).on_press(Global::BrowseAlbum(album.id).into())
    }
}
