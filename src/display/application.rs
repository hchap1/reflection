use std::sync::Arc;

use bytes::Bytes;
use iced::widget::image::Handle;
use iced::{
    Element,
    Task
};

use lan_tcp::networking::node::Destination;
use lan_tcp::networking::node::Node;
use lan_tcp::networking::node::RecvPacket;
use lan_tcp::networking::node::SendPacket;
use tokio::sync::Semaphore;
use tokio_stream::wrappers::ReceiverStream;
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
    ImageDataLoaded(usize, Handle, usize, Handle),
    
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
    photos_in_album: Arc<Vec<Photo>>,
    current_photo_idx: Option<usize>,
    current_handle: Option<Handle>,
    next_photo_idx: Option<usize>,
    next_handle: Option<Handle>
}

impl Application {

    /// Build a new Application initial state
    pub fn new() -> Self {
        Self {
            node: None,
            active_album: None,
            download_permit: Arc::new(Semaphore::new(10)),
            photos_in_album: Arc::new(Vec::new()),
            current_photo_idx: None,
            current_handle: None,
            next_photo_idx: None,
            next_handle: None
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
                    }),
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
                    Task::done(Message::SynchronisePhotos),
                    Task::done(Message::SynchroniseFiles)
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
                            .chain(vec![Message::SynchroniseFiles].into_iter())
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
            Message::ReloadPhotosInActive(photos) => {
                self.photos_in_album = Arc::new(photos);
                self.current_photo_idx = if self.photos_in_album.len() == 0 { None } else { Some(0) };
                self.current_handle = None;
                self.next_photo_idx = if self.photos_in_album.len() == 0 { None } else { Some(0) };
                self.next_handle = None;
                Task::done(Message::LoadNextImage)
            },

            // Load the current and next photo handle
            Message::LoadNextImage => {
                let idx = if let Some(idx) = self.next_photo_idx {
                    idx
                } else {
                    return Task::none()
                };

                self.current_photo_idx = Some(idx);

                // Clone ARC for future
                let current_photos_vec = self.photos_in_album.clone();

                let next = self.next_handle.take();

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
                                    Ok(image) => break Some((image.into_iced(), idx)),
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
                                Ok(image) => break Some((image.into_iced(), idx)),
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
                                messages.push(Message::Error(Error::NoValidImageInAlbum));
                                return messages;
                            }
                        };

                        messages.push(Message::ImageDataLoaded(
                            current_idx,
                            current_handle,
                            next_idx,
                            next_handle
                        ));

                        messages
                    },
                    |messages| Message::Batch(messages)
                )
            },
            
            // An image has been loaded
            Message::ImageDataLoaded(current_idx, current_handle, next_idx, next_handle) => {
                self.current_photo_idx = Some(current_idx);
                self.current_handle = Some(current_handle);
                self.next_photo_idx = Some(next_idx);
                self.next_handle = Some(next_handle);
                Task::none()
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

            // Handle a completed download TODO
            Message::DownloadComplete(photo) => {
                if let Some(album) = &self.active_album {
                    if album.id == photo.album_id {
                        // TODO add to cache
                    }
                }

                Task::none()
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
