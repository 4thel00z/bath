// src/export.rs

use crate::config::{Entry, EnvProfile};
use crate::db;
use anyhow::Result;

#[derive(Clone, Copy)]
pub enum OperationMode {
    Prepend,
    Append,
    Replace,
}

fn shell_single_quote_literal(s: &str) -> String {
    // POSIX-shell safe single-quote escaping:
    // wrap in single quotes, and represent any internal ' as: '"'"'
    // Example: foo'bar => 'foo'"'"'bar'
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Generates an export command for a single Entry.
pub fn generate_export_line(entry: &Entry, mode: OperationMode) -> String {
    let var_name = entry.var_name();
    let value = match entry {
        Entry::Path(ref pe) => pe.path.clone(),
        Entry::CPath(ref s)
        | Entry::CInclude(ref s)
        | Entry::CPlusInclude(ref s)
        | Entry::OBJCInclude(ref s)
        | Entry::CPPFlag(ref s)
        | Entry::CFlag(ref s)
        | Entry::CXXFlag(ref s)
        | Entry::LDFlag(ref s)
        | Entry::LibraryPath(ref s)
        | Entry::LDLibraryPath(ref s)
        | Entry::LDRunPath(ref s)
        | Entry::RanLib(ref s)
        | Entry::CC(ref s)
        | Entry::CXX(ref s)
        | Entry::AR(ref s)
        | Entry::Strip(ref s)
        | Entry::GCCExecPrefix(ref s)
        | Entry::CollectGCCOptions(ref s)
        | Entry::Lang(ref s) => s.clone(),
    };
    let sep = entry.separator();

    let var_ref = format!("${{{}}}", var_name);
    match mode {
        // Use safe shell quoting for user-provided values while keeping the variable reference expanded.
        // Also end each statement with ';' so `eval $(bath export ...)` works even if newlines collapse to spaces.
        OperationMode::Prepend => {
            let prefix = format!("{}{}", value, sep);
            format!(
                "export {}={}\"{}\";",
                var_name,
                shell_single_quote_literal(&prefix),
                var_ref
            )
        }
        OperationMode::Append => {
            let suffix = format!("{}{}", sep, value);
            format!(
                "export {}=\"{}\"{};",
                var_name,
                var_ref,
                shell_single_quote_literal(&suffix)
            )
        }
        OperationMode::Replace => format!(
            "export {}={};",
            var_name,
            shell_single_quote_literal(&value)
        ),
    }
}

/// Generates the full export commands for a given profile.
pub fn generate_full_export(profile: &EnvProfile, mode: OperationMode) -> String {
    profile
        .entries
        .iter()
        .map(|entry| generate_export_line(entry, mode))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Exports the given profile as export commands (without a shebang)
/// so you can eval the commands in your shell.
pub fn export_profile(profile_name: &str, mode: OperationMode) -> Result<()> {
    let conn = db::establish_connection()?;
    let profile: EnvProfile = db::load_profile(&conn, profile_name)?;

    for entry in profile.entries {
        let export_command = generate_export_line(&entry, mode);
        println!("{}", export_command);
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
    fn replace_mode_single_quotes_and_escapes_inner_single_quotes() {
        let e = Entry::CC("O'Reilly".to_string());
        assert_eq!(
            generate_export_line(&e, OperationMode::Replace),
            "export CC='O'\"'\"'Reilly';"
        );
    }

    #[test]
    fn prepend_mode_keeps_var_expansion_and_quotes_literal_prefix() {
        let e = Entry::Path(PathEntry {
            path: "/opt/$HOME/bin".to_string(),
            program: "tool".to_string(),
            version: "1".to_string(),
        });
        assert_eq!(
            generate_export_line(&e, OperationMode::Prepend),
            "export PATH='/opt/$HOME/bin:'\"${PATH}\";"
        );
    }

    #[test]
    fn append_mode_keeps_var_expansion_and_quotes_literal_suffix() {
        let e = Entry::Path(PathEntry {
            path: "/opt/bin".to_string(),
            program: "tool".to_string(),
            version: "1".to_string(),
        });
        assert_eq!(
            generate_export_line(&e, OperationMode::Append),
            "export PATH=\"${PATH}\"':/opt/bin';"
        );
    }

    #[test]
    fn statements_end_with_semicolon() {
        let e = Entry::CFlag("-O2 -Wall".to_string());
        let line = generate_export_line(&e, OperationMode::Replace);
        assert!(line.ends_with(';'), "line did not end with ';': {line}");
    }
}
