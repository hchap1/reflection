use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use bytes::Bytes;
use iced::widget::image::Handle;
use iced::widget::{container, image, stack};
use iced::{
    ContentFit,
    Element,
    Length,
    Subscription,
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
use crate::backend::database::settings::Settings;
use crate::backend::directories::image::{PREVIEW_SIZE, ReflectionImage};
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

    // Timing settings
    SettingsLoaded(Settings),
    ApplySettings(f32, f32),

    // Advance the crossfade between the outgoing and incoming photo
    FadeTick,

    // The active album has been changed
    AlbumChange(Option<Album>),
    AlbumChangeByID(String, String),
    ReloadPhotosInActive(Vec<Photo>),
    LoadNextImage,
    ImageDataLoaded(Box<LoadedSwitch>),
    SwitchFailed(u64),

    // Decode the following photo ahead of time, once the screen has settled
    PreloadNext,
    NextImageLoaded(u64, usize, ReflectionImage),
    NextImageUnavailable(u64),
    
    // Request a synchronisation of photos / files
    SynchronisePhotos,
    SynchroniseFiles,
    
    Error(Error),

}

/// Everything a completed image switch produces: the photo to show now, its
/// blurred backdrop, a small preview for the control applications, and the
/// next photo decoded ahead of time.
///
/// Grouped into a struct (and boxed in the message) because these are always
/// produced and consumed together, and a seven-field message variant would
/// otherwise be passed around positionally.
#[derive(Clone, Debug)]
pub struct LoadedSwitch {
    pub generation: u64,
    pub current_idx: usize,
    pub current: ReflectionImage,
    pub background: Handle,
    pub preview: ReflectionImage,
}

/// An in-progress crossfade from the previously displayed photo to the current
/// one. The outgoing pair of handles is held here (rather than being dropped as
/// soon as the new photo loads) so both can be drawn while the fade runs.
struct Fade {
    started: Instant,
    duration: std::time::Duration,
    previous_handle: Handle,
    previous_background: Handle,
}

/// How long to keep drawing frames after a crossfade has finished.
///
/// Ending a fade drops the outgoing photo's handles, and the renderer frees the
/// atlas space they occupied. A frame drawn while that is in progress can come
/// out damaged — and because a still photo needs no redraws, that damaged frame
/// would otherwise stay on screen until the next photo change several seconds
/// later. Continuing to draw briefly guarantees the last frame the viewer is
/// left looking at is a correct one.
const SETTLE_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

impl Fade {
    /// 0.0 at the start of the fade, 1.0 once it has finished.
    fn progress(&self) -> f32 {
        let duration = self.duration.as_secs_f32();
        if duration <= 0.0 {
            return 1.0;
        }
        (self.started.elapsed().as_secs_f32() / duration).clamp(0.0, 1.0)
    }

    fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }
}

pub struct Application {
    node: Option<Arc<Node>>,
    pub active_album: Option<Album>,
    download_permit: Arc<Semaphore>,

    // Photo cycling period and crossfade duration, persisted in the database
    // and editable from the control application.
    pub settings: Settings,
    fade: Option<Fade>,

    // Deadline until which frames keep being drawn after a crossfade ends.
    // See `SETTLE_DURATION`.
    settling_until: Option<Instant>,

    // Cycle timing. `cycle_generation` and `first_delay` together identify the
    // current cycling timer; changing either rebuilds the subscription. They are
    // recomputed only when a photo changes or the period changes, so the
    // subscription stays stable in between rather than being torn down on every
    // update. `last_switch` anchors the cycle so a new period is measured from
    // when the current photo actually appeared.
    last_switch: Instant,
    cycle_generation: u64,
    first_delay: std::time::Duration,

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
    switch_generation: u64,

