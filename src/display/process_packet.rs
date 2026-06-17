use iced::Task;
use onedrive_albums::api::drive::get_user;
use onedrive_albums::authentication::oauth2::api::post_oauth2_code;
use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;
use rkyv::option::ArchivedOption;

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

fn own_album(album: &ArchivedAlbum) -> Album {
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
        &photo_id,
        &album.id,
        &album.user_id
    ).await?;

    let photo = photo.ok_or(Error::NoThumbnail)?;
    ReflectionImage::load_thumbnail(photo).await.map(|f| (album, f))
}

pub fn process_packet(recv_packet: RecvPacket) -> Res<Task<Message>> {

    // It is expected that recv_packet contains a ControlToDisplay
    let control_to_display: &Archived<ControlToDisplay> = access::<
        Archived<ControlToDisplay>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

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
        _ => todo!("Implement.")
    };

    Ok(task)

}
