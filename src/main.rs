#![allow(clippy::enum_variant_names)]

use crate::callback::server::generate_csrf;

mod callback;
mod error;
mod util;

#[tokio::main]
async fn main() {
    let csrf = generate_csrf();
    println!("{csrf}");
    let ret = callback::server::run_server(csrf).await;
    println!("{ret:?}");
}
