use iced::Task;
use iced::widget::Column;

use crate::frontend::message::Message;
use crate::onedrive::get_album_children::Album;
use crate::onedrive::get_album_children::Photo;

#[derive(Debug, Clone)]
pub enum NewAlbumMessage {
    Display(Album, Vec<Photo>)
}

#[derive(Default)]
pub struct NewAlbumPage {
    album: Option<Album>,
    photos: Vec<Photo>
}

impl NewAlbumPage {
    pub fn view(&self) -> Column<Message> {

    }

    pub fn update(&mut self, message: NewAlbumMessage) -> Task<Message> {

    }
}
