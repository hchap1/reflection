use tokio::sync::OnceCell;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePool;

use crate::backend::directories::storage::Storage;
use crate::error::Res;

pub static DATABASE: OnceCell<Database> = OnceCell::const_new();

pub struct Database {
    pool: SqlitePool
}

impl Database {
    
    pub async fn initialise() -> Res<()> {

        let storage = Storage::get_storage()?;

        let options = SqliteConnectOptions::new()
            .filename(storage.get_database_path())
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await?;

        DATABASE.get_or_init(
            async || Database {
                pool
            }
        ).await;

        Ok(())
    }

}
