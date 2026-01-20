pub const CREATE_TOKEN_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        expiration INTEGER NOT NULL,
        token TEXT NOT NULL
    );
";

pub const CREATE_ALBUM_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Albums (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        onedrive_id TEXT UNIQUE,
        name TEXT NOT NULL
    );
";

pub const CREATE_PHOTO_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Photos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        onedrive_id TEXT UNIQUE,
        name TEXT NOT NULL,
        creation_date TEXT,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        filesize INTEGER NOT NULL,
        latitude FLOAT,
        longitude FLOAT,
        altitude FLOAT
    );
";

pub const CREATE_ENTRY_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Entries (
        album_id INTEGER,
        photo_id INTEGER,
        PRIMARY KEY (album_id, photo_id),
        FOREIGN KEY (album_id) REFERENCES Albums(id) ON DELETE CASCADE,
        FOREIGN KEY (photo_id) REFERENCES Photos(id) ON DELETE CASCADE
    );
";

pub const INSERT_TOKEN: &str = "
    INSERT INTO Tokens
    VALUES(null, ?, ?)
";

pub const SELECT_TOKEN: &str = "
    SELECT token, expiration FROM Tokens ORDER BY id DESC LIMIT 1;
";

pub const INSERT_PHOTO: &str = "
    INSERT INTO Photos (
        id,
        onedrive_id,
        name,
        creation_date,
        width,
        height,
        filesize,
        latitude,
        longitude,
        altitude
    ) VALUES (
        NULL,
        ?,
        ?,
        ?,
        ?,
        ?,
        ?,
        ?,
        ?,
        ?
    );
";

pub const INSERT_ALBUM: &str = "
    INSERT INTO Albums (
        id,
        onedrive_id,
        name
    ) VALUES (
        NULL,
        ?,
        ?
    );
";

pub const INSERT_ENTRY: &str = "
    INSERT INTO Entries (
        album_id,
        photo_id
    ) VALUES (
        ?,
        ?
    );
";

pub const DELETE_ENTRY: &str = "
    DELETE FROM Entries
    WHERE album_id = ?
      AND photo_id = ?;
";

pub const DELETE_PHOTO_BY_ID: &str = "
    DELETE FROM Photos
    WHERE id = ?;
";

pub const DELETE_ALBUM_BY_ID: &str = "
    DELETE FROM Albums
    WHERE id = ?;
";

pub const SELECT_PHOTOS_BY_ALBUM_ID: &str = "
    SELECT Photos.*
    FROM Photos
    INNER JOIN Entries ON Entries.photo_id = Photos.id
    WHERE Entries.album_id = ?;
";

pub const SELECT_ALL_ALBUMS: &str = "
    SELECT *
    FROM Albums;
";

pub const SELECT_ALL_PHOTOS: &str = "
    SELECT * FROM Photos;
";
