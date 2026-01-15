use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;

use crate::{authentication::callback::client::{CLIENT_ID, REDIRECT_URL, SCOPE}, error::Res, authentication::oauth2::api::{OAUTH2ApiError, Response, TokenSet}};

const URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const GRANT_TYPE: &str = "refresh_token";

/// Use a refresh token to retrieve a new access token and a new refresh token
pub async fn refresh_tokenset(refresh_token: String) -> Res<TokenSet> {
    let params = [
        ("client_id", CLIENT_ID),
        ("grant_type", GRANT_TYPE),

        // The long-lived refresh token as provided by either this function or the original authentication.
        ("refresh_token", &refresh_token),

        // Same as the GET request from the browser, Microsoft uses this as an extra layer of security.
        ("redirect_uri", REDIRECT_URL),
        ("scope", SCOPE)
    ];

    let client = Client::new();
    let res = client.post(URL)
        .form(&params)
        .send()
        .await?;

    // If the POST request failed, avoid serde as it will panic.
    if !res.status().is_success() {
        return Err(OAUTH2ApiError::POSTFailed.into());
    }

    // Attempt to retrieve body of the response and parse with serde.
    let text = res.text().await?;
    let response: Response = serde_json::from_str(&text)?;

    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        as usize
        + response.expires_in;

    // Retrieve important part of the tokenset.
    Ok(TokenSet {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        absolute_expiration: expiration
    })
}
