// CONTROL APPLICATION

use bytes::Bytes;
use lan_tcp::networking::node::Node;
use onedrive_albums::authentication::oauth2;
use reflection::{backend::networking::network_message::{ControlToDisplay, DisplayToControl}, IDENTIFIER, PORT};
use rkyv::{rancor, Archived};

#[tokio::main]
async fn main() -> reflection::error::Res<()> {
    let mut connection = Node::spawn_client(IDENTIFIER, PORT)
        .await?;

    let (raw_auth_code, pkce_verifier) = oauth2::wrapper::acquire_auth_code()
        .await?;

    println!("Grabbed code: {raw_auth_code}, {pkce_verifier}");

    connection.send(
        Bytes::from_owner(rkyv::to_bytes::<rancor::Error>(
            &ControlToDisplay::Authenticated(raw_auth_code, pkce_verifier)
        )?),
        lan_tcp::networking::node::Destination::Server
    ).await?;

    println!("Send codes.");

    connection.send(
        Bytes::from_owner(rkyv::to_bytes::<rancor::Error>(
            &ControlToDisplay::RequestUsers
        )?),
        lan_tcp::networking::node::Destination::Server
    ).await?;
    
    println!("Send request for users.");

    let mut receiver = connection.take_receiver().unwrap();
    let incoming_packet = receiver.recv().await.unwrap();

    println!("Got packet.");

    let display_to_control: &Archived<DisplayToControl> = rkyv::access::<
        Archived<DisplayToControl>,
        rkyv::rancor::Error
    >(&incoming_packet.data)?;

    println!("RECEIVED PACKET: {display_to_control:?}");

    Ok(())
}
