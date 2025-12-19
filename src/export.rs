// src/export.rs

use crate::config::{Entry, EnvProfile};
use crate::db;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub enum OperationMode {
    Prepend,
    Append,
    Replace,
}

fn shell_double_quote_literal(s: &str) -> String {
    // Escape for inside double quotes.
    //
    // Intentionally does NOT escape '$' so things like $HOME and ${VAR}
    // expand at eval-time as requested.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(ch),
        }
    }
    out
}

fn export_assignment(var_name: &str, value: &str, sep: &str, mode: OperationMode) -> String {
    let escaped_value = shell_double_quote_literal(value);
    match mode {
        OperationMode::Prepend => {
            // Only insert the separator + existing var if it is non-empty:
            // VAR="<new>${VAR:+<sep>${VAR}}"
            let tail = format!("${{{}:+{}${{{}}}}}", var_name, sep, var_name);
            format!("export {}=\"{}{}\";", var_name, escaped_value, tail)
        }
        OperationMode::Append => {
            // Only insert the existing var + separator if it is non-empty:
            // VAR="${VAR:+${VAR}<sep>}<new>"
            let head = format!("${{{}:+${{{}}}{}}}", var_name, var_name, sep);
            format!("export {}=\"{}{}\";", var_name, head, escaped_value)
        }
        OperationMode::Replace => format!("export {}=\"{}\";", var_name, escaped_value),
    }
}

fn entry_value(entry: &Entry) -> String {
    match entry {
        Entry::Path(pe) => pe.path.clone(),
        Entry::CPath(s)
        | Entry::CInclude(s)
        | Entry::CPlusInclude(s)
        | Entry::OBJCInclude(s)
        | Entry::CPPFlag(s)
        | Entry::CFlag(s)
        | Entry::CXXFlag(s)
        | Entry::LDFlag(s)
        | Entry::LibraryPath(s)
        | Entry::LDLibraryPath(s)
        | Entry::LDRunPath(s)
        | Entry::RanLib(s)
        | Entry::CC(s)
        | Entry::CXX(s)
        | Entry::AR(s)
        | Entry::Strip(s)
        | Entry::GCCExecPrefix(s)
        | Entry::CollectGCCOptions(s)
        | Entry::Lang(s) => s.clone(),
        Entry::CustomScalar { value, .. } => value.clone(),
        Entry::CustomPart { value, .. } => value.clone(),
    }
}

/// Generates an export command for a single Entry (treated as the new value).
pub fn generate_export_line(entry: &Entry, mode: OperationMode) -> String {
    let var_name = entry.var_name();
    let value = entry_value(entry);
    let sep = entry.separator();
    export_assignment(var_name.as_ref(), &value, sep.as_ref(), mode)
}

/// Generates the full export commands for a given profile.
pub fn generate_full_export(profile: &EnvProfile, mode: OperationMode) -> String {
    // One export line per variable, with parts joined in the order they were added.
    //
    // This keeps editing at the parts level in storage/UI, but export happens at the
    // variable level (e.g. one PATH assignment).
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (String, Vec<String>)> = HashMap::new();

    for entry in &profile.entries {
        let var = entry.var_name().into_owned();
        let sep = entry.separator().into_owned();
        if !groups.contains_key(&var) {
            order.push(var.clone());
            groups.insert(var.clone(), (sep, Vec::new()));
        }
        if let Some((_sep, parts)) = groups.get_mut(&var) {
            parts.push(entry_value(entry));
        }
    }

    let mut lines = Vec::new();
    for var in order {
        if let Some((sep, parts)) = groups.remove(&var) {
            let joined = parts.join(&sep);
            lines.push(export_assignment(&var, &joined, &sep, mode));
        }
    }
    lines.join("\n")
}

/// Exports the given profile as export commands (without a shebang)
/// so you can eval the commands in your shell.
pub fn export_profile(profile_name: &str, mode: OperationMode) -> Result<()> {
    let conn = db::establish_connection()?;
    let profile: EnvProfile = db::load_profile(&conn, profile_name)?;
    let out = generate_full_export(&profile, mode);
    if !out.is_empty() {
        println!("{out}");
    }
    Ok(())
}

