use bytes::Bytes;
use iced::{
    alignment::{Horizontal, Vertical},
    widget::{
        button, checkbox, container, image, image::Handle,
        mouse_area, slider, text, Column, Row, Scrollable, Space,
    },
    Background, Border, Color, Element, Length, Padding, Subscription, Task,
};
use std::time::{Duration, Instant};
use onedrive_albums::authentication::oauth2;
use rkyv::to_bytes;
use tokio_stream::wrappers::ReceiverStream;
use std::{collections::HashMap, sync::Arc};
use lan_tcp::networking::node::{Destination, Node, RecvPacket, SendPacket};

use crate::{
    IDENTIFIER, PORT,
    backend::{
        database::settings::{
            self, Settings, BLUR_MIN, PERIOD_MAX, PERIOD_MIN,
        },
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
    pub const RED: Color      = Color { r: 0.953, g: 0.545, b: 0.659, a: 1.0 }; // #f38ba8
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
    BrowseAlbum(Album),

    // Connection liveness
    Heartbeat,
    Reconnect,

    // Settings page
    OpenSettings,
    CloseSettings,
    PeriodChanged(f32),
    BlurDurationChanged(f32),
    CommitSettings,
    SettingsReceived(f32, f32),
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

    // Album whose thumbnails are shown in the browse grid. Kept separate from
    // `active_album` so the album playing on the display can be inspected
    // without changing it, and vice versa.
    pub browsing_album: Option<(String, String)>,

    // Timing settings as last confirmed by the display application, and whether
    // the settings page is open.
    pub settings: Settings,
    pub show_settings: bool,

    // Connection liveness. `last_seen` is the moment the most recent packet of
    // any kind arrived; `waiting_since` is when the current node was created, so
    // a link that never produced a single packet can still be judged.
    pub last_seen: Option<Instant>,
    pub waiting_since: Option<Instant>,
}

/// How the control application is currently faring against the display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// No node yet, or one that has not produced any traffic yet.
    Connecting,
    /// A packet arrived recently.
    Live,
    /// Nothing has arrived for long enough that the link should be treated as
    /// down — the display was restarted, went away, or the network dropped.
    Lost,
}

/// How often the control application probes the display.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

