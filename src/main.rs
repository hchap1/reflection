#![allow(clippy::enum_variant_names)]

use rusqlite_async::database::Database;
use error::Res;
use directories::create::Directories;
use authentication::oauth2::hashcodes::generate_csrf;
use authentication::oauth2::hashcodes::generate_pkce;
use authentication::oauth2::api::post_oauth2_code;
use authentication::oauth2::refresh::refresh_tokenset;
use authentication::callback::client::launch_oauth2;
use authentication::callback::server::run_server;

use crate::onedrive::api::AccessToken;
use crate::onedrive::get_drive::get_drives;

mod error;
mod util;
mod database;
mod directories;
mod authentication;

mod onedrive;

#[tokio::main]
async fn main() -> Res<()> {

    println!("Creating directories for application...");
    let directories = Directories::create_or_load()?;
    println!("Created directories. Root: {}", directories.root.to_string_lossy());

    println!("Loading database...");
    let (database, _) = Database::new(directories.root.clone());
    database::interface::create_tables(database.derive())?;
    println!("Database aquired");

    println!("Attempting to aquire refresh token from database...");
    let tokenset = match database::interface::retrieve_token(database.derive()).await {
        Ok((token, _)) => {
            println!("Aquired refresh token from database. Aquiring access token...");
            let tokenset = refresh_tokenset(token).await?;
            println!("Aquired access token: {}", tokenset.access_token);
            tokenset
        },
        Err(e) => {
            println!("Failed to retrieve token from the dataase: {e:?}");
            println!("Generating CSRF and PKCE...");
            let csrf = generate_csrf();
            let (pkce_verifier, pkce_challenge) = generate_pkce();
            println!("Generated CSRF {csrf} and PKCE {pkce_verifier} / {pkce_challenge}");

            println!("Redirecting user to oauth2...");
            launch_oauth2(csrf.clone(), pkce_challenge).await?;
            println!("Redirected user to oauth2");
            
            println!("Running callback server...");
            let temporary_code = run_server(csrf).await?;
            println!("Callback server returned code {temporary_code}");

            println!("Posting temporary_code to verification server.");
            let tokenset = post_oauth2_code(temporary_code, pkce_verifier).await?;
            println!("Received access token: {} and refresh token: {}", tokenset.access_token, tokenset.refresh_token);
            tokenset
        }
    };

    println!("Writing new refresh token into database...");
    database::interface::insert_token(database.derive(), tokenset.refresh_token, tokenset.absolute_expiration).await?;
    println!("New refresh token written.");

    println!("Requesting drive details for user...");
    let drive = get_drives(AccessToken::new(tokenset.access_token)).await?;
    println!("\nRetrieved drive:\n");
    println!("{}, {:?}, {}", drive.id, drive.owner, drive.drive_type);

    println!("ENDING\n\n");

    Ok(())
}
