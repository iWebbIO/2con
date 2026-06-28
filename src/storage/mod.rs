pub mod migrations;

use crate::model::{ProfileItem, RoutingItem, SubItem, ProfileType, AppSettings};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;

#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = db_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        migrations::init_db(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // --- Profiles ---
    pub fn add_profile(&self, item: &ProfileItem) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO profiles (name, address, port, protocol, detail, delay, is_active, sub_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                item.name,
                item.address,
                item.port,
                item.protocol,
                item.detail,
                item.delay,
                if item.is_active { 1 } else { 0 },
                item.sub_id
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_profiles(&self) -> Result<Vec<ProfileItem>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, address, port, protocol, detail, delay, is_active, sub_id FROM profiles")?;
        let rows = stmt.query_map([], |row| {
            let is_active_int: i32 = row.get(7)?;
            Ok(ProfileItem {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                address: row.get(2)?,
                port: row.get(3)?,
                protocol: row.get(4)?,
                detail: row.get(5)?,
                delay: row.get(6)?,
                is_active: is_active_int == 1,
                sub_id: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_active_profile(&self) -> Result<Option<ProfileItem>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, address, port, protocol, detail, delay, is_active, sub_id FROM profiles WHERE is_active = 1 LIMIT 1",
            [],
            |row| {
                let is_active_int: i32 = row.get(7)?;
                Ok(ProfileItem {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    address: row.get(2)?,
                    port: row.get(3)?,
                    protocol: row.get(4)?,
                    detail: row.get(5)?,
                    delay: row.get(6)?,
                    is_active: is_active_int == 1,
                    sub_id: row.get(8)?,
                })
            },
        ).optional()
    }

    pub fn set_active_profile(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE profiles SET is_active = 0", [])?;
        conn.execute("UPDATE profiles SET is_active = 1 WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn update_profile_delay(&self, id: i64, delay: Option<i32>) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE profiles SET delay = ? WHERE id = ?", params![delay, id])?;
        Ok(())
    }

    pub fn delete_profile(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM profiles WHERE id = ?", params![id])?;
        Ok(())
    }

    // --- Subscriptions ---
    pub fn add_subscription(&self, item: &SubItem) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO subscriptions (name, url, last_updated, update_interval, upload, download, total, expire, profile_type)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                item.name,
                item.url,
                item.last_updated,
                item.update_interval,
                item.upload,
                item.download,
                item.total,
                item.expire,
                item.profile_type.to_string()
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_subscriptions(&self) -> Result<Vec<SubItem>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, url, last_updated, update_interval, upload, download, total, expire, profile_type FROM subscriptions")?;
        let rows = stmt.query_map([], |row| {
            let pt_str: String = row.get::<_, Option<String>>(9)?.unwrap_or_default();
            let profile_type = pt_str.parse::<ProfileType>().unwrap_or_default();
            Ok(SubItem {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                url: row.get(2)?,
                last_updated: row.get(3)?,
                update_interval: row.get(4)?,
                upload: row.get(5)?,
                download: row.get(6)?,
                total: row.get(7)?,
                expire: row.get(8)?,
                profile_type,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn rename_subscription(&self, id: i64, new_name: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE subscriptions SET name = ? WHERE id = ?", params![new_name, id])?;
        Ok(())
    }

    pub fn clear_profiles_by_sub_id(&self, sub_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM profiles WHERE sub_id = ?", params![sub_id])?;
        Ok(())
    }

    pub fn delete_subscription(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM subscriptions WHERE id = ?", params![id])?;
        Ok(())
    }

    // --- Routing ---
    pub fn get_routing_rules(&self) -> Result<Vec<RoutingItem>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, rules, is_active FROM routing_rules")?;
        let rows = stmt.query_map([], |row| {
            let is_active_int: i32 = row.get(3)?;
            Ok(RoutingItem {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                rules: row.get(2)?,
                is_active: is_active_int == 1,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn set_active_routing(&self, preset_name: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE routing_rules SET is_active = 0", [])?;
        conn.execute("UPDATE routing_rules SET is_active = 1 WHERE name = ?", params![preset_name])?;
        Ok(())
    }
}

// --- App Settings Path Resolvers ---
pub fn get_app_dir() -> PathBuf {
    let mut path = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push(".twocon");
    path
}

pub fn load_settings() -> AppSettings {
    let mut path = get_app_dir();
    path.push("settings.json");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                return settings;
            }
        }
    }
    AppSettings::default()
}

pub fn save_settings(settings: &AppSettings) {
    let mut path = get_app_dir();
    let _ = fs::create_dir_all(&path);
    path.push("settings.json");
    if let Ok(content) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, content);
    }
}
