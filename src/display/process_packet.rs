use iced::Task;
use onedrive_albums::api::albums::get_albums;
use onedrive_albums::api::drive::get_user;
use onedrive_albums::authentication::oauth2::api::post_oauth2_code;
use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;
use rkyv::option::ArchivedOption;

use crate::display::application::Application;
use crate::backend::database::authentication_storage::Authentication;
use crate::backend::database::sql::Album;
use crate::backend::database::sql::ArchivedAlbum;
use crate::backend::database::sql::SQL;
use crate::backend::database::sql::User;
use crate::backend::directories::image::ReflectionImage;
use crate::backend::networking::network_message::ArchivedControlToDisplay;
use crate::backend::networking::network_message::ControlToDisplay;
use crate::backend::networking::network_message::DisplayToControl;
use crate::display::application::Message;
use crate::error::Error;
use crate::error::Res;

/// Authenticate a user, add to the database and then return the new user
async fn authenticate(temporary_token: String, pkce_verifier: String) -> Res<User> {
    let token_set = post_oauth2_code(temporary_token, pkce_verifier).await?;
    let user = get_user(token_set.access_token).await?;

    let user = User {
        id: user.id,
        email: user.email,
        name: user.display_name,
        refresh_token: token_set.refresh_token,
        expiry_date_time: token_set.absolute_expiration as i64
    };

    SQL::insert_or_update_user(&user).await?;

    Ok(user)
}

pub fn own_album(album: &ArchivedAlbum) -> Album {
    Album {
        id: album.id.to_string(),
        user_id: album.user_id.to_string(),
        name: album.name.to_string(),
        num_items: match album.num_items {
            ArchivedOption::Some(v) => Some(v.to_native()),
            ArchivedOption::None => None
        },
        cover_image_id: match &album.cover_image_id {
            ArchivedOption::Some(v) => Some(v.to_string()),
            ArchivedOption::None => None
        }
    }
}

async fn load_thumbnail_by_id(album: Album) -> Res<(Album, ReflectionImage)> {

    let photo_id = album.cover_image_id
        .as_ref()
        .ok_or(Error::NoThumbnail)?;

    let photo = SQL::select_photo_by_id(
        photo_id,
        &album.id,
        &album.user_id
    ).await?;

    let photo = photo.ok_or(Error::NoThumbnail)?;
    ReflectionImage::load_thumbnail(photo).await.map(|f| (album, f))
}

/// Get all the albums belonging to the given user from onedrive, do not touch database
async fn get_albums_belonging_to_user(user_id: String) -> Res<Vec<Album>> {
    let user = SQL::select_user_by_id(user_id).await?;
    let mut user = user.ok_or(Error::NoSuchUserInDatabase)?;
    let access_token = Authentication::get_access_token(&mut user).await?;
    Ok(
        get_albums(access_token)
            .await?
            .into_iter()
            .map(|md| Album {
                id: md.id,
                user_id: user.id.clone(),
                name: md.name,
                num_items: Some(md.num_items as i64),
                cover_image_id: md.cover_image_id
            })
            .collect()
    )
}

/// Save this into the database
/// Update the display server and control application
async fn set_active_album(
    album: Option<Album>
) -> Res<()> {

    // If the album exists
    if let Some(album) = album {
        // Ensure that the album is actually on record
        if !SQL::select_all_albums()
            .await?
            .into_iter()
            .any(|db_album| db_album.id == album.id) {
            Err(Error::InvalidAlbum)?
        }

        // Record the new setting
        let album_id = album.id;
        let user_id = album.user_id;
        let combination = format!(
            "{:04}{}{}",
            album_id.len(),
            album_id,
            user_id
        );
        SQL::insert_or_update_setting("active_album", &combination).await?;
    } else {

        // Delete the current album setting
        SQL::delete_setting_by_name("active_album").await?;
    }

    Ok(())
}

