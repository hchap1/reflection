use std::path::PathBuf;

pub type Res<T> = Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {

    #[error("Failed to find storage location")]
    FailedToFindStorageLocation,

    #[error("Could not check existance of root")]
    CheckFileExistsError,

    #[error("Could not create directory {:?}", .0)]
    FailedToCreateDirectory(PathBuf)
}
