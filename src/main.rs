pub mod error;
pub mod backend;

use crate::backend::directories::storage::Storage;
use crate::backend::database::database_backend::Database;
use crate::error::Res;

#[tokio::main]
async fn main() -> Res<()> {
    Storage::initialise()?;
    Database::initialise().await?;

    let storage = &crate::backend::directories::storage::STORAGE;
    println!("{:?}", storage.get().unwrap().get_database_path());

    let database = Database::get_database()?;
    database.create_tables().await?;

    Ok(())
}
