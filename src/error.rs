use std::path::PathBuf;

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