pub fn process_packet(
    application: &mut Application,
    recv_packet: RecvPacket
) -> Res<Task<Message>> {

    println!("PROCESS PACKET CALLED!");
    // It is expected that recv_packet contains a ControlToDisplay
    let control_to_display: &Archived<ControlToDisplay> = access::<
        Archived<ControlToDisplay>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

    println!("PACKET: {control_to_display:?}");

    // Process the command
    let task = match control_to_display {

        ArchivedControlToDisplay::RequestUsers => Task::perform(
            SQL::select_all_users(),
            |res| match res {
                Ok(users) => Message::Batch(
                    users.into_iter()
                        .map(|user| Message::Send(
                            DisplayToControl::UserInformation(user.into())
                        ))
                        .collect()
                ),
                Err(e) => Message::Error(e)
            }
        ),

        ArchivedControlToDisplay::RequestAlbums => Task::perform(
            SQL::select_all_albums(),
            |res| match res {
                Ok(albums) => Message::Batch(
                    albums.into_iter()
                        .map(|album| Message::Send(
                            DisplayToControl::AlbumInformation(album)
                        ))
                        .collect()
                ),
                Err(e) => Message::Error(e)
            }
        ),

        ArchivedControlToDisplay::RequestAlbumCover(album) => Task::perform(
            load_thumbnail_by_id(own_album(album)),
            |res| match res {
                Ok((album, image)) => Message::Send(
                    DisplayToControl::ReturnAlbumCover(album, image)
                ),
                Err(e) => Message::Error(e)
            }
        ),

        // Authenticate with the temporary token
        ArchivedControlToDisplay::Authenticated(temporary_token, pkce_verifier) => Task::perform(
            authenticate(
                temporary_token.to_string(),
                pkce_verifier.to_string()
            ),
            |res| match res {
                Ok(user) => Message::Send(
                    DisplayToControl::UserInformation(user.into())
                ),
                Err(e) => Message::Error(e)
            }
        ),

        ArchivedControlToDisplay::RequestAlbumsBelongingToUser(obfuscated_user) => Task::perform(
            get_albums_belonging_to_user(obfuscated_user.id.to_string()),
            |res| match res {
                Ok(albums) => Message::Batch(
                    albums
                        .into_iter()
                        .map(|album| Message::Send(
                            DisplayToControl::ReturnAlbumsBelongingToUser(album)
                        ))
                        .collect()
                ),
                Err(e) => Message::Error(e)
            }
        ),

        ArchivedControlToDisplay::AddAlbum(album) => {
            let album = own_album(album);
            Task::perform(
                SQL::insert_or_update_album(album.clone()),
                |res| match res {
                    Ok(_) => Message::Batch(vec![
                        Message::Send(
                            DisplayToControl::AlbumInformation(album)
                        ),
                        Message::SynchronisePhotos
                    ]),
                    Err(e) => Message::Error(e)
                }
            )
        },

        ArchivedControlToDisplay::RequestPhotosInAlbum(album) => {
            let album = own_album(album);
            Task::perform(
                SQL::select_photos_by_album(album.id, album.user_id),
                |res| match res {
                    Ok(photos) => Message::Batch(
                        photos
                            .into_iter()
                            .map(|photo| Message::Batch(
                                vec![
                                    Message::Send(
                                        DisplayToControl::ReturnPhoto(photo.clone())
                                    ),
                                    Message::SendThumbnail(
                                        photo
                                    )
                                ]
                            ))
                            .collect()
                    ),
                    Err(e) => Message::Error(e)
                }
            )
        }

        ArchivedControlToDisplay::RequestActive => {
            Task::done(
                Message::Send(
                    DisplayToControl::SelectedAlbum(
                        application.active_album.clone()
                    )
                )
            )
        }

        // Set the currently active album, return through
        // networking and also save in the database
        ArchivedControlToDisplay::SetActiveAlbum(album) => {
            let album = match album {
                ArchivedOption::Some(album) => Some(own_album(album)),
                ArchivedOption::None => None
            };
            Task::perform(
                set_active_album(album.clone()),
                |res| match res {
                    Ok(()) => Message::Batch(vec![
                        Message::Send(
                            DisplayToControl::SelectedAlbum(
                                album.clone()
                            )
                        ),
                        Message::AlbumChange(album)
                    ]),
                    Err(e) => Message::Error(e)
                }
            )
        }
    };

    Ok(task)

}
