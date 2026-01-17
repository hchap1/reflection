use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL_START: &str = "https://graph.microsoft.com/v1.0/me/drive/items/";
const URL_END: &str = "!0:/SkyDriveCache/Albums:";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Albums {

}

pub async fn get_albums(access_token: AccessToken, user_id: String) -> Res<Albums> {
    make_request::<Albums>(&format!("{URL_START}{user_id}{URL_END}"), access_token.get().to_string(), vec![]).await
}
