use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::error::Res;

const URLBASE: &str = "https://graph.microsoft.com/v1.0";

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

pub async fn make_request<T: DeserializeOwned>(url: &str, access_token: String, parameters: Vec<(String, String)>) -> Res<T> {
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

    // Attempt to deserialize with T.
    let object: T = serde_json::from_str(body.as_str())?;
    Ok(object)
}
