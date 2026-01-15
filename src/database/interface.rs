use crate::{database::sql, error::Res};
use rusqlite_async::database::{DataLink, DatabaseParam, DatabaseParams};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub enum DatabaseInterfaceError {
    IncorrectNumberOfRows,
    MalformedRow
}

/// Create the tables without checking for success. If this fails, later DB calls will indicate.
pub fn create_tables(database: DataLink) -> Res<()> {
    database.execute(sql::CREATE_TOKEN_TABLE, DatabaseParams::empty())?;
    Ok(())
}

/// Insert the latest token into the database along with expieration seconds.
pub async fn insert_token(database: DataLink, refresh_token: String, expiration: usize) -> Res<()> {
    database.insert(sql::INSERT_TOKEN, DatabaseParams::new(vec![
        DatabaseParam::Usize(expiration),
        DatabaseParam::String(refresh_token)
    ])).await?;
    Ok(())
}

/// Retrive the latest token
pub async fn retrieve_token(database: DataLink) -> Res<(String, usize)> {
    let rows = database.query_map(sql::SELECT_TOKEN, DatabaseParams::empty()).await?;
    if rows.len() != 1 { return Err(DatabaseInterfaceError::IncorrectNumberOfRows.into()); }
    let row = rows.first().ok_or(DatabaseInterfaceError::IncorrectNumberOfRows)?;
    let token = row.first().ok_or(DatabaseInterfaceError::MalformedRow)?.string();
    let expiration = row.get(1).ok_or(DatabaseInterfaceError::MalformedRow)?.usize();
    Ok((token, expiration))
}
