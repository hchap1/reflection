use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL: &str = "https://graph.microsoft.com/v1.0/me/drive";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriveData {
    pub id: String,
    #[serde(rename = "driveType")]
    pub drive_type: String,
    pub owner: User
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String
}

pub async fn get_drives(access_token: AccessToken) -> Res<DriveData> {
    make_request::<DriveData>(URL, access_token.get().to_string(), vec![]).await
}
