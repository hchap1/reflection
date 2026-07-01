use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use bytes::Bytes;
use iced::widget::image::Handle;
use iced::widget::{container, image, stack};
use iced::{
    ContentFit,
    Element,
    Length,
    Task
};

use lan_tcp::networking::node::Destination;
use lan_tcp::networking::node::Node;
use lan_tcp::networking::node::RecvPacket;
use lan_tcp::networking::node::SendPacket;
use tokio::sync::Semaphore;
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};
use rkyv::to_bytes;

use crate::IDENTIFIER;
use crate::PORT;
use crate::backend::database::authentication_storage::Authentication;
use crate::backend::database::database_backend::Database;
use crate::backend::database::sql::Album;
use crate::backend::database::sql::Photo;
use crate::backend::database::sql::SQL;
use crate::backend::directories::image::ReflectionImage;
use crate::backend::networking::network_message::DisplayToControl;
use crate::display::check_all::synchronise_files;
use crate::display::check_all::synchronise_photos;
use crate::display::process_packet::process_packet;
use crate::error::Error;

#[derive(Clone, Debug)]
pub enum AuthenticatedMessage {
    Download(Photo)
}

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Initialise database and networking
    Initialise,
    DatabaseReady,
    AuthenticationReady,
    NodeCreated(Arc<Node>),

    // Incoming TCP packet (recv packet)
    RecvPacket(RecvPacket),

    // Send to all Control applications
    Send(DisplayToControl),

    // Produce many messages all at once,
    Batch(Vec<Message>),

    // Load and send thumbnail to the control application
    SendThumbnail(Photo),

    // Messages that require authentication
    // Run an authenticated messaged by user_id
    Authenticate(AuthenticatedMessage, String),
    AuthenticatedMessage(String, AuthenticatedMessage),
    DownloadComplete(Photo),

    // The active album has been changed
    AlbumChange(Option<Album>),
    AlbumChangeByID(String, String),
    ReloadPhotosInActive(Vec<Photo>),
    LoadNextImage,
    ImageDataLoaded(u64, usize, ReflectionImage, Handle, usize, ReflectionImage),
    SwitchFailed(u64),
    
    // Request a synchronisation of photos / files
    SynchronisePhotos,
    SynchroniseFiles,
    
    Error(Error),

}

pub struct Application {
    node: Option<Arc<Node>>,
    pub active_album: Option<Album>,
    download_permit: Arc<Semaphore>,

    // Manage display
    pub photos_in_album: Arc<Mutex<Vec<Photo>>>,
    pub current_photo_idx: Option<usize>,
    current_handle: Option<Handle>,
    background_handle: Option<Handle>,
    next_photo_idx: Option<usize>,
    next_handle: Option<ReflectionImage>,

