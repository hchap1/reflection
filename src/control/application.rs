use bytes::Bytes;
use iced::{Element, Task, widget::image::Handle};
use rkyv::to_bytes;
use std::{collections::HashMap, sync::Arc};
use lan_tcp::networking::node::{Destination, Node, RecvPacket, SendPacket};

use crate::{IDENTIFIER, PORT, backend::{database::sql::{Album, Photo}, networking::network_message::{ControlToDisplay, ObfuscatedUser}}, control::process_packet::process_packet, error::Error};

#[derive(Clone, Debug)]
pub enum Message {
    None,
    Initialise,
    NodeCreated(Arc<Node>),
    Error(Error),
    Batch(Vec<Message>),

    // Networking
    Send(ControlToDisplay),
    Recv(RecvPacket)
}

pub struct Application {
    pub node: Option<Arc<Node>>,
    pub users: HashMap<String, ObfuscatedUser>,
    pub active_album: Option<String>,
    pub albums: HashMap<String, Album>,

    // Associate an album id with an image/photos
    pub album_covers: HashMap<(String, String), Handle>,
    pub album_photos: HashMap<(String, String), HashMap<String, Photo>>,
    pub photo_thumbnails: HashMap<(String, String, String), Handle>,

    // Active image display
    pub active_image: Option<Handle>,
    pub add_state: Option<Vec<(bool, Album)>>
}

impl Default for Application {
    fn default() -> Application {
        Application {
            node: None,
            users: HashMap::new(),
            active_album: None,
            albums: HashMap::new(),
            album_covers: HashMap::new(),
            album_photos: HashMap::new(),
            photo_thumbnails: HashMap::new(),
            active_image: None,
            add_state: None
        }
    }
}

impl Application {

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Initialise => {
                Task::batch(vec![
                    Task::future(Node::spawn_server(IDENTIFIER, PORT, 100))
                    .map(|res| match res {
                        Ok(node) => Message::NodeCreated(Arc::new(node)),
                        Err(e) => Message::Error(e.into())
                    }),
                ])
            },

            // TODO send initial state requests
            Message::NodeCreated(node) => {
                self.node = Some(node);
                Task::none()
            },

            Message::Error(e) => {
                eprintln!("ERROR: {e:?}");
                Task::none()
            }

            Message::Send(control_to_display) => {
                match self.node.as_ref() {
                    Some(node_ref) => match to_bytes::<rkyv::rancor::Error>(&control_to_display) {
                        Ok(aligned_vec) => {
                            let sender = node_ref.clone_sender();
                            Task::future(
                                async move {
                                    sender.send(
                                        SendPacket {
                                            data: Bytes::from_owner(aligned_vec),
                                            destination: Destination::Server
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
            },

            Message::Batch(messages) => Task::batch(
                messages.into_iter()
                    .map(Task::done)
            ),

            Message::Recv(recv_packet) => {
                match process_packet(self, recv_packet) {
                    Ok(task) => task,
                    Err(e) => Task::done(Message::Error(e))
                }
            },

            Message::None => Task::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::text("Hello, world!").into()
    }
}
