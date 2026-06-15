/*

USER[id, refresh_token]

ALBUM[id, name, num_items, cover_image_id, user_id]
ALBUM.user_id REFERENCES USER.id

PHOTO[id, name, created_date_time, width, height, location, size, album_id]
PHOTO.album_id REFERENCES ALBUM.id

// TODO ------------
SETTING[name, value]

*/

pub const CREATE_USER_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS USER (
        id VARCHAR(255),
        refresh_token TEXT NOT NULL,
        CONSTRAINT user_pk
            PRIMARY KEY (id)
    );
";

pub const CREATE_ALBUM_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS ALBUM (
        id VARCHAR(255),
        user_id VARCHAR(255) NOT NULL,
        name VARCHAR(255) NOT NULL,
        num_items INTEGER,
        cover_image_id VARCHAR(255),
        CONSTRAINT album_pk
            PRIMARY KEY (id, user_id),
        CONSTRAINT user_fk
            FOREIGN KEY (user_id)
            REFERENCES USER (id)
            ON DELETE CASCADE
    );
";

pub const CREATE_PHOTO_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS PHOTO (
        id VARCHAR(255),
        album_id VARCHAR(255),
        name VARCHAR(255) NOT NULL,
        created_date_time INTEGER NOT NULL,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        latitude FLOAT,
        longitude FLOAT,
        altitude FLOAT,
        size INTEGER,
        CONSTRAINT photo_pk
            PRIMARY KEY (id, album_id),
        CONSTRAINT album_fk
            FOREIGN KEY (album_id)
            REFERENCES ALBUM (id)
            ON DELETE CASCADE
    );
";

pub const SELECT_ALL_ALBUMS: &str = "
    SELECT * FROM ALBUM;
";
