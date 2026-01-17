use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL: &str = "https://graph.microsoft.com/v1.0/me/drive";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DriveData {

    #[serde(rename = "@odata.context")]
    odata_context: String,
    #[serde(rename = "createdDateTime")]
    creation_date: String,
    #[serde(rename = "description")]
    description: Option<String>,

    pub id: String,

    #[serde(rename = "driveType")]
    pub drive_type: String,
    pub owner: Owner
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Owner {
    pub user: User
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub email: Option<String>
}

pub async fn get_drive(access_token: AccessToken) -> Res<DriveData> {
    make_request::<DriveData>(URL, access_token.get().to_string(), vec![]).await
}
