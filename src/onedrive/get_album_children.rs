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

    #[serde(rename = "mediaAlbum")]
    pub album_metadata: AlbumMetadata
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumMetadata {
    #[serde(rename = "albumItemCount")]
    pub item_count: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumContentsResponse {
    value: Vec<Photo>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Photo {

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
    altitude: Option<f64>,
    latitude: f64,
    longitude: f64
}

#[derive(Clone, Debug)]
pub struct PhotoFile {
    pub id: String,
    pub name: String,
    pub creation_date: Option<SystemTime>,
    pub width: usize,
    pub height: usize,
    pub filesize: usize,
    pub location: Option<LocationData>
}

impl PhotoFile {
    fn from_response(response: Photo) -> PhotoFile {
        PhotoFile {
            id: response.id,
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

pub async fn get_album_children(access_token: AccessToken, drive_id: String, share_link: String) -> Res<(AlbumDriveItem, Vec<PhotoFile>)> {
    let encoded_link = BASE64_URL_SAFE_NO_PAD.encode(share_link);
    let drive_item = make_request::<AlbumDriveItem>(&format!("{READ_SHARE_URL}{encoded_link}/driveItem"), access_token.get().to_string(), vec![]).await?;

    let album_id = drive_item.id.clone();

    Ok((drive_item, make_request::<AlbumContentsResponse>(
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
        .map(PhotoFile::from_response)
        .collect()))
}
