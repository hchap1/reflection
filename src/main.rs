#![allow(clippy::enum_variant_names)]

use crate::callback::{client::launch_oauth2, server::{generate_csrf, generate_pkce, run_server}};

mod callback;
mod error;
mod util;
mod oauth2;

#[tokio::main]
async fn main() {
    let csrf = generate_csrf();
    let (pkce_verifier, pkce_challenge) = generate_pkce();

    println!("{:?}", launch_oauth2(csrf.clone(), pkce_challenge).await);
    println!("{:?}", run_server(csrf).await);
}
