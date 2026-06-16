use tokio::sync::OnceCell;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePool;

use crate::backend::directories::storage::Storage;
use crate::backend::database::sql;
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

    /// Unwrap the singleton
    pub fn get_database<'a>() -> Res<&'a Database> {
        DATABASE.get().ok_or(Error::FailedToAccessDatabase)
    }

    /// Create tables (if they don't exist)
    pub async fn create_tables(&self) -> Res<()> {
        sqlx::query(sql::CREATE_USER_TABLE).execute(&self.pool).await?;
        sqlx::query(sql::CREATE_ALBUM_TABLE).execute(&self.pool).await?;
        sqlx::query(sql::CREATE_PHOTO_TABLE).execute(&self.pool).await?;
        Ok(())
    }

}
