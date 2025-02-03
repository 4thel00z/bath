// src/export.rs

use anyhow::Result;
use crate::db;
use crate::config::{EnvProfile, Entry, PathEntry};
use std::io::{self, Write};

#[derive(Clone, Copy)]
pub enum OperationMode {
    Prepend,
    Append,
    Replace,
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
    match mode {
        OperationMode::Prepend => format!("export {}=\"{}{}${}\"", var_name, value, sep, var_name),
        OperationMode::Append  => format!("export {}=\"${}{}{}\"", var_name, var_name, sep, value),
        OperationMode::Replace => format!("export {}=\"{}\"", var_name, value),
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
    use crossterm::event::{read, poll, Event, KeyCode};
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
    use crossterm::execute;
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
    use ratatui::style::Style;
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
