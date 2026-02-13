pub static QUERIES: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                temp_old_password TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                avatar BLOB
            );",
    "CREATE TABLE IF NOT EXISTS passwords (
                id INTEGER PRIMARY KEY,
                user_id INTEGER NOT NULL,
                location TEXT NOT NULL,
                password BLOB NOT NULL,
                notes TEXT,
                strength TEXT NOT NULL CHECK (strength IN ('weak', 'medium', 'strong')),
                created_at TEXT DEFAULT (datetime('now')),
                nonce BLOB NOT NULL UNIQUE,
                FOREIGN KEY(user_id) REFERENCES users(id)
    )",
];
