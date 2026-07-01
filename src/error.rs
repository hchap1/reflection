use std::path::PathBuf;
use tokio::{sync::AcquireError, task::JoinError};

pub type Res<T> = Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {

    #[error("Failed to find storage location")]
    FailedToFindStorageLocation,

    #[error("Could not check existance of root")]
    CheckFileExistsError,

    #[error("Could not create directory {:?}", .0)]
    FailedToCreateDirectory(PathBuf),

    #[error("Failed to access storage (singleton)")]
    FailedToAccessStorage,

    #[error("DatabaseError: SQLX {:?}", .0)]
    DatabaseError(std::sync::Arc<sqlx::Error>),

    #[error("Failed to access database")]
    FailedToAccessDatabase,

    #[error("Image error: {:?}", .0)]
    ImageError(std::sync::Arc<image::error::ImageError>),

    #[error("Networking error: {:?}", .0)]
    NetworkError(lan_tcp::error::Error),

    #[error("Tcp receiver missing. Could not be taken")]
    TcpReceiverMissing,

    #[error("(De)serialisation error: {:?}", .0)]
    RancorError(std::sync::Arc<rkyv::rancor::Error>),

    #[error("Cannot mutate Node arc, thus could not take receiver")]
    CouldNotMutateNodeArc,

    #[error("Node does not exist, cannot perform networking operation")]
    MissingNode,

    #[error("The photo doesn't have a corresponding thumbnail file")]
    NoThumbnail,

    #[error("The photo doesn't have a corresponding file")]
    NoFile,

    #[error("OneDrive API error: {:?}", .0)]
    OneDriveError(onedrive_albums::error::Error),

    #[error("Control app specified user that doesn't exist in DB")]
    NoSuchUserInDatabase,

    #[error("Couldn't set singleton, value already exists")]
    SingletonSetError,

    #[error("The authentication singleton doesn't exist")]
    AuthenticationSingletonDead,

    #[error("Failed to parse image from buffer")]
    FailedToParseImage,

    #[error("Tokio join error: {:?}", .0)]
    TokioJoinError(std::sync::Arc<JoinError>),

    #[error("IO error: {:?}", .0)]
    IoError(std::sync::Arc<std::io::Error>),

    #[error("Reqwest Error: {:?}", .0)]
    ReqwestError(std::sync::Arc<reqwest::Error>),

    #[error("Invalid album (does not exist in db)")]
    InvalidAlbum,

    #[error("Sempahore acquisition error")]
    AcquisitionError,

    #[error("Download task failure")]
    DownloadError(String),

    #[error("The entire active album contains 0 images with an associated file")]
    NoValidImageInAlbum,
}

impl From<sqlx::Error> for Error {
    fn from(sqlx_error: sqlx::Error) -> Self {
        Self::DatabaseError(std::sync::Arc::new(sqlx_error))
    }
}

impl From<image::error::ImageError> for Error {
    fn from(image_error: image::error::ImageError) -> Self {
        Self::ImageError(std::sync::Arc::new(image_error))
    }
}

impl From<lan_tcp::error::Error> for Error {
    fn from(networking_error: lan_tcp::error::Error) -> Self {
        Self::NetworkError(networking_error)
    }
}

impl From<rkyv::rancor::Error> for Error {
    fn from(rancor_error: rkyv::rancor::Error) -> Self {
        Self::RancorError(std::sync::Arc::new(rancor_error))
    }
}

impl From<onedrive_albums::error::Error> for Error {
    fn from(onedrive_error: onedrive_albums::error::Error) -> Self {
        Self::OneDriveError(onedrive_error)
    }
}

impl From<JoinError> for Error {
    fn from(join_error: JoinError) -> Self {
        Self::TokioJoinError(std::sync::Arc::new(join_error))
    }
}

impl From<std::io::Error> for Error {
    fn from(io_error: std::io::Error) -> Self {
        Self::IoError(std::sync::Arc::new(io_error))
    }
}

impl From<reqwest::Error> for Error {
    fn from(reqwest_error: reqwest::Error) -> Self {
        Self::ReqwestError(std::sync::Arc::new(reqwest_error))
    }
}

impl From<AcquireError> for Error {
    fn from(_: AcquireError) -> Self {
        Self::AcquisitionError
    }
}
