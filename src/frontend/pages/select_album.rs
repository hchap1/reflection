use iced::{Task, widget::Scrollable};
use crate::frontend::message::Message;

pub enum SelectAlbumMessage {
    Refresh,
    AddNew(String)
}

pub struct SelectAlbumPage {

}

impl SelectAlbumPage {

    pub fn new() -> Self {
        Self {

        }
    }

    pub fn view(&self) -> Scrollable<Message> {

    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

    }
}