    // Guards against overlapping / stale image switches (e.g. a switch that's
    // still decoding when the next tick fires, or one that belongs to an album
    // that has since been replaced).
    switch_in_flight: bool,
    switch_generation: u64
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    /// Build a new Application initial state
    pub fn new() -> Self {
        Self {
            node: None,
            active_album: None,
            download_permit: Arc::new(Semaphore::new(10)),
            photos_in_album: Arc::new(Mutex::new(Vec::new())),
            current_photo_idx: None,
            current_handle: None,
            background_handle: None,
            next_photo_idx: None,
            next_handle: None,
            switch_in_flight: false,
            switch_generation: 0
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
                        Ok(()) => Message::DatabaseReady,
                        Err(e) => Message::Error(e)
                    }),
                    Task::future(Node::spawn_server(IDENTIFIER, PORT, 100))
                    .map(|res| match res {
                        Ok(node) => Message::NodeCreated(Arc::new(node)),
                        Err(e) => Message::Error(e.into())
                    }),
                    // Cycle to the next image in the active album once a second.
                    // `interval_at` (rather than `interval`) skips the immediate
                    // first tick, so we don't double-advance right on startup.
                    Task::stream(IntervalStream::new(
                        tokio::time::interval_at(
                            tokio::time::Instant::now() + Duration::from_secs(1),
                            Duration::from_secs(1)
                        )
                    )).map(|_| Message::LoadNextImage),
                ])
            },

            // Database is ready — run authentication before anything else
            Message::DatabaseReady => {
                Task::future(Authentication::create_initial())
                .map(|res| match res {
                    Ok(()) => Message::AuthenticationReady,
                    Err(e) => Message::Error(e)
                })
            },

            // Authentication is ready — start remaining DB-dependent tasks
            Message::AuthenticationReady => {
                Task::batch(vec![
                    // Check to see if database has a stored active album to resume
                    // This is a length prefixed setting format abcd:DATA where the first abcd
                    // characters belong to the album_id, and the rest are user_id
                    Task::future(SQL::select_setting_by_name("active_album"))
                    .map(|res| match res {
                        Ok(value) => match value {
                            Some(hybrid_id) => {
                                let mut chars = hybrid_id
                                    .chars();

                                let size = match chars
                                    .by_ref()
                                    .take(4)
                                    .collect::<String>()
                                    .parse::<usize>() {
                                    Ok(size) => size,
                                    Err(_) => return Message::Error(Error::InvalidAlbum)
                                };

                                let album_id = chars
                                    .by_ref()
                                    .take(size)
                                    .collect::<String>();

                                let user_id = chars
                                    .collect::<String>();

                                Message::AlbumChangeByID(album_id, user_id)
                            },
                            None => Message::None
                        }
                        Err(e) => Message::Error(e)
                    }),
                    Task::done(Message::SynchronisePhotos)
                ])
            },

            // Update the database photo records to match that of the current albums
            // Then, synchronise files
            Message::SynchronisePhotos => Task::future(synchronise_photos())
                .map(|res| match res {
                    Ok(issues) => Message::Batch(
                        issues.into_iter()
                            .map(|(string, res)| Message::Error(
                                Error::DownloadError(format!("{res:?} : {string}"))
                            ))
                            .chain(vec![Message::SynchroniseFiles])
                            .collect()
                    ),
                    Err(e) => Message::Error(e)
                }),

            // Downloads all files for photos where the thumbnail / image do not exist
            Message::SynchroniseFiles => Task::future(synchronise_files(self.download_permit.clone()))
                .map(|res| match res {
                    Ok(issues) => Message::Batch(
                        issues.into_iter()
                            .map(|(string, res)| Message::Error(
                                Error::DownloadError(format!("{res:?} : {string}"))
                            ))
                            .collect()
                    ),
                    Err(e) => Message::Error(e)
                }),

            // Once the node has been successfully initialised
            // Take the receiver from the node and keep it in a stream
            Message::NodeCreated(mut node) => {

                println!("Received creation of node.");

                // Try and mutate the node to retrieve the receiver
                // This fails if something else has a weak reference
                let task = if let Some(node) = Arc::get_mut(&mut node) {
                    match node.take_receiver() {
                        Some(receiver) => Task::stream(ReceiverStream::new(receiver))
                            .map(Message::RecvPacket),
                        None => Task::done(Message::Error(Error::TcpReceiverMissing))
                    }
                } else {
                    Task::done(Message::Error(Error::CouldNotMutateNodeArc))
                };
                self.node = Some(node);
                println!("Set node, spawning stream task.");
                task
            }

            // Process an outgoing tcp packet
            Message::Send(display_to_control) => {
                match self.node.as_ref() {
                    Some(node_ref) => {
                        let sender = node_ref.clone_sender();
                        Task::future(async move {
                            // Serialising a full-resolution image is CPU bound and can
                            // stall the UI thread if run inline in `update` — push it
                            // onto a blocking-friendly thread instead.
                            let aligned_vec = tokio::task::spawn_blocking(move || {
                                to_bytes::<rkyv::rancor::Error>(&display_to_control)
                            }).await.map_err(Error::from)?.map_err(Error::from)?;

                            sender.send(
                                SendPacket {
                                    data: Bytes::from_owner(aligned_vec),
                                    destination: Destination::All
                                }
                            ).await.map_err(|_| lan_tcp::error::Error::MpscChannelFailed.into())
                        }).map(|res: Result<(), Error>| match res {
                            Ok(()) => Message::None,
                            Err(e) => Message::Error(e)
                        })
                    },
                    None => Task::done(Message::Error(Error::MissingNode))
                }
            },

            // The album has been changed, reload display
            Message::AlbumChange(album) => {
                self.active_album = album;

                match self.active_album.as_ref() {
                    Some(album) => Task::perform(
                        SQL::select_photos_by_album(album.id.clone(), album.user_id.clone()),
                        |res| match res {
                            Ok(photos) => Message::ReloadPhotosInActive(photos),
                            Err(e) => Message::Error(e)
                        }
                    ),
                    None => {
                        Task::done(Message::ReloadPhotosInActive(Vec::new()))
                    }
                }
            },

            // A new set of photos is to be loaded for the active album
            Message::ReloadPhotosInActive(mut photos) => {
                if let Ok(mut photos_in_album) = self.photos_in_album.lock() {
                    let empty = photos.is_empty();

                    // Bump the generation and drop the switch-in-flight guard so any
                    // result from a previously started switch (which now refers to a
                    // stale album/index) is ignored rather than clobbering state, and
                    // so the LoadNextImage below is free to start immediately.
                    self.switch_generation += 1;
                    self.switch_in_flight = false;
                    self.next_handle = None;

                    if empty {
                        // Nothing to show for this album — clear the display.
                        self.current_photo_idx = None;
                        self.next_photo_idx = None;
                        self.current_handle = None;
                        self.background_handle = None;
                    } else {
                        // Keep showing the previous image (if any) until the new
                        // album's first image has actually loaded.
                        self.current_photo_idx = Some(0);
                        self.next_photo_idx = Some(0);
                    }

                    photos_in_album.clear();
                    photos_in_album.append(&mut photos);
                    Task::done(Message::LoadNextImage)
                } else {
                    Task::done(Message::Error(Error::MutexLockFailed))
                }

            },

            // Load the current and next photo handle
            Message::LoadNextImage => {
                // A switch is already decoding/blurring — let it finish rather than
                // starting an overlapping one (this is what caused the flicker: a
                // slow switch was still in flight when the next tick fired, and the
                // two completions could arrive out of order).
                if self.switch_in_flight {
                    return Task::none();
                }

                let idx = if let Some(idx) = self.next_photo_idx {
                    idx
                } else {
                    return Task::none()
                };

                self.current_photo_idx = Some(idx);

                // Clone ARC for future
                let current_photos_vec = match self.photos_in_album.lock() {
                    Ok(current_photos_vec) => current_photos_vec.clone(),
                    _ => return Task::done(Message::Error(Error::MutexLockFailed))
                };

                let next = self.next_handle.take();
                let generation = self.switch_generation;
                self.switch_in_flight = true;

                Task::perform(
                    async move {

                        let original_idx = idx;
                        let mut idx = idx;
                        let mut messages = Vec::new();

                        let current = match next {
                            Some(next) => Some((next, idx)),
                            None => loop {

                                // Retrieve the current photo
                                let current_photo = match current_photos_vec.get(idx) {
                                    Some(current_photo) => current_photo,
                                    None => { idx = 0; continue }
                                };

                                // See if the image for the current photo can be loaded
                                match ReflectionImage::load(current_photo.clone()).await {
                                    Ok(image) => break Some((image, idx)),
                                    Err(e) => messages.push(Message::Error(e))
                                }

                                idx += 1;

                                // If we have checked every idx
                                if idx == original_idx {
                                    break None
                                }
                            }
                        };

                        let (current_handle, current_idx) = match current {
                            Some(current) => current,
                            None => {
                                messages.push(Message::SwitchFailed(generation));
                                messages.push(Message::Error(Error::NoValidImageInAlbum));
                                return messages;
                            }
                        };

                        // Find the next valid image to cache
                        idx = current_idx + 1;
                        let maybe_next = loop {

                            // Retrieve the current photo
                            let current_photo = match current_photos_vec.get(idx) {
                                Some(current_photo) => current_photo,
                                None => { idx = 0; continue }
                            };

                            // See if the image for the current photo can be loaded
                            match ReflectionImage::load(current_photo.clone()).await {
                                Ok(image) => break Some((image, idx)),
                                Err(e) => messages.push(Message::Error(e))
                            }

                            idx += 1;

                            // If we have checked every idx
                            if idx == original_idx {
                                break None
                            }
                        };

                        let (next_handle, next_idx) = match maybe_next {
                            Some(next) => next,
                            None => {
                                messages.push(Message::SwitchFailed(generation));
                                messages.push(Message::Error(Error::NoValidImageInAlbum));
                                return messages;
                            }
                        };

                        // Blurring is CPU bound, so run it on a blocking-friendly thread
                        let background_image = current_handle.clone();
                        let background_handle = match tokio::task::spawn_blocking(
                            move || background_image.blurred_background()
                        ).await {
                            Ok(handle) => handle,
                            Err(_) => current_handle.clone().into_iced()
                        };

                        messages.push(Message::ImageDataLoaded(
                            generation,
                            current_idx,
                            current_handle,
                            background_handle,
                            next_idx,
                            next_handle
                        ));

                        messages
                    },
                    Message::Batch
                )
            },

            // A switch (current + next lookahead) failed to find any valid image.
            // Release the in-flight guard so the next tick can try again — but only
            // if this failure still belongs to the current album; a stale failure
            // from a since-replaced album should not affect the new one's guard.
            Message::SwitchFailed(generation) => {
                if generation == self.switch_generation {
                    self.switch_in_flight = false;
                }
                Task::none()
            }

            // An image has been loaded
            Message::ImageDataLoaded(generation, current_idx, current_image, background_handle, next_idx, next_image) => {

                // The active album changed while this switch was in flight — its
                // index/handles no longer refer to anything meaningful, discard it.
                if generation != self.switch_generation {
                    return Task::none();
                }

                let current_photos_vec = match self.photos_in_album.lock() {
                    Ok(current_photos_vec) => current_photos_vec,
                    _ => return Task::done(Message::Error(Error::MutexLockFailed))
                };


                let message = match current_photos_vec.get(current_idx) {
                    Some(photo) => Message::Send(
                        DisplayToControl::ActivePhoto(photo.clone(), current_image.clone())
                    ),
                    None => Message::None
                };

                let current_handle = current_image.into_iced();
                self.background_handle = Some(background_handle);
                self.current_photo_idx = Some(current_idx);
                self.current_handle = Some(current_handle);
                self.next_photo_idx = Some(next_idx);
                self.next_handle = Some(next_image);
                self.switch_in_flight = false;
                Task::done(message)
            }

            // Set album by ID
            Message::AlbumChangeByID(album_id, user_id) => {
                Task::perform(
                    SQL::select_album_by_id(album_id, user_id),
                    |res| match res {
                        Ok(maybe_album) => match maybe_album {
                            Some(album) => Message::AlbumChange(Some(album)),
                            None => Message::Error(Error::InvalidAlbum)
                        },
                        Err(e) => Message::Error(e)
                    }
                )
            }

            // Load the thumbnail from storage
            // Then use Send to send to control application
            Message::SendThumbnail(photo) => Task::perform(
               ReflectionImage::load_thumbnail(photo.clone()),
               |res| match res {
                    Ok(thumbnail) => Message::Send(
                        DisplayToControl::ReturnPhotoWithThumbnail(photo, thumbnail)
                    ),
                    Err(e) => Message::Error(e)
               }
            ),

            // Authenticate then execute
            Message::Authenticate(message, user_id) => {
                Task::perform(
                    async {
                        let mut user = SQL::select_user_by_id(user_id)
                            .await?
                            .ok_or(Error::NoSuchUserInDatabase)?;

                        Authentication::get_access_token(&mut user).await
                    },
                    |res| match res {
                        Ok(token) => Message::AuthenticatedMessage(token, message),
                        Err(e) => Message::Error(e)
                    }
                )
            }

            // Process a message that requires authentication
            Message::AuthenticatedMessage(access_token, message) => match message {
                AuthenticatedMessage::Download(photo) => {
                    let semaphore_arc = self.download_permit.clone();
                    let photo_clone = photo.clone();
                    Task::perform(
                        async {
                            let permit = semaphore_arc.acquire_owned().await?;
                            ReflectionImage::download(permit, access_token, photo_clone)
                                .await
                        },
                        |res| match res {
                            Ok(()) => Message::DownloadComplete(photo),
                            Err(e) => Message::Error(e)
                        }
                    )
                }
            }

            // Handle a completed download
            Message::DownloadComplete(photo) => {
                if let Some(album) = &self.active_album
                    && album.id == photo.album_id
                    && let Ok(mut photos_vec) = self.photos_in_album.lock()
                {
                    photos_vec.push(photo)
                }

                if self.current_handle.is_none() {
                    Task::done(Message::LoadNextImage)
                } else {
                    Task::none()
                }
            }

            // Process an incoming tcp packet
            Message::RecvPacket(recv_packet) => match process_packet(self, recv_packet) {
                Ok(task) => task,
                Err(e) => Task::done(Message::Error(e))
            },

            // Produce each message individually
            Message::Batch(messages) => {
                Task::batch(messages
                    .into_iter()
                    .map(Task::done)
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
    ///
    /// Renders the current photo centered over a full-screen, blurred copy of itself
    /// so there is never a hard edge / letterboxed gap around images that don't match
    /// the screen's aspect ratio.
    pub fn view(&self) -> Element<'_, Message> {
        match (&self.current_handle, &self.background_handle) {
            (Some(handle), Some(background)) => stack![
                image(background)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(ContentFit::Cover),
                container(
                    image(handle).content_fit(ContentFit::Contain)
                )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            ].into(),
            _ => container(iced::widget::text("No selected handle..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
    }
}
