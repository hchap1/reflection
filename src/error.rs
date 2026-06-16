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
    ImageError(std::sync::Arc<image::error::ImageError>)
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
