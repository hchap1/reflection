use crate::{database::sql::{self, SELECT_ALBUM_BY_ID}, error::Res, frontend::application::ApplicationError, onedrive::get_album_children::{Album, LocationData, Photo}};
use chrono::{DateTime, Utc};
use rusqlite_async::database::{DataLink, DatabaseParam, DatabaseParams};

#[derive(Clone, Debug)]
pub enum DatabaseInterfaceError {
    IncorrectNumberOfRows,
    MalformedRow,
}

/// Create the tables without checking for success. If this fails, later DB calls will indicate.
pub fn create_tables(database: DataLink) -> Res<()> {
    database.execute(sql::CREATE_TOKEN_TABLE, DatabaseParams::empty())?;
    database.execute(sql::CREATE_ALBUM_TABLE, DatabaseParams::empty())?;
    database.execute(sql::CREATE_PHOTO_TABLE, DatabaseParams::empty())?;
    database.execute(sql::CREATE_ENTRY_TABLE, DatabaseParams::empty())?;
    Ok(())
}

/// Insert the latest token into the database along with expieration seconds.
pub async fn insert_token(database: DataLink, refresh_token: String, expiration: usize) -> Res<()> {
    database.insert(sql::INSERT_TOKEN, DatabaseParams::new(vec![
        DatabaseParam::Usize(expiration),
        DatabaseParam::String(refresh_token)
    ])).await?;
    Ok(())
}

/// Retrive the latest token
pub async fn retrieve_token(database: DataLink) -> Res<(String, usize)> {
    let rows = database.query_map(sql::SELECT_TOKEN, DatabaseParams::empty()).await?;
    if rows.len() != 1 { return Err(DatabaseInterfaceError::IncorrectNumberOfRows.into()); }
    let row = rows.first().ok_or(DatabaseInterfaceError::IncorrectNumberOfRows)?;
    let token = row.first().ok_or(DatabaseInterfaceError::MalformedRow)?.string();
    let expiration = row.get(1).ok_or(DatabaseInterfaceError::MalformedRow)?.usize();
    Ok((token, expiration))
}

/// Use the onedrive_id to select an album
pub async fn select_album_by_id(database: DataLink, onedrive_id: String) -> Res<Option<Album>> {
    Ok(
        database.query_map(sql::SELECT_ALBUM_BY_ID, DatabaseParams::single(DatabaseParam::String(onedrive_id)))
            .await?
            .into_iter()
            .filter_map(parse_row_into_album)
            .next()
    )
}

/// Use the onedrive_id to select a photo
pub async fn select_photo_by_id(database: DataLink, onedrive_id: String) -> Res<Option<Photo>> {
    Ok(
        database.query_map(sql::SELECT_PHOTO_BY_ID, DatabaseParams::single(DatabaseParam::String(onedrive_id)))
            .await?
            .into_iter()
            .filter_map(parse_row_into_photo)
            .next()
    )
}

/// Insert a new album record, and returning the completed album object
pub async fn insert_album(database: DataLink, mut album: Album) -> Res<Album> {

    // Check if the album exists in the database yet (based on unique onedrive_id)
    match select_album_by_id(database.clone(), album.onedrive_id.clone()).await? {
        Some(existing_album) => Ok(existing_album),
        None => {
            let (row_id, _) = database.insert(sql::INSERT_ALBUM, DatabaseParams::new(vec![
                DatabaseParam::String(album.onedrive_id.clone()),
                DatabaseParam::String(album.name.clone()),
                DatabaseParam::String(album.share_link.clone())
            ])).await?;

            album.id = row_id;
            Ok(album)
        }
    }

}

