use tokio::sync::OnceCell;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePool;

use crate::backend::database::sql::SQL;
use crate::backend::directories::storage::Storage;
use crate::error::Error;
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
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(options)
            .await?;

        DATABASE.get_or_init(
            async || Database {
                pool
            }
        ).await;

        Ok(())
    }

    /// Unwrap the singleton
    pub fn get_database_pool<'a>() -> Res<&'a SqlitePool> {
        Ok(&DATABASE.get().ok_or(Error::FailedToAccessDatabase)?.pool)
    }

    /// Create tables (if they don't exist)
    pub async fn create_tables(&self) -> Res<()> {
        SQL::create_user_table(&self.pool).await?;
        SQL::create_album_table(&self.pool).await?;
        SQL::create_photo_table(&self.pool).await?;
        Ok(())
    }

}
