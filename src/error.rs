pub type Res<T> = Result<Error, T>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    
}
