#![allow(clippy::enum_variant_names)]

mod callback;
mod error;
mod util;

#[tokio::main]
async fn main() {
    let ret = callback::client::launch_oauth2();
    println!("{ret:?}");
}
