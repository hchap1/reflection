use reqwest::{Client, Url};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::Res;

const URLBASE: &str = "https://graph.microsoft.com/v1.0";
const GET_DRIVES: &str = "/me/drives";

#[derive(Clone, Debug)]
pub enum OnedriveError {
    BadStatus
}

#[derive(Clone, Debug)]
pub struct AccessToken {
    token: String
}

impl AccessToken {
    pub fn new(access_token: String) -> AccessToken {
        Self { token: access_token }
    }

    pub fn get(&self) -> &str {
        self.token.as_str()
    }
}

pub async fn make_request<'a, T: DeserializeOwned>(url: String, access_token: String, parameters: Vec<(String, String)>) -> Res<T> {
    let client = Client::new();
    let res = client.get(url)
        .bearer_auth(access_token.as_str())
        .form(&parameters)
        .send()
        .await?;

    // If the status indicates failure, don't bother with serde.
    if !res.status().is_success() {
        println!("{}", res.text().await?);
        return Err(OnedriveError::BadStatus.into());
    }

    // Extract body of response.
    let body = res.text().await?;
    println!("BODY: {body}");

    // Attempt to deserialize with T.
    let object: T = serde_json::from_str(body.as_str())?;
    Ok(object)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDriveResponse {
    #[serde(rename = "@odata.context")]
    odata_context: String
}

pub async fn get_drives(access_token: AccessToken) -> Res<GetDriveResponse> {
    make_request::<GetDriveResponse>(format!("{URLBASE}{GET_DRIVES}"), access_token.get().to_string(), vec![]).await
}
