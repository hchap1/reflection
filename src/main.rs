pub mod error;
pub mod backend;

use crate::backend::directories::storage::Storage;
use crate::error::Res;

fn main() -> Res<()> {
    Storage::initialise()?;

    let storage = &crate::backend::directories::storage::STORAGE;
    println!("{:?}", storage.get().unwrap().get_database_path());

    Ok(())
}