/// Launches an interactive ratatui TUI to select a profile to export.
/// When a profile is selected, its export commands (according to the given mode)
/// are printed to stdout.
pub fn interactive_export(mode: OperationMode) -> Result<()> {
    use crossterm::event::{poll, read, Event, KeyCode};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::Style;
    use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
    use ratatui::Terminal;
    use std::io::stdout;

    let conn = db::establish_connection()?;
    let profiles = db::load_all_profiles(&conn)?;
    if profiles.is_empty() {
        println!("No profiles available to export.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));
    let items: Vec<ListItem> = profiles
        .iter()
        .map(|p| ListItem::new(p.name.clone()))
        .collect();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Select a profile to export (Enter: select, Esc: cancel)");
            let list = List::new(items.clone())
                .block(block)
                .highlight_style(Style::default().bg(ratatui::style::Color::Blue));
            f.render_stateful_widget(list, chunks[1], &mut list_state);
        })?;

        if poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = read()? {
                match key.code {
                    KeyCode::Esc => {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;
                        return Ok(());
                    }
                    KeyCode::Down => {
                        let i = match list_state.selected() {
                            Some(i) if i >= profiles.len() - 1 => 0,
                            Some(i) => i + 1,
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match list_state.selected() {
                            Some(0) | None => profiles.len() - 1,
                            Some(i) => i - 1,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = list_state.selected() {
                            let selected = &profiles[i];
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            export_profile(&selected.name, mode)?;
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Entry, PathEntry};

    #[test]
    fn replace_mode_uses_double_quotes_and_escapes_inner_double_quotes() {
        let e = Entry::CC("O\"Reilly".to_string());
        assert_eq!(
            generate_export_line(&e, OperationMode::Replace),
            "export CC=\"O\\\"Reilly\";"
        );
    }

    #[test]
    fn replace_mode_allows_shell_expansion_of_dollar_vars() {
        let e = Entry::CC("/opt/$HOME/bin".to_string());
        assert_eq!(
            generate_export_line(&e, OperationMode::Replace),
            "export CC=\"/opt/$HOME/bin\";"
        );
    }

    #[test]
    fn prepend_mode_generates_single_export_per_var_with_all_parts_in_order() {
        let profile = EnvProfile {
            name: "p".to_string(),
            entries: vec![
                Entry::Path(PathEntry {
                    path: "/p1".to_string(),
                    program: "tool".to_string(),
                    version: "1".to_string(),
                }),
                Entry::Path(PathEntry {
                    path: "/p2".to_string(),
                    program: "tool".to_string(),
                    version: "2".to_string(),
                }),
                Entry::CFlag("-O2 -Wall".to_string()),
            ],
        };

        let out = generate_full_export(&profile, OperationMode::Prepend);

        let path_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("export PATH="))
            .collect();
        assert_eq!(path_lines.len(), 1, "expected a single PATH export line");
        assert_eq!(path_lines[0], "export PATH=\"/p1:/p2${PATH:+:${PATH}}\";");

        let cflag_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("export CFLAGS="))
            .collect();
        assert_eq!(cflag_lines.len(), 1, "expected a single CFLAGS export line");
        assert_eq!(
            cflag_lines[0],
            "export CFLAGS=\"-O2 -Wall${CFLAGS:+ ${CFLAGS}}\";"
        );
    }

    #[test]
    fn append_mode_uses_parameter_expansion_to_avoid_leading_separators() {
        let profile = EnvProfile {
            name: "p".to_string(),
            entries: vec![
                Entry::Path(PathEntry {
                    path: "/p1".to_string(),
                    program: "tool".to_string(),
                    version: "1".to_string(),
                }),
                Entry::Path(PathEntry {
                    path: "/p2".to_string(),
                    program: "tool".to_string(),
                    version: "2".to_string(),
                }),
            ],
        };

        let out = generate_full_export(&profile, OperationMode::Append);
        let path_lines: Vec<&str> = out
            .lines()
            .filter(|l| l.starts_with("export PATH="))
            .collect();
        assert_eq!(path_lines.len(), 1, "expected a single PATH export line");
        assert_eq!(path_lines[0], "export PATH=\"${PATH:+${PATH}:}/p1:/p2\";");
    }

    #[test]
    fn statements_end_with_semicolon() {
        let e = Entry::CFlag("-O2 -Wall".to_string());
        let line = generate_export_line(&e, OperationMode::Replace);
        assert!(line.ends_with(';'), "line did not end with ';': {line}");
    }
}
