use rkyv::Archive;
use rkyv::Deserialize;
use rkyv::Serialize;
use sqlx::{SqlitePool, query, query_as, sqlite::SqliteQueryResult};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, Archive)]
pub struct User {
    pub id: String,
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

    pub async fn create_user_table(pool: &SqlitePool) -> Result<SqliteQueryResult, sqlx::Error> {
        query("
            CREATE TABLE IF NOT EXISTS USER (
                id TEXT,
                refresh_token TEXT NOT NULL,
                expiry_date_time INTEGER NOT NULL,
                CONSTRAINT user_pk
                    PRIMARY KEY (id)
            );
        ")
        .execute(pool)
        .await
    }

    pub async fn create_album_table(pool: &SqlitePool) -> Result<SqliteQueryResult, sqlx::Error> {
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
        .execute(pool)
        .await
    }

    pub async fn create_photo_table(pool: &SqlitePool) -> Result<SqliteQueryResult, sqlx::Error> {
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
        .execute(pool)
        .await
    }

    // --- SELECT ---

    pub async fn select_all_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
        query_as::<_, User>("SELECT * FROM USER;")
            .fetch_all(pool)
            .await
    }

    pub async fn select_all_albums(pool: &SqlitePool) -> Result<Vec<Album>, sqlx::Error> {
        query_as::<_, Album>("SELECT * FROM ALBUM;")
            .fetch_all(pool)
            .await
    }

    pub async fn select_albums_by_user(
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<Vec<Album>, sqlx::Error> {
        query_as::<_, Album>("SELECT * FROM ALBUM WHERE user_id = ?;")
            .bind(user_id)
            .fetch_all(pool)
            .await
    }

    pub async fn select_photos_by_album(
        pool: &SqlitePool,
        album_id: &str,
        user_id: &str,
    ) -> Result<Vec<Photo>, sqlx::Error> {
        query_as::<_, Photo>("SELECT * FROM PHOTO WHERE album_id = ? AND user_id = ?;")
            .bind(album_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
    }

    // --- INSERT / UPDATE ---

    pub async fn insert_or_update_user(
        pool: &SqlitePool,
        user: &User,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        query("
            INSERT INTO USER (id, refresh_token, expiry_date_time)
            VALUES(?, ?, ?)
            ON CONFLICT(id)
            DO UPDATE SET
                refresh_token = excluded.refresh_token
                expiry_date_time = excluded.expiry_date_time;
        ")
        .bind(&user.id)
        .bind(&user.refresh_token)
        .bind(&user.expiry_date_time)
        .execute(pool)
        .await
    }

    pub async fn insert_or_update_album(
        pool: &SqlitePool,
        album: &Album,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
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
        .execute(pool)
        .await
    }

    pub async fn insert_or_update_photo(
        pool: &SqlitePool,
        photo: &Photo,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
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
        .execute(pool)
        .await
    }

    // --- DELETE ---

    pub async fn delete_user(
        pool: &SqlitePool,
        user_id: &str,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        query("DELETE FROM USER WHERE id = ?;")
            .bind(user_id)
            .execute(pool)
            .await
    }

    pub async fn delete_album(
        pool: &SqlitePool,
        album_id: &str,
        user_id: &str,
    ) -> Result<SqliteQueryResult, sqlx::Error> {
        query("DELETE FROM ALBUM WHERE id = ? AND user_id = ?;")
            .bind(album_id)
            .bind(user_id)
            .execute(pool)
            .await
    }
}
