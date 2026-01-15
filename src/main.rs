#![allow(clippy::enum_variant_names)]

use crate::{callback::{client::launch_oauth2, server::{generate_csrf, generate_pkce, run_server}}, oauth2::api};

mod callback;
mod error;
mod util;
mod oauth2;
mod database;
mod directories;

#[tokio::main]
async fn main() {

    let csrf = generate_csrf();
    let (pkce_verifier, pkce_challenge) = generate_pkce();

    println!("{:?}", launch_oauth2(csrf.clone(), pkce_challenge).await);
    let temporary_code = run_server(csrf).await.unwrap();

    let tokenset = api::post_oauth2_code(temporary_code, pkce_verifier).await.unwrap();
    println!("TOKEN: {} REFRESH: {}", tokenset.access_token, tokenset.refresh_token);
}
