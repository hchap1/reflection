use serde::{Deserialize, Serialize};

use crate::onedrive::api::{AccessToken, make_request};
use crate::error::Res;

const URL: &str = "https://graph.microsoft.com/v1.0/me";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    #[serde(rename = "givenName")]
    pub firstname: String,
    #[serde(rename = "surname")]
    pub lastname: String,
    #[serde(rename = "mail")]
    pub email: String,
    pub id: String
}

pub async fn get_me(access_token: AccessToken) -> Res<User> {
    make_request::<User>(URL, access_token.get().to_string(), vec![]).await
}
