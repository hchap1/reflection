use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL: &str = "https://graph.microsoft.com/v1.0/me/drives";

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDriveResponse {
    #[serde(rename = "@odata.context")]
    odata_context: String,
    value: Vec<DriveInstance>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DriveInstance {
    pub id: String,
    pub name: String,
    #[serde(rename = "webUrl")]
    pub url: String,
}

pub async fn get_drives(access_token: AccessToken) -> Res<Vec<DriveInstance>> {
    Ok(make_request::<GetDriveResponse>(URL, access_token.get().to_string(), vec![])
        .await?
        .value
        .into_iter()
        .filter(|drive| drive.name == "OneDrive")
        .collect())
}
