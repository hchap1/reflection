#![allow(clippy::enum_variant_names)]

mod callback;
mod error;
mod util;

fn main() {
    let ret = callback::client::launch_oauth_window();
    println!("{ret:?}");
}
