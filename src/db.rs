use crate::config::{CatalogItem, CustomVarDef, Entry, EnvProfile, ItemKind, VarKind};
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_vars (
            name TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            separator TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY,
            kind TEXT NOT NULL,
            value TEXT NOT NULL,
            program TEXT,
            version TEXT,
            tags TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

pub fn save_item(conn: &Connection, item: &mut CatalogItem) -> Result<()> {
    let kind = match item.kind {
        ItemKind::Text => "text",
        ItemKind::Path => "path",
    };
    let tags_json = serde_json::to_string(&item.tags)?;
    if let Some(id) = item.id {
        conn.execute(
            "UPDATE items SET kind = ?1, value = ?2, program = ?3, version = ?4, tags = ?5 WHERE id = ?6",
            params![kind, item.value, item.program, item.version, tags_json, id],
        )?;
        return Ok(());
    }

    conn.execute(
        "INSERT INTO items (kind, value, program, version, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![kind, item.value, item.program, item.version, tags_json],
    )?;
    item.id = Some(conn.last_insert_rowid());
    Ok(())
}

pub fn delete_item(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn load_items(conn: &Connection) -> Result<Vec<CatalogItem>> {
    let mut stmt =
        conn.prepare("SELECT id, kind, value, program, version, tags FROM items ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let kind_s: String = row.get(1)?;
        let value: String = row.get(2)?;
        let program: Option<String> = row.get(3)?;
        let version: Option<String> = row.get(4)?;
        let tags_json: String = row.get(5)?;
        let kind = match kind_s.as_str() {
            "text" => ItemKind::Text,
            "path" => ItemKind::Path,
            _ => ItemKind::Text,
        };
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(CatalogItem {
            id: Some(id),
            kind,
            value,
            program,
            version,
            tags,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn save_custom_var_def(conn: &Connection, def: &CustomVarDef) -> Result<()> {
    let kind = match def.kind {
        VarKind::Scalar => "scalar",
        VarKind::List => "list",
    };
    conn.execute(
        "INSERT OR REPLACE INTO custom_vars (name, kind, separator) VALUES (?1, ?2, ?3)",
        params![def.name, kind, def.separator],
    )?;
    Ok(())
}

pub fn load_custom_var_defs(_conn: &Connection) -> Result<Vec<CustomVarDef>> {
    let mut stmt = _conn.prepare("SELECT name, kind, separator FROM custom_vars ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let kind_s: String = row.get(1)?;
        let separator: String = row.get(2)?;
        let kind = match kind_s.as_str() {
            "scalar" => VarKind::Scalar,
            "list" => VarKind::List,
            _ => VarKind::Scalar,
        };
        Ok(CustomVarDef {
            name,
            kind,
            separator,
        })
    })?;

    let mut defs = Vec::new();
    for r in rows {
        defs.push(r?);
    }
    Ok(defs)
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
    fn items_roundtrip_insert_load_update_delete() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let mut item = CatalogItem {
            id: None,
            kind: ItemKind::Text,
            value: "/opt/bin".to_string(),
            program: None,
            version: None,
            tags: vec!["core".to_string()],
        };
        save_item(&conn, &mut item)?;
        let id = item.id.expect("id should be set");

        let items = load_items(&conn)?;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, Some(id));
        assert_eq!(items[0].value, "/opt/bin");

        item.value = "/usr/local/bin".to_string();
        item.tags.push("updated".to_string());
        save_item(&conn, &mut item)?;

        let items = load_items(&conn)?;
        assert_eq!(items[0].value, "/usr/local/bin");
        assert!(items[0].tags.contains(&"updated".to_string()));

        delete_item(&conn, id)?;
        let items = load_items(&conn)?;
        assert!(items.is_empty());
        Ok(())
    }

    #[test]
    fn custom_var_defs_roundtrip() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        initialize_db(&conn)?;

        let def = CustomVarDef {
            name: "MY_PATH".to_string(),
            kind: VarKind::List,
            separator: ";".to_string(),
        };
        save_custom_var_def(&conn, &def)?;

        let defs = load_custom_var_defs(&conn)?;
        assert_eq!(defs, vec![def]);
        Ok(())
    }

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
