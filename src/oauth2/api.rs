use reqwest::Client;
use serde::{Serialize, Deserialize};

use crate::{callback::client::{CLIENT_ID, REDIRECT_URL, SCOPE}, error::Res};

const URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const GRANT_TYPE: &str = "authorization_code";

#[derive(Clone, Debug)]
pub enum OAUTH2ApiError {
    POSTFailed
}

#[derive(Clone, Debug)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: String
}

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub access_token: String,
    expires_in: usize,
    pub refresh_token: String,
    scope: String,
    token_type: String
}

/// Take the temporary auth code and PKCE verifier string to produce a permanent tokenset.
pub async fn post_oauth2_code(code: String, verifier: String) -> Res<TokenSet> {
    let params = [
        ("client_id", CLIENT_ID),
        ("grant_type", GRANT_TYPE),

        // The temporary access token as provided by the OAUTH2 callback to localhost:3000.
        ("code", &code),

        // Same as the GET request from the browser, Microsoft uses this as an extra layer of security.
        ("redirect_uri", REDIRECT_URL),

        // The verifier is the raw PKCE string that was hashed when the user was redirected to the OAUTH2 page in the browser.
        // This code allows the request to be validated without the client secret.
        ("code_verifier", &verifier),
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

    // Retrieve important part of the tokenset.
    Ok(TokenSet {
        access_token: response.access_token,
        refresh_token: response.refresh_token
    })
}
