use crate::error::Res;
use open::that;

const URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";

pub const CLIENT_ID: &str = "9309988c-aa51-4b83-a387-b3613cc503c8";
const RESPONSE_TYPE: &str = "code";
pub const REDIRECT_URL: &str = "http://localhost:3000";
const RESPONSE_MODE: &str = "query";
pub const SCOPE: &str = "openid profile offline_access Files.Read";
const CODE_CHALLENGE_METHOD: &str = "S256";

/// Given the csrf state and the pkce challenge, load the OAUTH2 page in users default web browser.
pub async fn launch_oauth2(csrf: String, pkce: String) -> Res<()> {
    that(
        format!(
            "{URL}?client_id={}&response_type={}&redirect_uri={}&response_mode={}&scope={}&state={}&code_challenge={}&code_challenge_method={}",
            CLIENT_ID, RESPONSE_TYPE, REDIRECT_URL, RESPONSE_MODE, SCOPE, csrf, pkce, CODE_CHALLENGE_METHOD
        )
    )?;
    Ok(())
}