/// Silence beyond this is treated as a dead connection. Comfortably more than
/// several heartbeat intervals, so a single dropped probe does not raise a
/// false alarm.
const LIVENESS_TIMEOUT: Duration = Duration::from_secs(7);

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
            browsing_album: None,
            settings: Settings::default(),
            show_settings: false,
            last_seen: None,
            waiting_since: None,
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
                self.waiting_since = Some(Instant::now());

                Task::batch(vec![
                    Task::done(Message::Batch(vec![
                        Message::Send(ControlToDisplay::RequestUsers),
                        Message::Send(ControlToDisplay::RequestAlbums),
                        Message::Send(ControlToDisplay::RequestActive),
                        Message::Send(ControlToDisplay::RequestSettings),
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

            // Any packet at all is evidence the link is alive, whatever it says.
            Message::Recv(recv_packet) => {
                self.last_seen = Some(Instant::now());

                match process_packet(self, recv_packet) {
                    Ok(task) => task,
                    Err(e) => Task::done(Message::Error(e)),
                }
            }

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

            // Show an album's contents. Photos and thumbnails stream back one
            // packet at a time, so the grid fills in progressively rather than
            // waiting for the whole album.
            Message::BrowseAlbum(album) => {
                self.browsing_album = Some((album.user_id.clone(), album.id.clone()));
                Task::done(Message::Send(
                    ControlToDisplay::RequestPhotosInAlbum(album),
                ))
            }

            // Probe the display. The reply (or any other traffic) refreshes
            // `last_seen`; continued silence is what marks the link as lost.
            Message::Heartbeat => {
                if self.node.is_some() {
                    Task::done(Message::Send(ControlToDisplay::Ping))
                } else {
                    Task::none()
                }
            }

            // Drop the dead node and build a fresh one. Re-running Initialise
            // also re-requests the full state, so the UI repopulates rather than
            // being left showing whatever it held when the link died.
            Message::Reconnect => {
                self.node = None;
                self.last_seen = None;
                self.waiting_since = None;
                Task::done(Message::Initialise)
            }

            Message::OpenSettings => {
                self.show_settings = true;
                // Re-sync on open so the sliders reflect the display's current
                // state even if another control application changed it.
                Task::done(Message::Send(ControlToDisplay::RequestSettings))
            }

            Message::CloseSettings => {
                self.show_settings = false;
                Task::none()
            }

            // Slider drags only move local state; the change is sent on release
            // so dragging does not flood the display application with packets.
            Message::PeriodChanged(period) => {
                self.settings = Settings::clamped(period, self.settings.blur_duration);
                Task::none()
            }

            Message::BlurDurationChanged(blur_duration) => {
                self.settings = Settings::clamped(self.settings.period, blur_duration);
                Task::none()
            }

            Message::CommitSettings => Task::done(Message::Send(
                ControlToDisplay::SetSettings(
                    self.settings.period,
                    self.settings.blur_duration,
                ),
            )),

            // The display application's authoritative, post-clamp values
            Message::SettingsReceived(period, blur_duration) => {
                self.settings = Settings::clamped(period, blur_duration);
                Task::none()
            }

            Message::None => Task::none(),
        }
    }

    /// Poll the display so a dropped link is noticed promptly rather than only
    /// when the user next tries to do something and it silently fails.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(HEARTBEAT_INTERVAL).map(|_| Message::Heartbeat)
    }

    /// Current health of the link to the display.
    ///
    /// Judged purely on when traffic last arrived: the transport reports nothing
    /// about its own state, and a send can appear to succeed into a channel
    /// whose underlying socket has already gone.
    pub fn connection_status(&self) -> ConnectionStatus {
        if self.node.is_none() {
            return ConnectionStatus::Connecting;
        }

        match (self.last_seen, self.waiting_since) {
            // Heard from recently
            (Some(last_seen), _) if last_seen.elapsed() < LIVENESS_TIMEOUT => {
                ConnectionStatus::Live
            }
            // Heard from, but not lately
            (Some(_), _) => ConnectionStatus::Lost,
            // Never heard from; still within the grace period after connecting
            (None, Some(since)) if since.elapsed() < LIVENESS_TIMEOUT => {
                ConnectionStatus::Connecting
            }
            // Never heard from, and long enough that something is wrong
            (None, Some(_)) => ConnectionStatus::Lost,
            (None, None) => ConnectionStatus::Connecting,
        }
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let status = self.connection_status();

        let (colour, label) = match status {
            ConnectionStatus::Live => (mocha::GREEN, "Connected to display"),
            ConnectionStatus::Connecting => (mocha::BLUE, "Connecting to display…"),
            ConnectionStatus::Lost => (mocha::RED, "Disconnected — display not responding"),
        };

        let mut row = Row::new()
            // A filled dot in the status colour, so the state reads at a glance
            // without parsing the text.
            .push(text("●").color(colour).size(14))
            .push(text(label).color(mocha::TEXT).size(13))
            .spacing(8)
            .align_y(Vertical::Center)
            .padding([6, 14]);

        row = row.push(Space::new().width(Length::Fill));

        // Offer the remedy next to the diagnosis, rather than making a dead link
        // something the user can only fix by restarting the application.
        if status != ConnectionStatus::Live {
            row = row.push(
                button(text("Reconnect").size(12))
                    .on_press(Message::Reconnect)
                    .style(pill_button_style(mocha::BLUE, mocha::MAUVE))
                    .padding([4, 12]),
            );
        }

        container(row)
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
            .into()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base: Element<'_, Message> = container(
            Column::new()
                .push(self.status_bar())
                .push(
                    Row::new()
                        .push(self.users_panel())
                        .push(self.albums_panel())
                        .push(self.active_panel()),
                ),
        )
        .style(|_| container::Style {
            background: Some(Background::Color(mocha::BASE)),
            ..container::Style::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        if self.show_settings {
            iced::widget::stack([base, self.settings_page()]).into()
        } else if let Some(add_state) = &self.add_state {
            iced::widget::stack([base, self.add_modal(add_state)]).into()
        } else {
            base
        }
    }

    /// Settings page: photo period and crossfade duration.
    ///
    /// The crossfade slider's maximum tracks the period, because a fade that
    /// outlasted the period could never finish before the next photo was due.
    /// Lowering the period therefore drags the crossfade down with it.
    fn settings_page(&self) -> Element<'_, Message> {
        let max_blur = settings::max_blur_for(self.settings.period);

        let period_row = Column::new()
            .push(
                Row::new()
                    .push(text("Period").color(mocha::TEXT).size(15))
                    .push(Space::new().width(Length::Fill))
                    .push(
                        text(format!("{:.1} s", self.settings.period))
                            .color(mocha::BLUE)
                            .size(15),
                    )
                    .align_y(Vertical::Center),
            )
            .push(
                slider(PERIOD_MIN..=PERIOD_MAX, self.settings.period, Message::PeriodChanged)
                    .step(0.1_f32)
                    .on_release(Message::CommitSettings),
            )
            .push(
                text("Time each photo stays on screen (0.1 – 120 s)")
                    .color(mocha::SURFACE1)
                    .size(12),
            )
            .spacing(6);

        let blur_row = Column::new()
            .push(
                Row::new()
                    .push(text("Blur duration").color(mocha::TEXT).size(15))
                    .push(Space::new().width(Length::Fill))
                    .push(
                        text(format!("{:.1} s", self.settings.blur_duration))
                            .color(mocha::BLUE)
                            .size(15),
                    )
                    .align_y(Vertical::Center),
            )
            .push(
                slider(BLUR_MIN..=max_blur, self.settings.blur_duration, Message::BlurDurationChanged)
                    .step(0.1_f32)
                    .on_release(Message::CommitSettings),
            )
            .push(
                text(format!(
                    "Crossfade between photos (0.1 – 5 s, currently capped at {max_blur:.1} s by the period)"
                ))
                .color(mocha::SURFACE1)
                .size(12),
            )
            .spacing(6);

        let page = container(
            Column::new()
                .push(
                    Row::new()
                        .push(text("Settings").color(mocha::MAUVE).size(20))
                        .push(Space::new().width(Length::Fill))
                        .push(
                            button(text("✕").size(14).color(mocha::TEXT))
                                .on_press(Message::CloseSettings)
                                .style(ghost_button_style())
                                .padding([4, 8]),
                        )
                        .align_y(Vertical::Center),
                )
                .push(period_row)
                .push(blur_row)
                .push(
                    Row::new()
                        .push(Space::new().width(Length::Fill))
                        .push(
                            button(text("Done").size(13))
                                .on_press(Message::CloseSettings)
                                .style(pill_button_style(mocha::BLUE, mocha::MAUVE))
                                .padding([8, 16]),
                        )
                        .align_y(Vertical::Center),
                )
                .spacing(20)
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

        container(page)
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

            let label = Column::new()
                .push(
                    text(&album.name)
                        .color(if is_active { mocha::BASE } else { mocha::TEXT })
                        .size(14),
                )
                .push(
                    text(match album.num_items {
                        Some(count) => format!("{count} items"),
                        None => "— items".to_string(),
                    })
                    .color(if is_active { mocha::BASE } else { mocha::SURFACE1 })
                    .size(11),
                )
                .spacing(2);

            row = row.push(label);

            // Selecting an album both starts it on the display and opens it in
            // the browse grid, so one click does the expected thing.
            button(row)
                .on_press(Message::Batch(vec![
                    Message::SetActiveAlbum(album.clone()),
                    Message::BrowseAlbum(album.clone()),
                ]))
                .style(album_row_style(is_active))
                .width(Length::Fill)
                .into()
        }))
        .spacing(4)
        .padding([0, 8]);

        panel(
            "Albums",
            Scrollable::new(list).height(Length::Fill).into(),
            Some(
                button(text("Settings").size(13))
                    .on_press(Message::OpenSettings)
                    .style(pill_button_style(mocha::MAUVE, mocha::BLUE))
                    .padding([8, 16])
                    .width(Length::Fill)
                    .into(),
            ),
        )
    }

    /// How many photos of an album this control application holds a thumbnail
    /// for, against the album's total item count.
    ///
    /// Thumbnails arrive one packet at a time — both in response to browsing an
    /// album and unprompted as downloads finish on the display — so this ratio
    /// climbs on its own and doubles as a download progress readout.
    fn album_progress(&self, key: &(String, String)) -> (usize, Option<i64>) {
        let downloaded = self
            .photo_thumbnails
            .keys()
            .filter(|(user_id, album_id, _)| *user_id == key.0 && *album_id == key.1)
            .count();

        let total = self.albums.get(key).and_then(|album| album.num_items);

        (downloaded, total)
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
        } else {
            body = body.push(
                container(
                    text("Waiting for the display to report a photo…")
                        .color(mocha::SURFACE1)
                        .size(13),
                )
                .align_x(Horizontal::Center)
                .width(Length::Fill),
            );
        }

        // Thumbnail grid for the album being browsed
        if let Some(key) = &self.browsing_album {
            let (downloaded, total) = self.album_progress(key);

            let heading = match total {
                Some(total) => format!("{downloaded} of {total} downloaded"),
                None => format!("{downloaded} downloaded"),
            };

            body = body.push(
                Row::new()
                    .push(text("Photos").color(mocha::MAUVE).size(16))
                    .push(Space::new().width(Length::Fill))
                    .push(text(heading).color(mocha::TEXT).size(13))
                    .align_y(Vertical::Center),
            );

            let mut thumbnails = Row::new().spacing(6).width(Length::Fill);
            let mut shown = 0;

            // Iterate the album's photos so ordering is stable, rather than
            // walking the thumbnail map directly.
            if let Some(photos) = self.album_photos.get(key) {
                let mut ids: Vec<&String> = photos.keys().collect();
                ids.sort();

                for id in ids {
                    let thumb_key =
                        (key.0.clone(), key.1.clone(), id.clone());

                    if let Some(handle) = self.photo_thumbnails.get(&thumb_key) {
                        thumbnails = thumbnails.push(
                            container(image(handle.clone()).width(88).height(88))
                                .style(|_| container::Style {
                                    background: Some(Background::Color(mocha::SURFACE0)),
                                    border: Border {
                                        radius: 6.0.into(),
                                        ..Border::default()
                                    },
                                    ..container::Style::default()
                                }),
                        );
                        shown += 1;
                    }
                }
            }

            if shown == 0 {
                body = body.push(
                    text("No photos downloaded yet — they will appear as they arrive.")
                        .color(mocha::SURFACE1)
                        .size(12),
                );
            } else {
                body = body.push(thumbnails.wrap());
            }
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
