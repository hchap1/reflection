// CONTROL APPLICATION

use bytes::Bytes;
use lan_tcp::networking::node::Node;
use reflection::{backend::networking::network_message::{ArchivedDisplayToControl, ControlToDisplay, DisplayToControl, ObfuscatedUser}, IDENTIFIER, PORT};
use rkyv::{option::ArchivedOption, rancor, Archived};

#[tokio::main]
async fn main() -> reflection::error::Res<()> {
    let mut connection = Node::spawn_client(IDENTIFIER, PORT)
        .await?;

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

    let first_user = if let ArchivedDisplayToControl::UserInformation(user) = display_to_control {
        user
    } else { panic!("Didn't receive a user packet!") };


    connection.send(
        Bytes::from_owner(rkyv::to_bytes::<rancor::Error>(
            &ControlToDisplay::RequestAlbumsBelongingToUser(ObfuscatedUser {
                id: first_user.id.to_string(),
                name: match &first_user.name {
                    ArchivedOption::Some(name) => Some(name.to_string()),
                    ArchivedOption::None => None
                },
                email: match &first_user.email {
                    ArchivedOption::Some(email) => Some(email.to_string()),
                    ArchivedOption::None => None
                },
                expiry_date_time: first_user.expiry_date_time.into()
            })
        )?),
        lan_tcp::networking::node::Destination::Server
    ).await?;

    let display_to_control: &Archived<DisplayToControl> = rkyv::access::<
        Archived<DisplayToControl>,
        rkyv::rancor::Error
    >(&incoming_packet.data)?;

    println!("RECEIVED PACKET: {display_to_control:?}");


    Ok(())
}
