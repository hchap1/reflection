use iced::{Element, Task};

#[derive(Clone, Debug)]
pub enum Message {
    Initialise
}

pub struct Application {

}

impl Default for Application {
    fn default() -> Application {
        Application {

        }
    }
}

impl Application {

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Initialise => {
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("Hello, world!").into()
    }
}
