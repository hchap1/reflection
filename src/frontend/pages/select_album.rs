use iced::{widget::{text_input, Column, Row, Scrollable}, Task};
use iced::widget::text;
use iced::widget::button;
use crate::{frontend::{message::Message, widgets::album_widget::AlbumWidget}, onedrive::get_album_children::Album};

pub enum SelectAlbumMessage {
    Refresh,
    AddNew,
    Input(String)
}

pub struct SelectAlbumPage {
    albums: Vec<Album>,
    input: String }
impl SelectAlbumPage {
    pub fn view(&self) -> Column<Message> {
        Column::new()
            .push(
                Scrollable::new(
                    Column::from_iter(self.albums.iter().map(|album| AlbumWidget::list(album, None).into()))
                        .spacing(10)
                )
            ).push(
                Row::new()
                    .push(
                        text_input("Add new sharelink", &self.input)
                            .on_input(|value| SelectAlbumMessage::Input(value).into())
                            .on_submit(SelectAlbumMessage::AddNew.into())
                    ).push(
                        button(text("Add"))
                            .on_press(SelectAlbumMessage::AddNew.into())
                    )
            )
    }

    pub fn update(&mut self, message: SelectAlbumMessage) -> Task<Message> {

    }
}
