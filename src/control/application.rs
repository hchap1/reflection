use bytes::Bytes;
use iced::widget::image;
use iced::{widget::{button, image::Handle, Column, Row, Scrollable}, Element, Task};
use iced::widget::text;
use onedrive_albums::authentication::oauth2;
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
    Recv(RecvPacket),

    Authenticate,
}

pub struct Application {
    pub node: Option<Arc<Node>>,
    pub users: HashMap<String, ObfuscatedUser>,
    pub active_album: Option<(String, String)>,
    pub albums: HashMap<(String, String), Album>,

    // Associate an album id with an image/photos
    pub album_covers: HashMap<(String, String), Handle>,
    pub album_photos: HashMap<(String, String), HashMap<String, Photo>>,
    pub photo_thumbnails: HashMap<(String, String, String), Handle>,

    // Active image display
    pub active_image: Option<Handle>,
    pub add_state: Option<Vec<(bool, Album)>>
}

#[allow(clippy::derivable_impls)]
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
                    Task::future(Node::spawn_client(IDENTIFIER, PORT))
                    .map(|res| match res {
                        Ok(node) => Message::NodeCreated(Arc::new(node)),
                        Err(e) => Message::Error(e.into())
                    }),
                ])
            },

            Message::NodeCreated(node) => {
                self.node = Some(node);
                Task::done(
                    Message::Batch(vec![
                        Message::Send(ControlToDisplay::RequestUsers),
                        Message::Send(ControlToDisplay::RequestAlbums),
                        Message::Send(ControlToDisplay::RequestActive),
                    ])
                )
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

            Message::Authenticate => {
                Task::perform(
                    oauth2::wrapper::acquire_auth_code(),
                    |res| match res {
                        Ok((a, b)) => Message::Send(ControlToDisplay::Authenticated(a, b)),
                        Err(e) => Message::Error(e.into())
                    }
                )
            },

            Message::None => Task::none()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        Row::new()
            .push(
                Column::new()
                    .push(
                        text("Users")
                    ).push(
                        Scrollable::new(
                            Column::from_iter(
                                self.users.values()
                                    .map(|user| Row::new()
                                        .push(user.name.as_ref().map(text))
                                        .into()
                                    )
                            )
                        )
                    ).push(
                        button("AUTHENTICATE NEW")
                            .on_press(Message::Authenticate)
                    )
                )
            .push(
                Column::new()
                    .push(
                        text("Albums")
                    ).push(
                        Scrollable::new(
                            Column::from_iter(
                                self.albums.values()
                                    .map(|album| Row::new()
                                        .push(self.album_covers.get(
                                                &(album.user_id.clone(), album.id.clone())).map(image)
                                        ).push(text(&album.name))
                                        .into()
                                    )
                            )
                        )
                    )
                )
            .push(
                Column::new()
                    .push(text("Active Album"))
                    .push(
                        self.active_album.as_ref()
                            .map(|key|
                                self.albums.get(key)
                                    .map(|album|
                                        Row::new()
                                            .push(self.album_covers.get(key).map(image))
                                            .push(text(&album.name))
                                    )
                            )
                    ).push(text("Active Image"))
                    .push(
                        self.active_image.as_ref().map(image)
                    )
                )
            .into()
    }
}
