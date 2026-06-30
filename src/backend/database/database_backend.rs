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

        println!("Options created");
        let pool = SqlitePool::connect_with(options)
            .await?;

        println!("Connected");

        DATABASE.get_or_init(
            async || Database {
                pool
            }
        ).await;

        Self::create_tables().await?;

        println!("Tables created");

        Ok(())
    }

    /// Unwrap the singleton
    pub fn get_database_pool<'a>() -> Res<&'a SqlitePool> {
        println!("Trying to get database pool...");
        let res = &DATABASE.get().ok_or(Error::FailedToAccessDatabase);

        match res {
            Ok(database) => Ok(&database.pool),
            Err(e) => {
                println!("... Failed to get database pool");
                Err(e.clone())
            }
        }
    }

    /// Create tables (if they don't exist)
    pub async fn create_tables() -> Res<()> {
        SQL::create_user_table().await?;
        SQL::create_album_table().await?;
        SQL::create_photo_table().await?;
        SQL::create_settings_table().await?;
        Ok(())
    }

}
