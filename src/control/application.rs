use bytes::Bytes;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, checkbox, container, image, image::Handle,
        mouse_area, text, Column, Row, Scrollable, Space,
    },
    Background, Border, Color, Element, Length, Padding, Task,
};
use onedrive_albums::authentication::oauth2;
use rkyv::to_bytes;
use tokio_stream::wrappers::ReceiverStream;
use std::{collections::HashMap, sync::Arc};
use lan_tcp::networking::node::{Destination, Node, RecvPacket, SendPacket};

use crate::{
    IDENTIFIER, PORT,
    backend::{
        database::sql::{Album, Photo},
        networking::network_message::{ControlToDisplay, ObfuscatedUser},
    },
    control::process_packet::process_packet,
    error::Error,
};

mod mocha {
    use iced::Color;
    pub const BASE: Color     = Color { r: 0.118, g: 0.118, b: 0.180, a: 1.0 }; // #1e1e2e
    pub const MANTLE: Color   = Color { r: 0.102, g: 0.102, b: 0.153, a: 1.0 }; // #181825
    pub const SURFACE0: Color = Color { r: 0.192, g: 0.196, b: 0.267, a: 1.0 }; // #313244
    pub const SURFACE1: Color = Color { r: 0.271, g: 0.278, b: 0.345, a: 1.0 }; // #45475a
    pub const TEXT: Color     = Color { r: 0.804, g: 0.839, b: 0.957, a: 1.0 }; // #cdd6f4
    pub const BLUE: Color     = Color { r: 0.537, g: 0.706, b: 0.980, a: 1.0 }; // #89b4fa
    pub const GREEN: Color    = Color { r: 0.651, g: 0.890, b: 0.631, a: 1.0 }; // #a6e3a1
    pub const MAUVE: Color    = Color { r: 0.796, g: 0.651, b: 0.969, a: 1.0 }; // #cba6f7
    pub const MODAL_BG: Color = Color { r: 0.0,   g: 0.0,   b: 0.0,   a: 0.72 };
}

fn pill_button_style(
    accent: Color,
    accent_hover: Color,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered | button::Status::Pressed => accent_hover,
            _ => accent,
        })),
        text_color: mocha::BASE,
        border: Border { radius: 8.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

fn ghost_button_style() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered | button::Status::Pressed => mocha::SURFACE1,
            _ => Color::TRANSPARENT,
        })),
        text_color: mocha::TEXT,
        border: Border { radius: 8.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

fn album_row_style(
    is_active: bool,
) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(match (is_active, status) {
            (true, _) => mocha::BLUE,
            (false, button::Status::Hovered | button::Status::Pressed) => mocha::SURFACE1,
            _ => mocha::SURFACE0,
        })),
        text_color: if is_active { mocha::BASE } else { mocha::TEXT },
        border: Border { radius: 8.0.into(), ..Border::default() },
        ..button::Style::default()
    }
}

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

    // User hover / album loading
    HoverUser(Option<String>),
    LoadUserAlbums(ObfuscatedUser),
    ToggleAddAlbum(usize),
    SubmitAddAlbums,
    CloseAddModal,

    // Album interaction
    SetActiveAlbum(Album),
}

pub struct Application {
    pub node: Option<Arc<Node>>,
    pub users: HashMap<String, ObfuscatedUser>,
    pub active_album: Option<(String, String)>,
    pub albums: HashMap<(String, String), Album>,

    pub album_covers: HashMap<(String, String), Handle>,
    pub album_photos: HashMap<(String, String), HashMap<String, Photo>>,
    pub photo_thumbnails: HashMap<(String, String, String), Handle>,

