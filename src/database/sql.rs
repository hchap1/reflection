pub const CREATE_TOKEN_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Tokens (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        expiration INTEGER NOT NULL,
        token TEXT NOT NULL
    );
";

pub const INSERT_TOKEN: &str = "
    INSERT INTO Tokens
    VALUES(null, ?, ?)
";

pub const SELECT_TOKEN: &str = "
    SELECT (token, expiration) FROM Tokens ORDER BY id DESC LIMIT 1;
";
