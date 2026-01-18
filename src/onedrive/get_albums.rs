use serde::{Deserialize, Serialize};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const READ_SHARE_URL: &str = "https://graph.microsoft.com/v1.0/shares/u!";
const READ_CONTENTS_URL: &str = "https://graph.microsoft.com/v1.0/drives/";

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumContents {

}

pub async fn get_albums(access_token: AccessToken, drive_id: String, share_link: String) -> Res<AlbumContents> {
    let encoded_link = BASE64_URL_SAFE_NO_PAD.encode(share_link);
    let drive_item = make_request::<AlbumDriveItem>(&format!("{READ_SHARE_URL}{encoded_link}/driveItem"), access_token.get().to_string(), vec![]).await?;

    make_request::<AlbumContents>(
        &format!(
            "{READ_CONTENTS_URL}{}/items/{}/children",
            drive_id,
            drive_item.id
        ),
        access_token.get().to_string(),
        vec![]
    ).await
}