    pub active_image: Option<Handle>,
    pub add_state: Option<Vec<(bool, Album)>>,
    pub hovered_user: Option<String>,
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
            add_state: None,
            hovered_user: None,
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
                            Err(e) => Message::Error(e.into()),
                        }),
                ])
            }

            Message::NodeCreated(mut node) => {
                let task = if let Some(node) = Arc::get_mut(&mut node) {
                    match node.take_receiver() {
                        Some(receiver) => {
                            Task::stream(ReceiverStream::new(receiver)).map(Message::Recv)
                        }
                        None => Task::done(Message::Error(Error::TcpReceiverMissing)),
                    }
                } else {
                    Task::done(Message::Error(Error::CouldNotMutateNodeArc))
                };

                self.node = Some(node);

                Task::batch(vec![
                    Task::done(Message::Batch(vec![
                        Message::Send(ControlToDisplay::RequestUsers),
                        Message::Send(ControlToDisplay::RequestAlbums),
                        Message::Send(ControlToDisplay::RequestActive),
                    ])),
                    task,
                ])
            }

            Message::Error(e) => {
                eprintln!("ERROR: {e:?}");
                Task::none()
            }

            Message::Send(control_to_display) => {
                match self.node.as_ref() {
                    Some(node_ref) => match to_bytes::<rkyv::rancor::Error>(&control_to_display) {
                        Ok(aligned_vec) => {
                            let sender = node_ref.clone_sender();
                            Task::future(async move {
                                sender
                                    .send(SendPacket {
                                        data: Bytes::from_owner(aligned_vec),
                                        destination: Destination::Server,
                                    })
                                    .await
                            })
                            .map(|res| match res {
                                Ok(()) => Message::None,
                                Err(_) => Message::Error(
                                    lan_tcp::error::Error::MpscChannelFailed.into(),
                                ),
                            })
                        }
                        Err(e) => Task::done(Message::Error(e.into())),
                    },
                    None => Task::done(Message::Error(Error::MissingNode)),
                }
            }

            Message::Batch(messages) => {
                Task::batch(messages.into_iter().map(Task::done))
            }

            Message::Recv(recv_packet) => match process_packet(self, recv_packet) {
                Ok(task) => task,
                Err(e) => Task::done(Message::Error(e)),
            },

            Message::Authenticate => Task::perform(
                oauth2::wrapper::acquire_auth_code(),
                |res| match res {
                    Ok((a, b)) => Message::Send(ControlToDisplay::Authenticated(a, b)),
                    Err(e) => Message::Error(e.into()),
                },
            ),

            Message::HoverUser(id) => {
                self.hovered_user = id;
                Task::none()
            }

            Message::LoadUserAlbums(user) => {
                self.add_state = None;
                Task::done(Message::Send(
                    ControlToDisplay::RequestAlbumsBelongingToUser(user),
                ))
            }

            Message::ToggleAddAlbum(index) => {
                if let Some(state) = self.add_state.as_mut() {
                    if let Some((checked, _)) = state.get_mut(index) {
                        *checked = !*checked;
                    }
                }
                Task::none()
            }

            Message::SubmitAddAlbums => {
                // TODO: Also remove albums from the database that were unticked
                let tasks: Vec<Task<Message>> = self
                    .add_state
                    .iter()
                    .flatten()
                    .filter(|(checked, _)| *checked)
                    .map(|(_, album)| {
                        Task::done(Message::Send(ControlToDisplay::AddAlbum(album.clone())))
                    })
                    .collect();
                self.add_state = None;
                Task::batch(tasks)
            }

            Message::CloseAddModal => {
                self.add_state = None;
                Task::none()
            }

            Message::SetActiveAlbum(album) => Task::done(Message::Send(
                ControlToDisplay::SetActiveAlbum(Some(album)),
            )),

            Message::None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base: Element<'_, Message> = container(
            Row::new()
                .push(self.users_panel())
                .push(self.albums_panel())
                .push(self.active_panel()),
        )
        .style(|_| container::Style {
            background: Some(Background::Color(mocha::BASE)),
            ..container::Style::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        if let Some(add_state) = &self.add_state {
            iced::widget::stack([base, self.add_modal(add_state)]).into()
        } else {
            base
        }
    }

    fn users_panel(&self) -> Element<'_, Message> {
        let list = Column::from_iter(self.users.values().map(|user| {
            let is_hovered = self.hovered_user.as_deref() == Some(user.id.as_str());
            let name = user.name.as_deref().unwrap_or("Unknown");

            let mut inner = Column::new()
                .push(text(name).color(mocha::TEXT).size(14))
                .spacing(6)
                .padding([8, 10]);

            if is_hovered {
                inner = inner.push(
                    button(text("Load Albums").size(12))
                        .on_press(Message::LoadUserAlbums(user.clone()))
                        .style(pill_button_style(mocha::BLUE, mocha::MAUVE))
                        .padding([4, 10]),
                );
            }

            let card = container(inner)
                .style(move |_| container::Style {
                    background: Some(Background::Color(if is_hovered {
                        mocha::SURFACE1
                    } else {
                        mocha::SURFACE0
                    })),
                    border: Border { radius: 8.0.into(), ..Border::default() },
                    ..container::Style::default()
                })
                .width(Length::Fill);

            mouse_area(card)
                .on_enter(Message::HoverUser(Some(user.id.clone())))
                .on_exit(Message::HoverUser(None))
                .into()
        }))
        .spacing(6)
        .padding([0, 8]);

        panel(
            "Users",
            Scrollable::new(list).height(Length::Fill).into(),
            Some(
                button(text("Authenticate New").size(13))
                    .on_press(Message::Authenticate)
                    .style(pill_button_style(mocha::GREEN, mocha::MAUVE))
                    .padding([8, 16])
                    .width(Length::Fill)
                    .into(),
            ),
        )
    }

    fn albums_panel(&self) -> Element<'_, Message> {
        let list = Column::from_iter(self.albums.values().map(|album| {
            let key = (album.user_id.clone(), album.id.clone());
            let is_active = self.active_album.as_ref() == Some(&key);

            let mut row = Row::new()
                .align_y(Vertical::Center)
                .padding([6, 8])
                .spacing(8);

            if let Some(h) = self.album_covers.get(&key) {
                row = row.push(image(h.clone()).width(36).height(36));
            }

            row = row.push(
                text(&album.name)
                    .color(if is_active { mocha::BASE } else { mocha::TEXT })
                    .size(14),
            );

            button(row)
                .on_press(Message::SetActiveAlbum(album.clone()))
                .style(album_row_style(is_active))
                .width(Length::Fill)
                .into()
        }))
        .spacing(4)
        .padding([0, 8]);

        panel(
            "Albums",
            Scrollable::new(list).height(Length::Fill).into(),
            None,
        )
    }

    fn active_panel(&self) -> Element<'_, Message> {
        let mut body = Column::new().spacing(12).padding([0, 8]);

        if let Some(key) = &self.active_album {
            if let Some(album) = self.albums.get(key) {
                let mut album_col = Column::new()
                    .spacing(8)
                    .align_x(Horizontal::Center)
                    .padding(12);

                if let Some(h) = self.album_covers.get(key) {
                    album_col = album_col.push(image(h.clone()).width(100).height(100));
                }

                album_col = album_col.push(text(&album.name).color(mocha::TEXT).size(15));

                body = body.push(
                    container(album_col)
                        .align_x(Horizontal::Center)
                        .width(Length::Fill),
                );
            }
        }

        if let Some(h) = &self.active_image {
            body = body.push(image(h.clone()).width(Length::Fill));
        }

        panel("Active Album", Scrollable::new(body).height(Length::Fill).into(), None)
    }

    fn add_modal<'a>(&'a self, add_state: &'a [(bool, Album)]) -> Element<'a, Message> {
        let album_list = Column::from_iter(add_state.iter().enumerate().map(|(i, (checked, album))| {
            Row::new()
                .push(
                    checkbox(*checked)
                        .on_toggle(move |_| Message::ToggleAddAlbum(i)),
                )
                .push(text(&album.name).color(mocha::TEXT).size(14))
                .align_y(Vertical::Center)
                .spacing(8)
                .into()
        }))
        .spacing(10)
        .padding([4, 0]);

        let modal = container(
            Column::new()
                .push(
                    Row::new()
                        .push(text("Add Albums").color(mocha::MAUVE).size(20))
                        .push(Space::new().width(Length::Fill))
                        .push(
                            button(text("✕").size(14).color(mocha::TEXT))
                                .on_press(Message::CloseAddModal)
                                .style(ghost_button_style())
                                .padding([4, 8]),
                        )
                        .align_y(Vertical::Center),
                )
                .push(Scrollable::new(album_list).height(Length::Fixed(360.0)))
                .push(
                    Row::new()
                        .push(Space::new().width(Length::Fill))
                        .push(
                            button(text("Cancel").size(13))
                                .on_press(Message::CloseAddModal)
                                .style(ghost_button_style())
                                .padding([8, 16]),
                        )
                        .push(
                            button(text("Submit").size(13))
                                .on_press(Message::SubmitAddAlbums)
                                .style(pill_button_style(mocha::BLUE, mocha::MAUVE))
                                .padding([8, 16]),
                        )
                        .spacing(8)
                        .align_y(Vertical::Center),
                )
                .spacing(16)
                .padding(24),
        )
        .style(|_| container::Style {
            background: Some(Background::Color(mocha::MANTLE)),
            border: Border {
                color: mocha::SURFACE1,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..container::Style::default()
        })
        .max_width(480);

        container(modal)
            .style(|_| container::Style {
                background: Some(Background::Color(mocha::MODAL_BG)),
                ..container::Style::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }
}

fn panel<'a>(
    title: &'static str,
    content: Element<'a, Message>,
    bottom: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut col = Column::new()
        .push(
            container(text(title).color(mocha::MAUVE).size(18))
                .padding(Padding { top: 16.0, right: 16.0, bottom: 10.0, left: 16.0 }),
        )
        .push(content)
        .height(Length::Fill);

    if let Some(b) = bottom {
        col = col.push(
            container(b).padding(Padding { top: 8.0, right: 16.0, bottom: 16.0, left: 16.0 }),
        );
    }

    container(col)
        .style(|_| container::Style {
            background: Some(Background::Color(mocha::MANTLE)),
            border: Border {
                color: mocha::SURFACE0,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
