use std::time::SystemTime;
use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

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

pub async fn retrieve_album(access_token: AccessToken, drive_id: String, share_link: String) -> Res<(Album, Vec<Photo>)> {
    let encoded_link = BASE64_URL_SAFE_NO_PAD.encode(&share_link);
    let drive_item = make_request::<AlbumDriveItem>(&format!("{READ_SHARE_URL}{encoded_link}/driveItem"), access_token.get().to_string(), vec![]).await?;

    let album_id = drive_item.id.clone();

    Ok((Album::from_response(drive_item, share_link), make_request::<AlbumContentsResponse>(
        &format!(
            "{READ_CONTENTS_URL}{}/items/{}/children",
            drive_id,
            album_id
        ),
        access_token.get().to_string(),
        vec![]
    )
        .await?
        .value
        .into_iter()
        .map(Photo::from_response)
        .collect()))
}
