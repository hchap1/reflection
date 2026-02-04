use iced::Task;
use iced::widget::Container;

use crate::communication::server::Server;
use crate::frontend::control_application::message::Message;
use crate::onedrive::get_album_children::Album;

#[derive(Default)]
pub struct Application {
    remote_connection: Option<Server>,
    albums: Vec<Album>,
    active_album: Option<Album>,
}

impl Application {
    pub fn view(&self) -> Container<Message> {

    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

    }
}
