use std::sync::Arc;

use futures_util::FutureExt;

use bytes::Bytes;
use iced::{
    Element,
    Task
};

async fn yield_res<T>(error: T) -> Result<(), T> {
    Err(error)
}

use lan_tcp::networking::node::Destination;
use lan_tcp::networking::node::Node;
use lan_tcp::networking::node::RecvPacket;
use lan_tcp::networking::node::SendPacket;
use tokio_stream::wrappers::ReceiverStream;
use rkyv::to_bytes;

use crate::IDENTIFIER;
use crate::PORT;
use crate::backend::database::authentication_storage::Authentication;
use crate::backend::database::database_backend::Database;
use crate::backend::networking::network_message::DisplayToControl;
use crate::display::process_packet::process_packet;
use crate::error::Error;

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Initialise database and networking
    Initialise,
    NodeCreated(Arc<Node>),

    // Incoming TCP packet (recv packet)
    RecvPacket(RecvPacket),

    // Send to all Control applications
    Send(DisplayToControl),

    // Produce many messages all at once,
    Batch(Vec<Message>),
    
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

            // Called from the creation of the application
            // Used to gain asynchronous context for initialisation
            Message::Initialise => {
                Task::batch(vec![
                    Task::future(Database::initialise())
                    .map(|res| match res {
                        Ok(()) => Message::None,
                        Err(e) => Message::Error(e)
                    }).chain(
                        Task::future(Authentication::create_initial())
                        .map(|res| match res {
                            Ok(()) => Message::None,
                            Err(e) => Message::Error(e)
                        })
                    ),
                    Task::future(Node::spawn_server(IDENTIFIER, PORT, 100))
                    .map(|res| match res {
                        Ok(node) => Message::NodeCreated(Arc::new(node)),
                        Err(e) => Message::Error(e.into())
                    })
                ])
            },

            // Once the node has been successfully initialised
            // Take the receiver from the node and keep it in a stream
            Message::NodeCreated(mut node) => {

                // Try and mutate the node to retrieve the receiver
                // This fails if something else has a weak reference
                let task = if let Some(node) = Arc::get_mut(&mut node) {
                    match node.take_receiver() {
                        Some(receiver) => Task::stream(ReceiverStream::new(receiver))
                            .map(|recv_packet| Message::RecvPacket(recv_packet)),
                        None => Task::done(Message::Error(Error::TcpReceiverMissing))
                    }
                } else {
                    Task::done(Message::Error(Error::CouldNotMutateNodeArc))
                };
                self.node = Some(node);
                task
            }

            // Process an outgoing tcp packet
            Message::Send(display_to_control) => {
                match self.node.as_ref() {
                    Some(node_ref) => match to_bytes::<rkyv::rancor::Error>(&display_to_control) {
                        Ok(aligned_vec) => {
                            let sender = node_ref.clone_sender();
                            Task::future(
                                async move {
                                    sender.send(
                                        SendPacket {
                                            data: Bytes::from_owner(aligned_vec),
                                            destination: Destination::All
                                        }
                                    ).await
                                }
                            ).map(|res| match res {
                                Ok(()) => Message::None,
                                Err(_) => Message::Error(
                                    lan_tcp::error::Error::MpscChannelFailed.into()
                                )
                            })
                        },

                        Err(e) => Task::done(Message::Error(e.into()))
                    },
                    None => Task::done(Message::Error(Error::MissingNode))
                }
            }

            // Process an incoming tcp packet
            Message::RecvPacket(recv_packet) => match process_packet(recv_packet) {
                Ok(task) => task,
                Err(e) => Task::done(Message::Error(e))
            },

            // Produce each message individually
            Message::Batch(messages) => {
                Task::batch(messages
                    .into_iter()
                    .map(|message| Task::done(message))
                )
            }

            // Process an error. For now, this is just printed
            Message::Error(e) => {
                eprintln!("ERROR: {e:?}");
                Task::none()
            },

            // Do nothing
            Message::None => Task::none()
        }

    }

    /// View logic of the application
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("Hello, world!").into()
    }
}
