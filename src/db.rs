use crate::config::{Entry, EnvProfile};
use anyhow::Result;
use rusqlite::{params, types::Type, Connection};
use std::env;
use std::path::PathBuf;

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

/// Rename a profile without leaving stale rows behind.
pub fn rename_profile(conn: &Connection, old_name: &str, new_name: &str) -> Result<()> {
    let updated = conn.execute(
        "UPDATE profiles SET name = ?1 WHERE name = ?2",
        params![new_name, old_name],
    )?;
    if updated == 0 {
        anyhow::bail!("profile not found: {old_name}");
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_profile_updates_row_in_place() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let p = EnvProfile {
            name: "old".to_string(),
            entries: vec![Entry::CFlag("-O2".to_string())],
        };
        save_profile(&conn, &p)?;

        rename_profile(&conn, "old", "new")?;

        // Only one row should exist, with the new name.
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))?;
        assert_eq!(count, 1);

        let loaded = load_profile(&conn, "new")?;
        assert_eq!(loaded.name, "new");
        assert_eq!(loaded.entries.len(), 1);

        assert!(load_profile(&conn, "old").is_err());
        Ok(())
    }
}
