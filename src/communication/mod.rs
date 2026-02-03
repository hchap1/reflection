use rkyv::{Deserialize, Serialize, Archive};

use crate::{authentication::oauth2::api::TokenSet, error::Res, onedrive::get_album_children::{Album, Photo}};

pub mod server;
pub mod client;

#[derive(Serialize, Deserialize, Archive)]
pub enum NetworkMessage {

    // Client to server
    TokenSet(TokenSet),
    Sharelink(String),
    PlayAlbum(Album),

    RequestAllAlbums,
    RequestPhotosInAlbum(Album),
    RequestThumbnails(Album),

    // Server to client
    ReturnAllAlbums(Vec<Album>),
    ReturnPhotosInAlbum(Vec<Photo>),
    Thumbnail(Vec<u8>),
}

impl NetworkMessage {
    // pub fn to_bytes(&self) -> Res<Vec<u8>> {

    // }
}
