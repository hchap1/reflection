use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;
use sqlx::{query, query_as, query_scalar, sqlite::SqliteQueryResult};

use crate::backend::database::database_backend::Database;
use crate::error::Error;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Archive)]
pub struct User {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub refresh_token: String,
    pub expiry_date_time: i64
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Archive)]
pub struct Album {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub num_items: Option<i64>,
    pub cover_image_id: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Archive)]
pub struct Photo {
    pub id: String,
    pub album_id: String,
    pub user_id: String,
    pub name: String,
    pub created_date_time: i64,
    pub width: i64,
    pub height: i64,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub size: Option<i64>,
}

pub struct SQL;

impl SQL {
    // --- DDL ---

    pub async fn create_user_table() -> Result<SqliteQueryResult, Error> {
        query("
            CREATE TABLE IF NOT EXISTS USER (
                id TEXT,
                refresh_token TEXT NOT NULL,
                name TEXT,
                email TEXT,
                expiry_date_time INTEGER NOT NULL,
                CONSTRAINT user_pk
                    PRIMARY KEY (id)
            );
        ")
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn create_album_table() -> Result<SqliteQueryResult, Error> {
        query("
            CREATE TABLE IF NOT EXISTS ALBUM (
                id TEXT,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                num_items INTEGER,
                cover_image_id TEXT,
                CONSTRAINT album_pk
                    PRIMARY KEY (id, user_id),
                CONSTRAINT user_fk
                    FOREIGN KEY (user_id)
                    REFERENCES USER (id)
                    ON DELETE CASCADE
            );
        ")
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn create_photo_table() -> Result<SqliteQueryResult, Error> {
        query("
            CREATE TABLE IF NOT EXISTS PHOTO (
                id TEXT,
                album_id TEXT,
                user_id TEXT,
                name TEXT NOT NULL,
                created_date_time INTEGER NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                latitude REAL,
                longitude REAL,
                altitude REAL,
                size INTEGER,
                CONSTRAINT photo_pk
                    PRIMARY KEY (id, album_id, user_id),
                CONSTRAINT album_fk
                    FOREIGN KEY (album_id, user_id)
                    REFERENCES ALBUM (id, user_id)
                    ON DELETE CASCADE
            );
        ")
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    // --- SELECT ---

    pub async fn select_all_users() -> Result<Vec<User>, Error> {
        query_as::<_, User>("SELECT * FROM USER;")
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn select_user_by_id(
        user_id: &str
    ) -> Result<Option<User>, Error> {
        query_as::<_, User>("SELECT * FROM USER WHERE id = ?;")
            .bind(user_id)
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
            .map(|v| v.into_iter().next())
    }

    pub async fn select_all_albums() -> Result<Vec<Album>, Error> {
        query_as::<_, Album>("SELECT * FROM ALBUM;")
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn select_album_by_id(
        album_id: String,
        user_id: String
    ) -> Result<Option<Album>, Error> {
        query_as::<_, Album>("SELECT * FROM ALBUM WHERE id = ? AND user_id = ?;")
            .bind(album_id)
            .bind(user_id)
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
            .map(|v| v.into_iter().next())
    }

    pub async fn select_albums_by_user(
        user_id: &str,
    ) -> Result<Vec<Album>, Error> {
        query_as::<_, Album>("SELECT * FROM ALBUM WHERE user_id = ?;")
            .bind(user_id)
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn select_photos_by_album(
        album_id: String,
        user_id: String,
    ) -> Result<Vec<Photo>, Error> {
        query_as::<_, Photo>("SELECT * FROM PHOTO WHERE album_id = ? AND user_id = ?;")
            .bind(album_id)
            .bind(user_id)
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn select_photo_by_id(
        photo_id: &str,
        album_id: &str,
        user_id: &str,
    ) -> Result<Option<Photo>, Error> {
        query_as::<_, Photo>("SELECT * FROM PHOTO WHERE id = ? AND album_id = ? AND user_id = ?;")
            .bind(photo_id)
            .bind(album_id)
            .bind(user_id)
            .fetch_all(Database::get_database_pool()?)
            .await.map_err(Error::from)
            .map(|photos| photos.into_iter().next())
    }

    // --- INSERT / UPDATE ---

    pub async fn insert_or_update_user(
        user: &User,
    ) -> Result<SqliteQueryResult, Error> {
        query("
            INSERT INTO USER (id, name, email, refresh_token, expiry_date_time)
            VALUES(?, ?, ?, ?, ?)
            ON CONFLICT(id)
            DO UPDATE SET
                refresh_token = excluded.refresh_token,
                name = excluded.name,
                email = excluded.email,
                expiry_date_time = excluded.expiry_date_time;
        ")
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.email)
        .bind(&user.refresh_token)
        .bind(&user.expiry_date_time)
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn insert_or_update_album(
        album: Album,
    ) -> Result<SqliteQueryResult, Error> {
        query("
            INSERT INTO ALBUM (id, user_id, name, num_items, cover_image_id)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id, user_id)
            DO UPDATE SET
                name = excluded.name,
                num_items = excluded.num_items,
                cover_image_id = excluded.cover_image_id;
        ")
        .bind(&album.id)
        .bind(&album.user_id)
        .bind(&album.name)
        .bind(album.num_items)
        .bind(&album.cover_image_id)
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn insert_or_update_photo(
        photo: &Photo,
    ) -> Result<SqliteQueryResult, Error> {
        query("
            INSERT INTO PHOTO (
                id, album_id, user_id, name, created_date_time,
                width, height, latitude, longitude, altitude, size
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id, album_id, user_id)
            DO UPDATE SET
                name = excluded.name,
                created_date_time = excluded.created_date_time,
                width = excluded.width,
                height = excluded.height,
                latitude = excluded.latitude,
                longitude = excluded.longitude,
                altitude = excluded.altitude,
                size = excluded.size;
        ")
        .bind(&photo.id)
        .bind(&photo.album_id)
        .bind(&photo.user_id)
        .bind(&photo.name)
        .bind(photo.created_date_time)
        .bind(photo.width)
        .bind(photo.height)
        .bind(photo.latitude)
        .bind(photo.longitude)
        .bind(photo.altitude)
        .bind(photo.size)
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    // --- DELETE ---

    pub async fn delete_user(user_id: &str) -> Result<SqliteQueryResult, Error> {
        query("DELETE FROM USER WHERE id = ?;")
            .bind(user_id)
            .execute(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn delete_album(
        album_id: &str,
        user_id: &str,
    ) -> Result<SqliteQueryResult, Error> {
        query("DELETE FROM ALBUM WHERE id = ? AND user_id = ?;")
            .bind(album_id)
            .bind(user_id)
            .execute(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn delete_album_by_name(
        name: &str,
        user_id: &str,
    ) -> Result<SqliteQueryResult, Error> {
        query("DELETE FROM ALBUM WHERE name = ? AND user_id = ?;")
            .bind(name)
            .bind(user_id)
            .execute(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn create_settings_table() -> Result<SqliteQueryResult, Error> {
        query("
            CREATE TABLE IF NOT EXISTS SETTINGS (
                name TEXT,
                value TEXT NOT NULL,
                CONSTRAINT settings_pk
                    PRIMARY KEY (name)
            );
        ")
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn insert_or_update_setting(
        name: &str,
        value: &str,
    ) -> Result<SqliteQueryResult, Error> {
        query("
            INSERT INTO SETTINGS (name, value)
            VALUES (?, ?)
            ON CONFLICT(name)
            DO UPDATE SET
                value = excluded.value;
        ")
        .bind(name)
        .bind(value)
        .execute(Database::get_database_pool()?)
        .await.map_err(Error::from)
    }

    pub async fn select_setting_by_name(name: &str) -> Result<Option<String>, Error> {
        query_scalar::<_, String>("SELECT value FROM SETTINGS WHERE name = ?;")
            .bind(name)
            .fetch_optional(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }

    pub async fn delete_setting_by_name(
        name: &str,
    ) -> Result<SqliteQueryResult, Error> {
        query("DELETE FROM SETTINGS WHERE name = ?;")
            .bind(name)
            .execute(Database::get_database_pool()?)
            .await.map_err(Error::from)
    }
}
