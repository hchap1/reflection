use reqwest::Client;
use serde::{Serialize, Deserialize};

use crate::{callback::client::{CLIENT_ID, SCOPE}, error::Res};

const URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const GRANT_TYPE: &str = "authorization_code";
const REDIRECT_URI: &str = "?";

#[derive(Clone, Debug)]
pub struct TokenSet {
    access_token: String,
    refresh_token: String
}

impl TokenSet {
    pub fn get_access_token(&self) -> String {
        self.access_token.clone()
    }

    pub fn get_refresh_token(&self) -> String {
        self.refresh_token.clone()
    }
}

#[derive(Serialize, Deserialize)]
struct Response {
    access_token: String,
    expires_in: usize,
    refresh_token: String,
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
        ("redirect_uri", REDIRECT_URI),

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

    let text = res.text().await?;
    let response: Response = serde_json::from_str(&text)?;

    Ok(TokenSet {
        access_token: response.access_token,
        refresh_token: response.refresh_token
    })
}
