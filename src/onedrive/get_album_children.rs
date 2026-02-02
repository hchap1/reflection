use std::time::SystemTime;
use async_channel::unbounded;
use chrono::{DateTime, Utc};

use futures_util::{StreamExt, stream};
use rusqlite_async::database::DataLink;
use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

use crate::database::interface::{insert_album, insert_photo, select_albums};
use crate::onedrive::api::{AccessToken, make_request};
use crate::error::{Error, Res};

const READ_SHARE_URL: &str = "https://graph.microsoft.com/v1.0/shares/u!";
const READ_CONTENTS_URL: &str = "https://graph.microsoft.com/v1.0/drives/";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumDriveItem {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub id: usize,
    pub onedrive_id: String,
    pub name: String,
    pub share_link: String
}

impl Album {
    fn from_response(response: AlbumDriveItem, share_link: String) -> Album {
        Album {
            id: 0,
            onedrive_id: response.id,
            name: response.name,
            share_link
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumMetadata {
    #[serde(rename = "albumItemCount")]
    pub item_count: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumContentsResponse {
    value: Vec<PhotoResponse>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PhotoResponse {

    #[serde(rename = "createdDateTime")]
    creation_date: String,
    id: String,
    name: String,
    size: usize,
    
    #[serde(rename = "image")]
    resolution_data: ResolutionData,
    location: Option<LocationData>
}


#[derive(Serialize, Deserialize, Debug, Clone)]
struct ResolutionData {
    height: usize,
    width: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocationData {
    pub altitude: Option<f64>,
    pub latitude: f64,
    pub longitude: f64
}

#[derive(Clone, Debug)]
pub struct Photo {
    pub id: usize,
    pub onedrive_id: String,
    pub name: String,
    pub creation_date: Option<SystemTime>,
    pub width: usize,
    pub height: usize,
    pub filesize: usize,
    pub location: Option<LocationData>
}

impl Photo {
    fn from_response(response: PhotoResponse) -> Photo {
        Photo {
            id: 0,
            onedrive_id: response.id,
            name: response.name,
            creation_date: {
                match response.creation_date.parse::<DateTime<Utc>>() {
                    Ok(datetime) => Some(SystemTime::from(datetime)),
                    Err(_) => None
                }
            },
            width: response.resolution_data.width,
            height: response.resolution_data.height,
            filesize: response.size,
            location: response.location
        }
    }
}

pub async fn new_album(access_token: AccessToken, drive_id: String, share_link: String, database: DataLink) -> Res<(Album, Vec<Photo>)> {
    let encoded_link = BASE64_URL_SAFE_NO_PAD.encode(&share_link);
    let drive_item = make_request::<AlbumDriveItem>(&format!("{READ_SHARE_URL}{encoded_link}/driveItem"), access_token.get().to_string(), vec![]).await?;
    println!("RECEIVED DRIVE ITEM");
    let album = insert_album(database.clone(), Album::from_response(drive_item, share_link)).await?;
    check_album(access_token, drive_id, album, database).await
}

// TODO PAGINATION

pub async fn check_album(access_token: AccessToken, drive_id: String, album: Album, database: DataLink) -> Res<(Album, Vec<Photo>)> {
    let (error_send, error_recv) = unbounded();
    let stream = stream::iter(make_request::<AlbumContentsResponse>(
        &format!(
            "{READ_CONTENTS_URL}{}/items/{}/children",
            drive_id,
            &album.onedrive_id
        ),
        access_token.get().to_string(),
        vec![]
    )
        .await?
        .value
        .into_iter()
        .map(Photo::from_response))
        .filter_map(async |photo| match insert_photo(database.clone(), photo).await {
            Ok(photo) => Some(photo),
            Err(error) => {
                let _ = error_send.send(error).await;
                None
            }
        })
        .collect()
        .await;

    while let Ok(error) = error_recv.try_recv() {
        eprintln!("SQL Error: {error:?}");
    }

    Ok((
        album,
        stream
    ))
}

// Update all cached albums
pub async fn check_all_albums(access_token: AccessToken, drive_id: String, database: DataLink) -> Res<Vec<(Album, Vec<Photo>)>> {
    Ok(stream::iter(
        select_albums(database.clone())
            .await?
            .into_iter()
    )
        .filter_map(async |album| check_album(access_token.clone(), drive_id.clone(), album.clone(), database.clone()).await.ok())
        .collect()
        .await)
}
