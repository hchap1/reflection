use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;

use crate::backend::database::sql::Album;
use crate::backend::database::sql::User;
use crate::backend::database::sql::Photo;
use crate::backend::directories::image::ReflectionImage;

/// User, without a token
#[derive(Clone, Debug, Archive, Serialize, Deserialize)]
pub struct ObfuscatedUser {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub expiry_date_time: i64
}

impl From<User> for ObfuscatedUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            expiry_date_time: user.expiry_date_time
        }
    }
}

// TODO SECURITY

// Communication channel between display / control application
#[derive(Clone, Debug, Archive, Serialize, Deserialize)]
pub enum ControlToDisplay {
    
    // Request state
    // The control application has just connected
    // Therefore, it needs to know the state of the display application
    // This includes:
    // - User, AuthState
    // - Album, Owner
    // - Selected Album
    RequestUsers,

    // When request albums is called, every time an album is received a request
    // to get the cover of the album should be sent
    RequestAlbums,
    RequestActive,

    // Authentication: TODO Security
    // It is IMPERATIVE that the display application immediately use this token
    // This is because it has a very limited lifetime
    // TOKEN, PKCE_VERIFIER
    Authenticated(String, String),

    // Request the cover image of an album
    // If this doesn't exist, give a random one (or None)
    // This expects a single
    // - Photo, Option<Thumbnail>
    RequestAlbumCover(Album),

    // Request all photos in an album
    // This expects a stream of both
    // - Photo
    // - Photo, Thumbnail
    // If no thumbnail exists, never send the second packet
    RequestPhotosInAlbum(Album),

    // Tell the display application to pull photos from a specified album
    // When the display application is changed, the display application echoes back
    SetActiveAlbum(Option<Album>),

    // Request the albums owned by a User, so that the control application
    // Can select to add an album to the database
    RequestAlbumsBelongingToUser(ObfuscatedUser),
    AddAlbum(Album),
}

#[derive(Clone, Debug, Archive, Serialize, Deserialize)]
pub enum DisplayToControl {
    
    // Responses to 'RequestState'

    // UserInformation
    // The state of the authentication can be derived from the expiry
    // This serves to give the control application a basis to work off
    // The control application may choose to reauthenticate a 'dead' user
    // The control application may remove a user or add a new one
    // The control application may also manage albums per user.
    // This is sent whenever a new user is acquired
    UserInformation(ObfuscatedUser),

    // AlbumInformation
    // Provide the state of a current album, including the corresponding user id
    // It is expected that the user for each album have been transmitted previously
    // This is sent whenever a new album is added, OR a state is requested
    AlbumInformation(Album),

    // SelectedAlbum
    // Provide the state of the currently selected album
    // This is sent whenever this changes, OR a state is requested
    SelectedAlbum(Option<Album>),

    // Response to RequestAlbumCover
    // This includes the thumbnail bytes
    ReturnAlbumCover(Album, ReflectionImage),

    // Response to RequestPhotosInAlbum
    ReturnPhoto(Photo),
    ReturnPhotoWithThumbnail(Photo, ReflectionImage),

    // When the active photo changes, send the change to the control application
    ActivePhoto(Photo, ReflectionImage),

    // Return the albums owned by a user
    ReturnAlbumsBelongingToUser(Album),
}