    // Guards against stacking up lookahead decodes
    preload_in_flight: bool
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
            settings: Settings::default(),
            fade: None,
            last_switch: Instant::now(),
            cycle_generation: 0,
            first_delay: Settings::default().period_duration(),
            photos_in_album: Arc::new(Mutex::new(Vec::new())),
            current_photo_idx: None,
            current_handle: None,
            background_handle: None,
            next_photo_idx: None,
            next_handle: None,
            switch_in_flight: false,
            switch_generation: 0,
            preload_in_flight: false,
            settling_until: None
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
                ])
                // Photo cycling is driven by `subscription` rather than a stream
                // started here, so that changing the period from the control
                // application re-times the cycle immediately.
            },

            // Database is ready — run authentication before anything else
            Message::DatabaseReady => {
                Task::batch(vec![
                    Task::future(Authentication::create_initial())
                    .map(|res| match res {
                        Ok(()) => Message::AuthenticationReady,
                        Err(e) => Message::Error(e)
                    }),
                    // Restore the persisted timing settings now the tables exist
                    Task::future(Settings::load())
                    .map(|res| match res {
                        Ok(settings) => Message::SettingsLoaded(settings),
                        Err(e) => Message::Error(e)
                    }),
                ])
            },

            // Timing settings restored from (or written to) the database
            Message::SettingsLoaded(settings) => {
                self.settings = settings;
                self.retime_cycle();
                Task::none()
            },

            // A control application changed the settings — clamp, persist, and
            // echo the clamped result back to every control application so their
            // sliders reflect what was actually stored.
            Message::ApplySettings(period, blur_duration) => {
                let settings = Settings::clamped(period, blur_duration);
                self.settings = settings;
                self.retime_cycle();

                Task::future(async move { settings.save().await })
                    .map(|res| match res {
                        Ok(saved) => Message::Batch(vec![
                            Message::SettingsLoaded(saved),
                            Message::Send(DisplayToControl::SettingsInformation(
                                saved.period,
                                saved.blur_duration
                            ))
                        ]),
                        Err(e) => Message::Error(e)
                    })
            },

            // Drive the crossfade; drop it once it has run its course so the
            // per-frame subscription can go idle again.
            Message::FadeTick => {
                if self.fade.as_ref().is_some_and(Fade::is_complete) {
                    self.fade = None;

                    // Keep drawing for a moment while the renderer releases the
                    // outgoing photo's atlas space, so the frame left on screen
                    // is a settled one.
                    self.settling_until = Some(Instant::now() + SETTLE_DURATION);

                    // The animation is over and the screen is static again —
                    // now is the cheapest moment to decode the next photo.
                    return Task::done(Message::PreloadNext);
                }

                // Stop the extra redraws once things have settled
                if self.settling_until.is_some_and(|until| Instant::now() >= until) {
                    self.settling_until = None;
                }

                Task::none()
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
                    // Any lookahead decode still running belongs to the old
                    // album; its result will be discarded by generation, so free
                    // the guard for the new one straight away.
                    self.preload_in_flight = false;

                    if empty {
                        // Nothing to show for this album — clear the display.
                        self.current_photo_idx = None;
                        self.next_photo_idx = None;
                        self.current_handle = None;
                        self.background_handle = None;
                        self.fade = None;
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

                // Clone ARC for future
                let current_photos_vec = match self.photos_in_album.lock() {
                    Ok(current_photos_vec) => current_photos_vec.clone(),
                    _ => return Task::done(Message::Error(Error::MutexLockFailed))
                };

                if current_photos_vec.is_empty() {
                    return Task::none();
                }

                // Normally the lookahead has already chosen and decoded the next
                // photo. If it has not finished yet — a very short period, or a
                // slow disk — fall back to the one after the current photo and
                // let the task below decode it inline.
                let idx = match (self.next_photo_idx, self.current_photo_idx) {
                    (Some(idx), _) => idx,
                    (None, Some(current)) => (current + 1) % current_photos_vec.len(),
                    (None, None) => 0
                };

                self.current_photo_idx = Some(idx);

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

                        // Derive the backdrop and the control-application preview
                        // from the already-decoded image on a blocking-friendly
                        // thread rather than reading the file a second time.
                        //
                        // The preview is produced first and the blur taken from
                        // it: blurring a 512px copy costs a fraction of blurring
                        // the full display-sized image, and the result is
                        // downscaled to 64px and blurred anyway.
                        let derive_from = current_handle.clone();
                        let derived = tokio::task::spawn_blocking(move || {
                            let preview = derive_from.fitted(PREVIEW_SIZE);
                            let background = preview.blurred_background();
                            (background, preview)
                        }).await;

                        let (background, preview) = match derived {
                            Ok(derived) => derived,
                            Err(_) => (
                                current_handle.clone().into_iced(),
                                current_handle.clone()
                            )
                        };

                        messages.push(Message::ImageDataLoaded(Box::new(LoadedSwitch {
                            generation,
                            current_idx,
                            current: current_handle,
                            background,
                            preview
                        })));

                        messages
                    },
                    Message::Batch
                )
            },

            // Decode the photo that follows the one on screen, so the next
            // switch is a cache hit rather than a decode.
            //
            // Deliberately kept out of the switch itself: decoding a full
            // resolution photo costs over a second of CPU, and doing it as part
            // of the switch delayed the new photo appearing and starved the
            // crossfade of the frames it needed to animate smoothly. The photo
            // is not needed for another whole period, so it is decoded once the
            // screen has settled instead.
            Message::PreloadNext => {
                if self.preload_in_flight || self.next_handle.is_some() {
                    return Task::none();
                }

                let from_idx = match self.current_photo_idx {
                    Some(idx) => idx,
                    None => return Task::none()
                };

                let photos = match self.photos_in_album.lock() {
                    Ok(photos) => photos.clone(),
                    _ => return Task::done(Message::Error(Error::MutexLockFailed))
                };

                if photos.is_empty() {
                    return Task::none();
                }

                let generation = self.switch_generation;
                self.preload_in_flight = true;

                Task::perform(
                    async move {
                        let count = photos.len();
                        let mut messages = Vec::new();

                        // Walk forward from the current photo, wrapping, until
                        // one decodes or every candidate has been tried.
                        for offset in 1..=count {
                            let idx = (from_idx + offset) % count;

                            let photo = match photos.get(idx) {
                                Some(photo) => photo.clone(),
                                None => continue
                            };

                            match ReflectionImage::load(photo).await {
                                Ok(image) => {
                                    messages.push(Message::NextImageLoaded(
                                        generation, idx, image
                                    ));
                                    return messages;
                                }
                                Err(e) => messages.push(Message::Error(e))
                            }
                        }

                        messages.push(Message::NextImageUnavailable(generation));
                        messages
                    },
                    Message::Batch
                )
            },

            // The lookahead decode finished
            Message::NextImageLoaded(generation, idx, image) => {
                self.preload_in_flight = false;

                // Discard a decode that belongs to an album since replaced
                if generation == self.switch_generation {
                    self.next_photo_idx = Some(idx);
                    self.next_handle = Some(image);
                }

                Task::none()
            },

            // Nothing in the album could be decoded; drop the guard so a later
            // attempt (e.g. after a download completes) can try again.
            Message::NextImageUnavailable(generation) => {
                if generation == self.switch_generation {
                    self.preload_in_flight = false;
                }
                Task::none()
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
            Message::ImageDataLoaded(loaded) => {
                let LoadedSwitch {
                    generation,
                    current_idx,
                    current,
                    background,
                    preview
                } = *loaded;

                // The active album changed while this switch was in flight — its
                // index/handles no longer refer to anything meaningful, discard it.
                if generation != self.switch_generation {
                    return Task::none();
                }

                // Control applications get the small preview, never the full
                // size image — a display-resolution photo is far too large to
                // push over the network on every switch.
                //
                // Scoped so the lock is released before the rest of this handler
                // touches `self` again.
                let message = {
                    let current_photos_vec = match self.photos_in_album.lock() {
                        Ok(current_photos_vec) => current_photos_vec,
                        _ => return Task::done(Message::Error(Error::MutexLockFailed))
                    };

                    match current_photos_vec.get(current_idx) {
                        Some(photo) => Message::Send(
                            DisplayToControl::ActivePhoto(photo.clone(), preview)
                        ),
                        None => Message::None
                    }
                };

                let current_handle = current.into_iced();

                // Start a crossfade from whatever is currently on screen. There
                // is nothing to fade from on the very first photo, so that one
                // appears immediately rather than fading up from a blank screen.
                self.fade = match (self.current_handle.take(), self.background_handle.take()) {
                    (Some(previous_handle), Some(previous_background)) => Some(Fade {
                        started: Instant::now(),
                        duration: self.settings.blur_duration(),
                        previous_handle,
                        previous_background
                    }),
                    _ => None
                };

                let fading = self.fade.is_some();

                self.background_handle = Some(background);
                self.current_photo_idx = Some(current_idx);
                self.current_handle = Some(current_handle);
                // The cached decode has been consumed by this switch
                self.next_photo_idx = None;
                self.next_handle = None;
                self.switch_in_flight = false;

                // Anchor the next cycle to when this photo actually appeared, so
                // a slow decode does not shorten the time it stays on screen.
                self.last_switch = Instant::now();
                self.retime_cycle();

                // With a crossfade running, the lookahead decode waits until it
                // finishes (see FadeTick) so the animation gets the CPU. Without
                // one there is nothing to protect, so start immediately.
                if fading {
                    Task::done(message)
                } else {
                    Task::batch(vec![
                        Task::done(message),
                        Task::done(Message::PreloadNext)
                    ])
                }
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
                    photos_vec.push(photo.clone())
                }

                // Push the newly available photo to the control applications so
                // an album appears to fill in as it downloads, rather than only
                // when the whole synchronisation finishes.
                let announce = Task::batch(vec![
                    Task::done(Message::Send(DisplayToControl::ReturnPhoto(photo.clone()))),
                    Task::done(Message::SendThumbnail(photo))
                ]);

                if self.current_handle.is_none() {
                    Task::batch(vec![announce, Task::done(Message::LoadNextImage)])
                } else {
                    announce
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

    /// Re-anchor the cycling timer against the photo currently on screen.
    ///
    /// Called whenever the period changes so the new value applies immediately:
    /// the next switch lands one *new* period after the current photo appeared,
    /// rather than after the old interval finishes running down. If that moment
    /// has already passed — the period was shortened below the time already
    /// elapsed — the delay collapses to zero and the photo changes at once.
    fn retime_cycle(&mut self) {
        self.first_delay = self.settings
            .period_duration()
            .saturating_sub(self.last_switch.elapsed());
        self.cycle_generation = self.cycle_generation.wrapping_add(1);
    }

    /// Timed events the application reacts to.
    ///
    /// The cycling timer's identity is `(cycle_generation, first_delay, period)`,
    /// all of which change only on a photo switch or a settings change — so the
    /// subscription is stable between those events, and rebuilt precisely when
    /// the timing needs to change. The per-frame fade ticks are only subscribed
    /// to while a crossfade is actually running, so an idle photo frame is not
    /// redrawing continuously.
    pub fn subscription(&self) -> Subscription<Message> {
        let cycle = Subscription::run_with(
            (self.cycle_generation, self.first_delay, self.settings.period_duration()),
            |(_, first_delay, period)| {
                let first_delay = *first_delay;
                let period = *period;

                IntervalStream::new(tokio::time::interval_at(
                    tokio::time::Instant::now() + first_delay,
                    period
                ))
            }
        ).map(|_| Message::LoadNextImage);

        // Frames are drawn while a crossfade runs, and for a short while after it
        // ends (see `SETTLE_DURATION`). A photo sitting still needs none.
        if self.fade.is_some() || self.settling_until.is_some() {
            Subscription::batch(vec![
                cycle,
                iced::window::frames().map(|_| Message::FadeTick)
            ])
        } else {
            cycle
        }
    }

    /// View logic of the application
    ///
    /// Renders the current photo centered over a full-screen, blurred copy of itself
    /// so there is never a hard edge / letterboxed gap around images that don't match
    /// the screen's aspect ratio.
    ///
    /// While a crossfade is running the outgoing photo is drawn underneath at full
    /// opacity and the incoming one fades in on top of it, so the transition never
    /// dips through a blank frame.
    pub fn view(&self) -> Element<'_, Message> {
        match (&self.current_handle, &self.background_handle) {
            (Some(handle), Some(background)) => {
                let mut layers: Vec<Element<'_, Message>> = Vec::new();

                let opacity = match &self.fade {
                    Some(fade) => {
                        layers.push(photo_layers(
                            &fade.previous_background,
                            &fade.previous_handle,
                            1.0
                        ));
                        fade.progress()
                    }
                    None => 1.0
                };

                layers.push(photo_layers(background, handle, opacity));

                stack(layers).into()
            }
            _ => container(iced::widget::text("No selected handle..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
    }
}

/// One photo as it is composited on screen: a blurred cover-fitted backdrop with
/// the photo itself contained and centred over it, both at the given opacity so
/// the pair fades as a single unit.
fn photo_layers<'a>(
    background: &Handle,
    handle: &Handle,
    opacity: f32,
) -> Element<'a, Message> {
    stack![
        image(background)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Cover)
            .opacity(opacity),
        container(
            image(handle)
                .content_fit(ContentFit::Contain)
                .opacity(opacity)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
    ].into()
}
