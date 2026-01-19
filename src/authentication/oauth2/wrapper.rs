use rusqlite_async::database::DataLink;
use crate::authentication::oauth2::api::TokenSet;
use crate::authentication::oauth2::refresh::refresh_tokenset;
use crate::authentication::oauth2::hashcodes::generate_csrf;
use crate::authentication::oauth2::hashcodes::generate_pkce;
use crate::authentication::oauth2::api::post_oauth2_code;
use crate::authentication::callback::server::run_server;
use crate::authentication::callback::client::launch_oauth2;
use crate::database::interface::insert_token;
use crate::database::interface::retrieve_token;
use crate::onedrive::get_drive::DriveData;
use crate::onedrive::api::AccessToken;
use crate::onedrive::get_drive::get_drive;
use crate::error::Res;

pub async fn authenticate(datalink: DataLink) -> Res<(TokenSet, DriveData)> {
    let tokenset = match retrieve_token(datalink.clone()).await {
        Ok((token, _)) => {
            refresh_tokenset(token).await?
        },
        Err(_) => {
            let csrf = generate_csrf();
            let (pkce_verifier, pkce_challenge) = generate_pkce();

            launch_oauth2(csrf.clone(), pkce_challenge).await?;
            let temporary_code = run_server(csrf).await?;
            post_oauth2_code(temporary_code, pkce_verifier).await?
        }
    };

    insert_token(datalink, tokenset.refresh_token.clone(), tokenset.absolute_expiration).await?;
    let drive = get_drive(AccessToken::new(tokenset.access_token.clone())).await?;

    Ok((tokenset, drive))
}
