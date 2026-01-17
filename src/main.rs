#![allow(clippy::enum_variant_names)]

use error::Res;
use rusqlite_async::database::Database;
use directories::create::Directories;
use authentication::oauth2::hashcodes::generate_csrf;
use authentication::oauth2::hashcodes::generate_pkce;
use authentication::oauth2::api::post_oauth2_code;
use authentication::oauth2::refresh::refresh_tokenset;
use authentication::callback::client::launch_oauth2;
use authentication::callback::server::run_server;

use crate::onedrive::api::AccessToken;
use crate::onedrive::get_albums::get_albums;
use crate::onedrive::get_drive::get_drive;

mod error;
mod util;
mod database;
mod directories;
mod authentication;

mod onedrive;

#[tokio::main]
async fn main() -> Res<()> {

    let directories = Directories::create_or_load()?;
    let (database, _) = Database::new(directories.root.clone());
    database::interface::create_tables(database.derive())?;

    let tokenset = match database::interface::retrieve_token(database.derive()).await {
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

    database::interface::insert_token(database.derive(), tokenset.refresh_token, tokenset.absolute_expiration).await?;

    let drive = get_drive(AccessToken::new(tokenset.access_token.clone())).await?;

    println!("Retrieved drive:");
    println!("{}, {:?}, {}", drive.id, drive.owner, drive.drive_type);

    let albums = get_albums(AccessToken::new(tokenset.access_token)).await?;

    Ok(())
}