/// Insert a new photo record
pub async fn insert_photo(database: DataLink, mut photo: Photo) -> Res<Photo> {

    // Check if the photo already exists
    match select_photo_by_id(database.clone(), photo.onedrive_id.clone()).await? {
        Some(updated_photo) => Ok(updated_photo),
        None => {
            let time_string = match photo.creation_date {
                Some(date) => {
                    let datetime: DateTime<Utc> = date.into();
                    datetime.to_rfc3339()
                },
                None => String::from("NONE")
            };

            let (latitude, longitude, altitude) = match photo.location.as_ref() {
                Some(location) => (location.latitude, location.longitude, location.altitude.unwrap_or(0f64)),
                None => (0f64, 0f64, 0f64)
            };

            let (row_id, _) = database.insert(sql::INSERT_PHOTO, DatabaseParams::new(vec![
                DatabaseParam::String(photo.onedrive_id.clone()),
                DatabaseParam::String(photo.name.clone()),
                DatabaseParam::String(time_string),
                DatabaseParam::Usize(photo.width),
                DatabaseParam::Usize(photo.height),
                DatabaseParam::Usize(photo.filesize),
                DatabaseParam::F64(latitude),
                DatabaseParam::F64(longitude),
                DatabaseParam::F64(altitude)
            ])).await?;

            photo.id = row_id;
            Ok(photo)
        }
    }
}

/// Insert an entry tagging a photo as part of an album
pub async fn insert_entry(database: DataLink, album_id: usize, photo_id: usize) -> Res<usize> {
    let (row_id, _) = database.insert(sql::INSERT_ALBUM, DatabaseParams::new(vec![
        DatabaseParam::Usize(album_id),
        DatabaseParam::Usize(photo_id)
    ])).await?;

    Ok(row_id)
}

pub fn parse_row_into_photo(row: Vec<DatabaseParam>) -> Option<Photo> {
    let mut iterator = row.into_iter();
    let id = iterator.next()?.usize();
    let onedrive_id = iterator.next()?.string();
    let name = iterator.next()?.string();
    let time_string = iterator.next()?.string();
    let width = iterator.next()?.usize();
    let height = iterator.next()?.usize();
    let filesize = iterator.next()?.usize();
    let raw_latitude = iterator.next()?.f64();
    let raw_longitude = iterator.next()?.f64();
    let raw_altitude = iterator.next()?.f64();

    let creation_date = match time_string.as_str() {
        "NONE" => None,
        encoded_time => {
            Some(DateTime::parse_from_rfc3339(encoded_time).ok()?.with_timezone(&Utc).into())
        }
    };

    let location = if raw_latitude == 0f64 && raw_longitude == 0f64 {
        None
    } else {
        Some(
            LocationData {
                latitude: raw_latitude,
                longitude: raw_longitude,
                altitude: match raw_altitude {
                    0f64 => None,
                    nonzero => Some(nonzero)
                }
            }
        )
    };

    Some(
        Photo {
            id,
            onedrive_id,
            name,
            creation_date,
            width,
            height,
            filesize,
            location
        }
    )
}

pub fn parse_row_into_album(row: Vec<DatabaseParam>) -> Option<Album> {
    let mut iterator = row.into_iter();
    let id = iterator.next()?.usize();
    let onedrive_id = iterator.next()?.string();
    let name = iterator.next()?.string();
    let share_link = iterator.next()?.string();

    Some(Album {
        id, onedrive_id, name, share_link
    })
}

/// Selects photos in album
pub async fn select_photos_in_album(database: DataLink, album_id: usize) -> Res<(Album, Vec<Photo>)> {

    let album = database.query_map(SELECT_ALBUM_BY_ID, DatabaseParams::single(DatabaseParam::Usize(album_id)))
        .await?
        .into_iter()
        .filter_map(parse_row_into_album)
        .next()
        .ok_or(ApplicationError::NoSuchAlbum)?;


    Ok((album, database.query_map(sql::SELECT_PHOTOS_BY_ALBUM_ID, DatabaseParams::single(DatabaseParam::Usize(album_id)))
        .await?
        .into_iter()
        .filter_map(parse_row_into_photo)
        .collect()))
}

/// Select all photos
pub async fn select_all_photos(database: DataLink) -> Res<Vec<Photo>> {
    Ok(database.query_map(sql::SELECT_ALL_PHOTOS, DatabaseParams::empty())
        .await?
        .into_iter()
        .filter_map(parse_row_into_photo)
        .collect())
}

/// Select all albums
pub async fn select_albums(database: DataLink) -> Res<Vec<Album>> {
    Ok(database.query_map(sql::SELECT_ALL_ALBUMS, DatabaseParams::empty())
        .await?
        .into_iter()
        .filter_map(parse_row_into_album)
        .collect())
}
