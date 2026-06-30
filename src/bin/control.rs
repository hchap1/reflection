// CONTROL APPLICATION

use bytes::Bytes;
use lan_tcp::networking::node::{Destination, Node, RecvPacket};
use reflection::{
    backend::networking::network_message::{ArchivedDisplayToControl, ControlToDisplay, DisplayToControl},
    display::process_packet::own_album,
    error::Res,
    IDENTIFIER, PORT,
};
use rkyv::{rancor, Archived};
use tokio::sync::mpsc::Receiver;

async fn send(connection: &Node, msg: &ControlToDisplay) -> Res<()> {
    connection.send(
        Bytes::from_owner(rkyv::to_bytes::<rancor::Error>(msg)?),
        Destination::Server,
    ).await?;
    Ok(())
}

async fn recv(receiver: &mut Receiver<RecvPacket>) -> Bytes {
    receiver.recv().await.unwrap().data
}

fn decode(data: &[u8]) -> Res<&Archived<DisplayToControl>> {
    Ok(rkyv::access::<Archived<DisplayToControl>, rkyv::rancor::Error>(data)?)
}

#[tokio::main]
async fn main() -> Res<()> {
    let mut connection = Node::spawn_client(IDENTIFIER, PORT).await?;
    let mut receiver = connection.take_receiver().unwrap();

    // Request all albums currently in the database
    send(&connection, &ControlToDisplay::RequestAlbums).await?;
    println!("Sent RequestAlbums.");

    let bytes = recv(&mut receiver).await;
    let packet = decode(&bytes)?;
    println!("Received: {packet:?}");

    let album = if let ArchivedDisplayToControl::AlbumInformation(album) = packet {
        own_album(album)
    } else {
        panic!("Expected AlbumInformation, got: {packet:?}");
    };

    println!("First album in database: {:?}", album.name);

    // Set it as the active album
    send(&connection, &ControlToDisplay::SetActiveAlbum(Some(album))).await?;
    println!("Sent SetActiveAlbum.");

    let bytes = recv(&mut receiver).await;
    let packet = decode(&bytes)?;
    println!("Confirmed active album: {packet:?}");

    Ok(())
}
