pub type Res<T> = Result<T, Error>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {

    #[error("Failed to find storage location")]
    FailedToFindStorageLocation
}
