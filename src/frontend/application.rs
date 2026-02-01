use iced::{Element, Task};

use crate::frontend::message::Message;

#[derive(Default)]
pub struct Application {
    select_album_page
}

impl Application {
    pub fn view(&self) -> Element<Message> {

    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        
    }
}
