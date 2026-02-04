use rkyv::{Archive, Deserialize, Serialize};

use crate::{authentication::oauth2::api::TokenSet, error::Res, onedrive::get_album_children::{Album, Photo}};

pub mod server;
pub mod client;

#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub enum NetworkMessage {

    // Client to server
    TokenSet(TokenSet),
    Sharelink(String),
    PlayAlbum(Album),

    RequestAllAlbums,
    RequestPhotosInAlbum(Album),
    RequestThumbnails,
    RequestActiveAlbum,

    // Server to client
    NewAlbum(Album),
    ReturnAllAlbums(Vec<Album>),
    ReturnPhotosInAlbum(Vec<Photo>),
    Thumbnail(Album, Vec<u8>),
    ReturnActiveAlbum(Option<Album>),

    // Dummy
    TerminateThread
}

impl NetworkMessage {
    pub fn to_bytes(&self) -> Res<Vec<u8>> {
        Ok(rkyv::to_bytes::<rkyv::rancor::Error>(self)?.into_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> Res<Self> {
        Ok(rkyv::from_bytes::<Self, rkyv::rancor::Error>(bytes)?)
    }
}
