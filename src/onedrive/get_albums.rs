use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL_START: &str = "https://graph.microsoft.com/v1.0/shares/u!";
const URL_END: &str = "/driveItem";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumDriveItem {
    id: String,
    name: String,

    #[serde(rename = "mediaAlbum")]
    album_metadata: AlbumMetadata
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumMetadata {
    #[serde(rename = "albumItemCount")]
    item_count: usize
}

pub async fn get_albums(access_token: AccessToken, share_link: String) -> Res<AlbumDriveItem> {
    let encoded_link = BASE64_URL_SAFE_NO_PAD.encode(share_link);
    let drive_item = make_request::<AlbumDriveItem>(&format!("{URL_START}{encoded_link}{URL_END}"), access_token.get().to_string(), vec![]).await?;
}
