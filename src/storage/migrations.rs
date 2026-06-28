use rusqlite::Connection;

pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    // Create subscriptions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS subscriptions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL UNIQUE,
            last_updated TEXT,
            update_interval INTEGER NOT NULL DEFAULT 24,
            upload INTEGER,
            download INTEGER,
            total INTEGER,
            expire TEXT
        );",
        [],
    )?;

    // Migrate: add profile_type column to subscriptions if it doesn't already exist
    // rusqlite returns an error if the column exists; we intentionally ignore it.
    let _ = conn.execute(
        "ALTER TABLE subscriptions ADD COLUMN profile_type TEXT NOT NULL DEFAULT 'subscription'",
        [],
    );

    // Create profiles table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            detail TEXT NOT NULL,
            delay INTEGER,
            is_active INTEGER NOT NULL DEFAULT 0,
            sub_id INTEGER,
            FOREIGN KEY(sub_id) REFERENCES subscriptions(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Create routing_rules table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS routing_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            rules TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0
        );",
        [],
    )?;

    // Insert default routing presets if not present
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM routing_rules",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        // Insert standard Bypass China preset
        conn.execute(
            "INSERT INTO routing_rules (name, rules, is_active) VALUES (?, ?, ?);",
            (
                "Bypass LAN & China",
                r#"[
                    {"outbound": "direct", "ip": ["geoip:private", "geoip:cn"], "domain": ["geosite:private", "geosite:cn"]},
                    {"outbound": "proxy", "domain": ["geosite:geolocation-!cn"]}
                ]"#,
                1,
            ),
        )?;
        
        // Insert Global routing preset
        conn.execute(
            "INSERT INTO routing_rules (name, rules, is_active) VALUES (?, ?, ?);",
            (
                "Global",
                r#"[
                    {"outbound": "proxy", "port": "1-65535"}
                ]"#,
                0,
            ),
        )?;
    }

    Ok(())
}
