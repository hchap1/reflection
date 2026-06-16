use std::sync::Arc;

use iced::{Element, Task};
use lan_tcp::networking::node::Node;

use crate::{IDENTIFIER, PORT, backend::database::database_backend::Database, error::Error};

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Initialise database and networking
    Initialise,
    NodeCreated(Arc<Node>),
    
    Error(Error),

}

pub struct Application {
    node: Option<Arc<Node>>
}

impl Application {

    /// Build a new Application initial state
    pub fn new() -> Self {
        Self {
            node: None
        }
    }
    
    /// Handle the internal state of the application
    pub fn update(&mut self, message: Message) -> Task<Message> {
        
        match message {
            Message::Initialise => {
                Task::batch(vec![
                    Task::future(Database::initialise()).map(|res| match res {
                        Ok(()) => Message::None,
                        Err(e) => Message::Error(e)
                    }),
                    Task::future(Node::spawn_server(IDENTIFIER, PORT, 100))
                        .map(|res| match res {
                        Ok(node) => Message::NodeCreated(Arc::new(node)),
                        Err(e) => Message::Error(e.into())
                    })
                ])
            },

            Message::NodeCreated(node) => {
                self.node = Some(node);
                Task::none()
            }

            Message::Error(e) => {
                eprintln!("ERROR: {e:?}");
                Task::none()
            },

            Message::None => Task::none()
        }

    }

    /// View logic of the application
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("Hello, world!").into()
    }
}
