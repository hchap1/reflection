// CONTROL APPLICATION

use lan_tcp::networking::node::Node;
use onedrive_albums::authentication::oauth2;
use reflection::{IDENTIFIER, PORT};

#[tokio::main]
async fn main() -> reflection::error::Res<()> {
    let connection = Node::spawn_client(IDENTIFIER, PORT)
        .await?;

    let refresh_token =  oauth2::wrapper::acquire_refresh_token()
        .await?;

    Ok(())
}
