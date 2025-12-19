use crate::db;
use crate::tui::daisyui_themes;
use crate::tui::dialogs::{create_custom_var_dialog, create_or_edit_item_dialog};
use crate::tui::state::AppState;
use crate::tui::view::View;
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;

fn all_commands() -> Vec<String> {
    vec![
        "quit".to_string(),
        "profiles".to_string(),
        "vars".to_string(),
        "defs".to_string(),
        "parts".to_string(),
        "items".to_string(),
        "preview".to_string(),
        "export".to_string(),
        "use".to_string(),
        "themes".to_string(),
        "theme".to_string(),
        "new-var".to_string(),
        "new-item".to_string(),
        "help".to_string(),
    ]
}

pub fn refresh_command_suggestions(app: &mut AppState) {
    let input = app.command_input.trim_start();
    let mut suggestions = Vec::new();

    if input.starts_with("use ") {
        let q = input.trim_start_matches("use ").trim().to_lowercase();
        for p in &app.profiles {
            if q.is_empty() || p.name.to_lowercase().contains(&q) {
                suggestions.push(format!("use {}", p.name));
            }
        }
    } else if input.starts_with("theme ") {
        let q = input
            .trim_start_matches("theme ")
            .trim()
            .to_lowercase();
        for name in daisyui_themes::names() {
            if q.is_empty() || name.to_lowercase().contains(&q) {
                suggestions.push(format!("theme {name}"));
            }
        }
    } else {
        let cmds = all_commands();
        let q = input.to_lowercase();
        for c in cmds {
            if q.is_empty() || c.to_lowercase().starts_with(&q) || c.to_lowercase().contains(&q) {
                suggestions.push(c);
            }
        }
    }

    suggestions.sort();
    suggestions.dedup();
    app.command_suggestions = suggestions;
    if app.command_suggestions.is_empty() {
        app.command_selected = 0;
    } else {
        app.command_selected = app
            .command_selected
            .min(app.command_suggestions.len().saturating_sub(1));
    }
}

pub fn pick_command_to_execute(app: &AppState) -> String {
    let typed = app.command_input.trim().to_string();
    if typed.is_empty() {
        return app
            .command_suggestions
            .get(app.command_selected)
            .cloned()
            .unwrap_or_default();
    }
    if app.command_suggestions.is_empty() {
        return typed;
    }
    // If user typed something that isn't an exact command, execute the selected suggestion.
    if app.command_suggestions.iter().any(|s| s == &typed) {
        return typed;
    }
    app.command_suggestions
        .get(app.command_selected)
        .cloned()
        .unwrap_or(typed)
}

pub fn execute_command<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    cmd: &str,
) -> Result<bool> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Ok(false);
    }

    if cmd == "quit" || cmd == "q" || cmd == "exit" {
        return Ok(true);
    }
    if cmd == "profiles" {
        app.active_view = View::Profiles;
        return Ok(false);
    }
    if cmd == "vars" {
        app.active_view = View::Vars;
        return Ok(false);
    }
    if cmd == "defs" {
        app.active_view = View::Defs;
        return Ok(false);
    }
    if cmd == "parts" {
        app.active_view = View::Parts;
        return Ok(false);
    }
    if cmd == "items" {
        app.active_view = View::Items;
        return Ok(false);
    }
    if cmd == "preview" {
        app.active_view = View::Preview;
        return Ok(false);
    }
    if cmd == "export" {
        app.active_view = View::Export;
        return Ok(false);
    }
    if cmd == "themes" {
        app.active_view = View::Help;
        app.status = format!(
            "{} themes available (use :theme <name>)",
            daisyui_themes::THEMES.len()
        );
        return Ok(false);
    }
    if cmd == "theme" {
        app.status = "Usage: theme <name>".to_string();
        return Ok(false);
    }
    if cmd == "new-var" {
        if let Some(def) = create_custom_var_dialog(terminal)? {
            db::save_custom_var_def(&app.conn, &def)?;
            app.refresh_var_options()?;
            app.status = format!("saved var def: {}", def.name);
        }
        return Ok(false);
    }
    if cmd == "new-item" {
        if let Some(mut item) = create_or_edit_item_dialog(terminal, None)? {
            db::save_item(&app.conn, &mut item)?;
            app.refresh_items()?;
            app.status = format!("saved item: {}", item.value);
        }
        return Ok(false);
    }
    if cmd == "help" {
        app.active_view = View::Help;
        return Ok(false);
    }

    if let Some(rest) = cmd.strip_prefix("use ") {
        let name = rest.trim();
        if let Some((idx, _)) = app
            .profiles
            .iter()
            .enumerate()
            .find(|(_, p)| p.name == name)
        {
            app.active_profile_index = idx;
            app.status = format!("profile: {name}");
        } else {
            app.status = format!("profile not found: {name}");
        }
        return Ok(false);
    }

    if let Some(rest) = cmd.strip_prefix("theme ") {
        let name = rest.trim();
        if name.is_empty() {
            app.status = "Usage: theme <name>".to_string();
            return Ok(false);
        }
        app.set_theme_preset(name, true)?;
        app.status = format!("theme: {}", app.theme.name);
        return Ok(false);
    }

    Ok(false)
}

