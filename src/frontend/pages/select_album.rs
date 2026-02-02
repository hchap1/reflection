use iced::{widget::{Column, Scrollable}, Task};
use crate::{frontend::message::Message, onedrive::get_album_children::Album};

pub enum SelectAlbumMessage {
    Refresh,
    AddNew(String)
}

pub struct SelectAlbumPage {
    albums: Vec<Album>
}

impl SelectAlbumPage {
    pub fn view(&self) -> Scrollable<Message> {
        Scrollable::new(
            Column::from_iter(self.albums.iter().map(||))
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

    }
}
