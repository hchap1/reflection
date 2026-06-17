use iced::Task;
use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;
use rkyv::option::ArchivedOption;

use crate::backend::database::sql::Album;
use crate::backend::database::sql::ArchivedAlbum;
use crate::backend::database::sql::SQL;
use crate::backend::directories::image::ReflectionImage;
use crate::backend::networking::network_message::ArchivedControlToDisplay;
use crate::backend::networking::network_message::ControlToDisplay;
use crate::backend::networking::network_message::DisplayToControl;
use crate::display::application::Message;
use crate::error::Error;
use crate::error::Res;

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

        _ => todo!("Implement.")
    };

    Ok(task)

}
