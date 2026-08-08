use std::collections::HashMap;

use crate::backend::database::sql::ArchivedPhoto;
use crate::backend::database::sql::Photo;
use crate::backend::networking::network_message::ArchivedDisplayToControl;
use crate::backend::networking::network_message::ArchivedObfuscatedUser;
use crate::backend::networking::network_message::ControlToDisplay;
use crate::backend::networking::network_message::ObfuscatedUser;
use crate::display::process_packet::own_album;
use crate::error::Res;
use crate::control::application::Application;
use crate::control::application::Message;
use crate::backend::networking::network_message::DisplayToControl;

use iced::Task;

use iced::widget::image::Handle;
use lan_tcp::networking::node::RecvPacket;
use rkyv::Archived;
use rkyv::access;
use rkyv::option::ArchivedOption;

fn own_photo(photo: &ArchivedPhoto) -> Photo {
    Photo {
        id: photo.id.to_string(),
        album_id: photo.album_id.to_string(),
        user_id: photo.user_id.to_string(),
        name: photo.name.to_string(),
        created_date_time: photo.created_date_time.into(),
        width: photo.width.into(),
        height: photo.height.into(),
        latitude: match photo.latitude {
            ArchivedOption::Some(thing) => Some(thing.into()),
            ArchivedOption::None => None
        },
        longitude: match photo.longitude {
            ArchivedOption::Some(thing) => Some(thing.into()),
            ArchivedOption::None => None
        },
        altitude: match photo.altitude {
            ArchivedOption::Some(thing) => Some(thing.into()),
            ArchivedOption::None => None
        },
        size: match photo.size {
            ArchivedOption::Some(thing) => Some(thing.into()),
            ArchivedOption::None => None
        }
    }
}

fn own_user(user: &ArchivedObfuscatedUser) -> ObfuscatedUser {
    ObfuscatedUser {
        id: user.id.to_string(),
        name: match &user.name {
            ArchivedOption::Some(name) => Some(name.to_string()),
            ArchivedOption::None => None
        },
        email: match &user.email {
            ArchivedOption::Some(email) => Some(email.to_string()),
            ArchivedOption::None => None
        },
        expiry_date_time: user.expiry_date_time.into()
    }
}

pub fn process_packet(application: &mut Application, recv_packet: RecvPacket) -> Res<Task<Message>> {
    // It is expected that recv_packet contains a ControlToDisplay
    let display_to_control: &Archived<DisplayToControl> = access::<
        Archived<DisplayToControl>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

    Ok(
        match display_to_control {
            ArchivedDisplayToControl::UserInformation(obfuscated_user) => {
                application.users.insert(obfuscated_user.id.to_string(), own_user(obfuscated_user));
                Task::none()
            },

            ArchivedDisplayToControl::AlbumInformation(album) => {
                application.albums.insert((album.user_id.to_string(), album.id.to_string()), own_album(album));
                Task::done(
                    Message::Send(ControlToDisplay::RequestAlbumCover(own_album(album)))
                )
            },

            ArchivedDisplayToControl::SelectedAlbum(album) => {
                match album {
                    ArchivedOption::Some(album) => {
                        application.active_album = Some((album.user_id.to_string(), album.id.to_string()));
                        application.albums.insert((album.user_id.to_string(), album.id.to_string()), own_album(album));

                        // Open the playing album in the browse grid on connect,
                        // so its thumbnails load without needing a click.
                        if application.browsing_album.is_none() {
                            return Ok(Task::done(Message::BrowseAlbum(own_album(album))));
                        }

                        Task::none()
                    },
                    ArchivedOption::None => {
                        application.active_album = None;
                        Task::none()
                    }
                }
            },

            ArchivedDisplayToControl::ReturnAlbumCover(album, cover) => {
                let handle = Handle::from_rgba(cover.width.into(), cover.height.into(), cover.data.to_owned());
                application.album_covers.insert((album.user_id.to_string(), album.id.to_string()), handle);
                application.albums.insert((album.user_id.to_string(), album.id.to_string()), own_album(album));
                Task::none()
            },

            ArchivedDisplayToControl::ReturnPhoto(photo) => {
                let key = (photo.user_id.to_string(), photo.album_id.to_string());
                let photo = own_photo(photo);
                _ = match application.album_photos.get_mut(&key) {
                    Some(hashmap) => hashmap.insert(photo.id.clone(), photo),
                    None => {
                        let mut hashmap = HashMap::new();
                        hashmap.insert(photo.id.clone(), photo);
                        application.album_photos.insert(key, hashmap);
                        None
                    }
                };

                Task::none()
            },

            ArchivedDisplayToControl::ReturnPhotoWithThumbnail(photo, image) => {
                let key = (photo.user_id.to_string(), photo.album_id.to_string(), photo.id.to_string());
                let handle = Handle::from_rgba(
                    image.width.into(),
                    image.height.into(),
                    image.data.to_owned()
                );

                application.photo_thumbnails.insert(key, handle);
                Task::none()
            },

            ArchivedDisplayToControl::ActivePhoto(_, image) => {
                let handle = Handle::from_rgba(
                    image.width.into(),
                    image.height.into(),
                    image.data.to_owned()
                );

                application.active_image = Some(handle);
                Task::none()
            },

            ArchivedDisplayToControl::ReturnAlbumsBelongingToUser(album, exists) => {
                let exists = *exists;
                let album = own_album(album);
                match application.add_state.as_mut() {
                    Some(vec) => vec.push((exists, album)),
                    None => application.add_state = Some(vec![(exists, album)])
                }
                Task::none()
            },

            // Liveness only — `Message::Recv` has already recorded its arrival.
            ArchivedDisplayToControl::Pong => Task::none(),

            ArchivedDisplayToControl::SettingsInformation(period, blur_duration) => {
                Task::done(Message::SettingsReceived(
                    period.to_native(),
                    blur_duration.to_native()
                ))
            }
        }
    )
}
