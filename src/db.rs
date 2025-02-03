use std::env;
use std::path::PathBuf;
use crate::config::{Entry, EnvProfile};
use anyhow::Result;
use rusqlite::{params, types::Type, Connection};
use serde_json;

/// Open (or create) the SQLite database.
pub fn establish_connection() -> Result<Connection> {
    let home = env::var("HOME").expect("HOME environment variable not set");
    
    let conn = Connection::open(PathBuf::from(home).join(".bath.db"))?;
    initialize_db(&conn)?;
    Ok(conn)
}

/// Create the profiles table if it does not exist.
pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS profiles (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            entries TEXT
        )",
        [],
    )?;
    Ok(())
}

/// Save (or update) a profile.
pub fn save_profile(conn: &Connection, profile: &EnvProfile) -> Result<()> {
    let entries_json = serde_json::to_string(&profile.entries)?;
    conn.execute(
        "INSERT OR REPLACE INTO profiles (name, entries) VALUES (?1, ?2)",
        params![profile.name, entries_json],
    )?;
    Ok(())
}

/// Load a profile by name.
pub fn load_profile(conn: &Connection, name: &str) -> Result<EnvProfile> {
    let mut stmt = conn.prepare("SELECT name, entries FROM profiles WHERE name = ?1")?;
    let profile = stmt.query_row([name], |row| {
        let name: String = row.get(0)?;
        let entries_json: String = row.get(1)?;
        let entries: Vec<Entry> = serde_json::from_str(&entries_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(e)))?;
        Ok(EnvProfile { name, entries })
    })?;
    Ok(profile)
}

/// Load all profiles from the database.
pub fn load_all_profiles(conn: &Connection) -> Result<Vec<EnvProfile>> {
    let mut stmt = conn.prepare("SELECT name, entries FROM profiles")?;
    let profile_iter = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let entries_json: String = row.get(1)?;
        let entries: Vec<Entry> = serde_json::from_str(&entries_json)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?;
        Ok(EnvProfile { name, entries })
    })?;
    let mut profiles = Vec::new();
    for profile in profile_iter {
        profiles.push(profile?);
    }
    Ok(profiles)
}

/// Delete a profile by name.
pub fn delete_profile(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM profiles WHERE name = ?1", params![name])?;
    Ok(())
}
