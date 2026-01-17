use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL: &str = "https://graph.microsoft.com/v1.0/me/drive/items/<user-id>!0:/SkyDriveCache/Albums:";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Albums {

}

pub async fn get_albums(access_token: AccessToken) -> Res<Albums> {
    make_request::<Albums>(URL, access_token.get().to_string(), vec![]).await
}
