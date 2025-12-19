use crate::config::{CustomVarDef, Entry, EnvProfile, VarKind};
use crate::db;
use crate::export::{self, OperationMode};
use crate::profile_editor::{confirm_dialog, edit_profile_name_dialog};
use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Terminal,
};
use rusqlite::Connection;
use std::io::stdout;
//
// Application State and CRUD Operations
//

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveTab {
    EnvVars,
    Profiles,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditorStyle {
    Single,
    PathPart,
    PartsList,
}

#[derive(Clone)]
pub struct VarTypeOption {
    pub name: String,
    pub kind: VarKind,
    pub separator: String,
    pub editor: EditorStyle,
}

fn builtin_var_options() -> Vec<VarTypeOption> {
    vec![
        VarTypeOption {
            name: "PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PathPart,
        },
        // colon-separated lists
        VarTypeOption {
            name: "CPATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "C_INCLUDE_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "CPLUS_INCLUDE_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "OBJC_INCLUDE_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "LIBRARY_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "LD_LIBRARY_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "LD_RUN_PATH".to_string(),
            kind: VarKind::List,
            separator: ":".to_string(),
            editor: EditorStyle::PartsList,
        },
        // space-separated lists
        VarTypeOption {
            name: "CPPFLAGS".to_string(),
            kind: VarKind::List,
            separator: " ".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "CFLAGS".to_string(),
            kind: VarKind::List,
            separator: " ".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "CXXFLAGS".to_string(),
            kind: VarKind::List,
            separator: " ".to_string(),
            editor: EditorStyle::PartsList,
        },
        VarTypeOption {
            name: "LDFLAGS".to_string(),
            kind: VarKind::List,
            separator: " ".to_string(),
            editor: EditorStyle::PartsList,
        },
        // scalars
        VarTypeOption {
            name: "RANLIB".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "CC".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "CXX".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "AR".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "STRIP".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "GCC_EXEC_PREFIX".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "COLLECT_GCC_OPTIONS".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
        VarTypeOption {
            name: "LANG".to_string(),
            kind: VarKind::Scalar,
            separator: "".to_string(),
            editor: EditorStyle::Single,
        },
    ]
}

pub struct AppState {
    pub(crate) active_tab: ActiveTab,
    pub conn: Connection,
    pub profiles: Vec<EnvProfile>,
    pub active_profile_index: usize,
    pub env_list_state: ListState,
    pub profile_list_state: ListState,
    pub custom_var_defs: Vec<CustomVarDef>,
    pub var_options: Vec<VarTypeOption>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let conn = db::establish_connection()?;
        let mut profiles = db::load_all_profiles(&conn)?;
        if profiles.is_empty() {
            let default = EnvProfile::new("default");
            db::save_profile(&conn, &default)?;
            profiles.push(default);
        }
        let mut env_list_state = ListState::default();
        env_list_state.select(Some(0));
        let mut profile_list_state = ListState::default();
        profile_list_state.select(Some(0));
        let mut app = AppState {
            active_tab: ActiveTab::EnvVars,
            conn,
            profiles,
            active_profile_index: 0,
            env_list_state,
            profile_list_state,
            custom_var_defs: Vec::new(),
            var_options: Vec::new(),
        };
        app.refresh_var_options()?;
        Ok(app)
    }

    pub fn refresh_var_options(&mut self) -> Result<()> {
        self.custom_var_defs = db::load_custom_var_defs(&self.conn)?;
        let mut opts = builtin_var_options();
        for d in &self.custom_var_defs {
            opts.push(VarTypeOption {
                name: d.name.clone(),
                kind: d.kind.clone(),
                separator: d.separator.clone(),
                editor: match d.kind {
                    VarKind::Scalar => EditorStyle::Single,
                    VarKind::List => EditorStyle::PartsList,
                },
            });
        }
        self.var_options = opts;
        Ok(())
    }

    // CRUD for environment variables (active profile)
    pub fn add_env_var(&mut self, entry: Entry) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        profile.entries.push(entry);
        db::save_profile(&self.conn, profile)?;
        Ok(())
    }

    pub fn delete_env_var(&mut self, index: usize) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index < profile.entries.len() {
            profile.entries.remove(index);
            db::save_profile(&self.conn, profile)?;
        }
        Ok(())
    }

    pub fn update_env_var(&mut self, index: usize, entry: Entry) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index < profile.entries.len() {
            profile.entries[index] = entry;
            db::save_profile(&self.conn, profile)?;
        }
        Ok(())
    }

    pub fn move_env_var_up(&mut self, index: usize) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index == 0 || index >= profile.entries.len() {
            return Ok(());
        }
        profile.entries.swap(index - 1, index);
        db::save_profile(&self.conn, profile)?;
        Ok(())
    }

    pub fn move_env_var_down(&mut self, index: usize) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if profile.entries.is_empty() || index >= profile.entries.len().saturating_sub(1) {
            return Ok(());
        }
        profile.entries.swap(index, index + 1);
        db::save_profile(&self.conn, profile)?;
        Ok(())
    }

    pub fn replace_var_parts(&mut self, var_name: &str, new_parts: Vec<Entry>) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];

        let mut idxs: Vec<usize> = profile
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if e.var_name().as_ref() == var_name {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        if idxs.is_empty() {
            // If the var did not exist yet, append parts at the end.
            profile.entries.extend(new_parts);
            db::save_profile(&self.conn, profile)?;
            return Ok(());
        }

        let insert_at = *idxs.iter().min().unwrap_or(&0);
        idxs.sort_unstable();
        for i in idxs.into_iter().rev() {
            profile.entries.remove(i);
        }
        for (offset, e) in new_parts.into_iter().enumerate() {
            profile.entries.insert(insert_at + offset, e);
        }

        db::save_profile(&self.conn, profile)?;
        Ok(())
    }

    // CRUD for profiles
    pub fn add_profile(&mut self, profile: EnvProfile) -> Result<()> {
        db::save_profile(&self.conn, &profile)?;
        self.profiles.push(profile);
        Ok(())
    }
    pub fn delete_profile(&mut self, index: usize) -> Result<()> {
        if self.profiles.len() <= 1 {
            // Keep at least one profile to avoid later panics from empty state.
            return Ok(());
        }
        if index < self.profiles.len() {
            let profile = self.profiles.remove(index);
            db::delete_profile(&self.conn, &profile.name)?;
            if self.active_profile_index >= self.profiles.len() {
                self.active_profile_index = self.profiles.len().saturating_sub(1);
            }
        }
        Ok(())
    }
    pub fn update_profile(&mut self, index: usize, new_name: String) -> Result<()> {
        if index < self.profiles.len() {
            let old_name = self.profiles[index].name.clone();
            db::rename_profile(&self.conn, &old_name, &new_name)?;
            self.profiles[index].name = new_name;
        }
        Ok(())
    }
}

//
// Utility: Centered Rectangle (no generic parameter)
//
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1]);
    horizontal[1]
}

fn create_custom_var_dialog<B: Backend>(
    terminal: &mut Terminal<B>,
) -> Result<Option<CustomVarDef>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Field {
        Name,
        Kind,
        Separator,
    }

    let mut name = String::new();
    let mut kind = VarKind::List;
    let mut separator = ":".to_string();
    let mut field = Field::Name;

    loop {
        terminal.draw(|f| {
            let area = centered_rect(70, 35, f.size());
            let title = "Create custom env var (Tab: next, t: toggle kind, Enter: save, Esc: cancel)";
            let block = Block::default().borders(Borders::ALL).title(title);

            let kind_s = match kind {
                VarKind::Scalar => "Scalar",
                VarKind::List => "List",
            };

            let name_prefix = if field == Field::Name { "> " } else { "  " };
            let kind_prefix = if field == Field::Kind { "> " } else { "  " };
            let sep_prefix = if field == Field::Separator { "> " } else { "  " };

            let sep_line = if kind == VarKind::List {
                format!("{sep_prefix}Separator: {separator}")
            } else {
                format!("{sep_prefix}Separator: (n/a)")
            };

            let text = format!(
                "{name_prefix}Name: {name}\n{kind_prefix}Kind: {kind_s}\n{sep_line}\n\nNote: list vars are edited as parts; export joins parts using Separator."
            );
            let p = Paragraph::new(text).block(block);
            f.render_widget(p, area);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            return Ok(None);
                        }
                        let def = CustomVarDef {
                            name: trimmed.to_string(),
                            kind: kind.clone(),
                            separator: if kind == VarKind::List {
                                separator.clone()
                            } else {
                                String::new()
                            },
                        };
                        return Ok(Some(def));
                    }
                    KeyCode::Tab => {
                        field = match field {
                            Field::Name => Field::Kind,
                            Field::Kind => Field::Separator,
                            Field::Separator => Field::Name,
                        };
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        kind = match kind {
                            VarKind::Scalar => VarKind::List,
                            VarKind::List => VarKind::Scalar,
                        };
                        if kind == VarKind::List && separator.is_empty() {
                            separator = ":".to_string();
                        }
                    }
                    KeyCode::Backspace => match field {
                        Field::Name => {
                            name.pop();
                        }
                        Field::Kind => {}
                        Field::Separator => {
                            if kind == VarKind::List {
                                separator.pop();
                            }
                        }
                    },
                    KeyCode::Char(c) => match field {
                        Field::Name => name.push(c),
                        Field::Kind => {}
                        Field::Separator => {
                            if kind == VarKind::List {
                                separator.push(c);
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

//
// Main TUI: Two Tabs ("Env Vars" and "Profiles")
// In the "Env Vars" tab, the upper pane shows the list of entries,
// and the lower pane shows a full export preview (using Prepend mode).
//
pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::new()?;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            // Split into tabs and main content.
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);
            let tabs_titles = ["Env Vars", "Profiles"]
                .iter()
                .map(|t| Spans::from(Span::raw(*t)))
                .collect::<Vec<_>>();
            let tabs = Tabs::new(tabs_titles)
                .select(match app.active_tab {
                    ActiveTab::EnvVars => 0,
                    ActiveTab::Profiles => 1,
                })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tabs (←/→ to switch, q to quit)"),
                )
                .highlight_style(Style::default().fg(Color::Yellow))
                .divider(Span::raw("|"));
            f.render_widget(tabs, chunks[0]);

            match app.active_tab {
                ActiveTab::EnvVars => draw_env_vars_ui(f, chunks[1], &mut app),
                ActiveTab::Profiles => draw_profiles_ui(f, chunks[1], &mut app),
            }
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Left | KeyCode::Right => {
                        app.active_tab = if app.active_tab == ActiveTab::EnvVars {
                            ActiveTab::Profiles
                        } else {
                            ActiveTab::EnvVars
                        };
                    }
                    // Env Vars tab key bindings.
                    KeyCode::Char('a') if app.active_tab == ActiveTab::EnvVars => {
                        // Launch the edit variable widget.
                        if let Some(new_entry) =
                            editor::edit_env_var_dialog(&mut terminal, &app.var_options, None)?
                        {
                            app.add_env_var(new_entry)?;
                        }
                    }
                    KeyCode::Char('d') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(i) = app.env_list_state.selected() {
                            app.delete_env_var(i)?;
                        }
                    }
                    KeyCode::Char('e') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(i) = app.env_list_state.selected() {
                            let initial = app.profiles[app.active_profile_index]
                                .entries
                                .get(i)
                                .cloned();
                            if let Some(new_entry) = editor::edit_env_var_dialog(
                                &mut terminal,
                                &app.var_options,
                                initial.as_ref(),
                            )? {
                                app.update_env_var(i, new_entry)?;
                            }
                        }
                    }
                    KeyCode::Char('p') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(i) = app.env_list_state.selected() {
                            let entry = app.profiles[app.active_profile_index]
                                .entries
                                .get(i)
                                .cloned();
                            if let Some(entry) = entry {
                                let var_name = entry.var_name().into_owned();
                                let var_opt = app
                                    .var_options
                                    .iter()
                                    .find(|o| o.name == var_name)
                                    .cloned()
                                    .unwrap_or_else(|| VarTypeOption {
                                        name: var_name.clone(),
                                        kind: match entry {
                                            Entry::CustomScalar { .. } => VarKind::Scalar,
                                            _ => VarKind::List,
                                        },
                                        separator: match entry {
                                            Entry::CustomPart { separator, .. } => separator,
                                            _ => entry.separator().into_owned(),
                                        },
                                        editor: EditorStyle::PartsList,
                                    });

                                if var_opt.kind == VarKind::List {
                                    let parts: Vec<Entry> = app.profiles[app.active_profile_index]
                                        .entries
                                        .iter()
                                        .filter(|e| e.var_name().as_ref() == var_opt.name)
                                        .cloned()
                                        .collect();

                                    if let Some(new_parts) = editor::edit_var_parts_dialog(
                                        &mut terminal,
                                        &var_opt,
                                        &parts,
                                    )? {
                                        app.replace_var_parts(&var_opt.name, new_parts)?;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('C') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(def) = create_custom_var_dialog(&mut terminal)? {
                            db::save_custom_var_def(&app.conn, &def)?;
                            app.refresh_var_options()?;
                        }
                    }
                    KeyCode::Char('K') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(i) = app.env_list_state.selected() {
                            app.move_env_var_up(i)?;
                            let new_i = i.saturating_sub(1);
                            app.env_list_state.select(Some(new_i));
                        }
                    }
                    KeyCode::Char('J') if app.active_tab == ActiveTab::EnvVars => {
                        if let Some(i) = app.env_list_state.selected() {
                            app.move_env_var_down(i)?;
                            let profile = &app.profiles[app.active_profile_index];
                            let new_i = (i + 1).min(profile.entries.len().saturating_sub(1));
                            app.env_list_state.select(Some(new_i));
                        }
                    }
                    KeyCode::Down if app.active_tab == ActiveTab::EnvVars => {
                        let profile = &app.profiles[app.active_profile_index];
                        let i = match app.env_list_state.selected() {
                            Some(i) if i >= profile.entries.len().saturating_sub(1) => 0,
                            Some(i) => i + 1,
                            None => 0,
                        };
                        app.env_list_state.select(Some(i));
                    }
                    KeyCode::Up if app.active_tab == ActiveTab::EnvVars => {
                        let profile = &app.profiles[app.active_profile_index];
                        let i = match app.env_list_state.selected() {
                            Some(0) | None => profile.entries.len().saturating_sub(1),
                            Some(i) => i - 1,
                        };
                        app.env_list_state.select(Some(i));
                    }
                    // Profiles tab key bindings.
                    KeyCode::Char('A') if app.active_tab == ActiveTab::Profiles => {
                        if let Some(new_name) = edit_profile_name_dialog(&mut terminal, None)? {
                            let new_profile = EnvProfile::new(&new_name);
                            app.add_profile(new_profile)?;
                        }
                    }
                    KeyCode::Char('D') if app.active_tab == ActiveTab::Profiles => {
                        if let Some(i) = app.profile_list_state.selected() {
                            if confirm_dialog(&mut terminal, "Delete profile?")? {
                                app.delete_profile(i)?;
                            }
                        }
                    }
                    KeyCode::Char('E') if app.active_tab == ActiveTab::Profiles => {
                        if let Some(i) = app.profile_list_state.selected() {
                            let current_name = app.profiles[i].name.clone();
                            if let Some(new_name) =
                                edit_profile_name_dialog(&mut terminal, Some(&current_name))?
                            {
                                app.update_profile(i, new_name)?;
                            }
                        }
                    }
                    KeyCode::Down if app.active_tab == ActiveTab::Profiles => {
                        let i = match app.profile_list_state.selected() {
                            Some(i) if i >= app.profiles.len().saturating_sub(1) => 0,
                            Some(i) => i + 1,
                            None => 0,
                        };
                        app.profile_list_state.select(Some(i));
                    }
                    KeyCode::Up if app.active_tab == ActiveTab::Profiles => {
                        let i = match app.profile_list_state.selected() {
                            Some(0) | None => app.profiles.len().saturating_sub(1),
                            Some(i) => i - 1,
                        };
                        app.profile_list_state.select(Some(i));
                    }
                    // Handle Enter key to select the profile.
                    KeyCode::Enter if app.active_tab == ActiveTab::Profiles => {
                        if let Some(i) = app.profile_list_state.selected() {
                            app.active_profile_index = i; // Update the active profile
                            terminal.draw(|f| {
                                // Force a UI refresh
                                let size = f.size();
                                let chunks = Layout::default()
                                    .direction(Direction::Vertical)
                                    .constraints(
                                        [Constraint::Length(3), Constraint::Min(0)].as_ref(),
                                    )
                                    .split(size);
                                match app.active_tab {
                                    ActiveTab::EnvVars => draw_env_vars_ui(f, chunks[1], &mut app),
                                    ActiveTab::Profiles => draw_profiles_ui(f, chunks[1], &mut app),
                                }
                            })?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Draws the Env Vars UI with two vertical panes:
///   - Upper pane: list of entries for the active profile.
///   - Lower pane: full export preview (using Prepend mode).
fn draw_env_vars_ui<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(area);

    let active_profile = &app.profiles[app.active_profile_index];
    let items: Vec<ListItem> = active_profile
        .entries
        .iter()
        .map(|entry| ListItem::new(entry.to_string()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Env Vars for profile: {} (a:add, e:edit, p:parts, d:delete, J/K:move entry, C:custom var def)",
            active_profile.name
        )))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, chunks[0], &mut app.env_list_state);

    let full_export = export::generate_full_export(active_profile, OperationMode::Prepend);
    let preview = Paragraph::new(full_export).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Full Export Preview (for eval)"),
    );
    f.render_widget(preview, chunks[1]);
}

/// Draws the Profiles UI.
fn draw_profiles_ui<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let items: Vec<ListItem> = app
        .profiles
        .iter()
        .enumerate() // Add enumeration to track the index
        .map(|(i, p)| {
            // Highlight the active profile with a different style
            let style = if i == app.active_profile_index {
                Style::default()
                    .fg(Color::Yellow) // Highlight color
                    .add_modifier(Modifier::BOLD) // Make it bold
            } else {
                Style::default()
            };

            // Add a visual cue (e.g., "*") for the active profile
            let display_name = if i == app.active_profile_index {
                format!("* {}", p.name) // Add a star for the active profile
            } else {
                format!("  {}", p.name) // No star for other profiles
            };

            ListItem::new(display_name).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Profiles (A: add, D: delete, E: edit selected, Enter: select)"),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, area, &mut app.profile_list_state);
}
//
// Editor Module: Provides the Edit/Create Env Var Widget with Integrated Export Preview.
//
mod editor {
    use crate::config::{Entry, PathEntry, VarKind};
    use crate::export::{self, OperationMode};
    use anyhow::Result;
    use crossterm::event;
    use crossterm::event::{Event, KeyCode};
    use ratatui::{
        backend::Backend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
        Terminal,
    };

    fn is_path_part(opt: &super::VarTypeOption) -> bool {
        opt.editor == super::EditorStyle::PathPart
    }

    pub fn edit_var_parts_dialog<B: Backend>(
        terminal: &mut Terminal<B>,
        var: &super::VarTypeOption,
        initial_parts: &[Entry],
    ) -> Result<Option<Vec<Entry>>> {
        let mut parts: Vec<Entry> = initial_parts.to_vec();
        let mut selected: usize = 0;

        loop {
            terminal.draw(|f| {
                let size = f.size();
                let area = super::centered_rect(85, 70, size);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(2)].as_ref())
                    .split(area);

                let items: Vec<ListItem> = if parts.is_empty() {
                    vec![ListItem::new("(no parts)")]
                } else {
                    parts.iter().map(|e| ListItem::new(e.to_string())).collect()
                };
                let mut list_state = ListState::default();
                if !parts.is_empty() {
                    list_state.select(Some(selected.min(parts.len().saturating_sub(1))));
                }

                let title = format!(
                    "{} parts (a:add, e:edit, d:delete, J/K:move, Enter:save, Esc:cancel)",
                    var.name
                );
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(title))
                    .highlight_style(Style::default().bg(Color::Blue));
                f.render_stateful_widget(list, chunks[0], &mut list_state);

                let hint = Paragraph::new(format!(
                    "Export preview uses separator '{}' and produces one export line.",
                    var.separator
                ))
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(hint, chunks[1]);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Enter => return Ok(Some(parts)),
                        KeyCode::Up => {
                            selected = selected.saturating_sub(1);
                        }
                        KeyCode::Down => {
                            if selected + 1 < parts.len() {
                                selected += 1;
                            }
                        }
                        KeyCode::Char('K') => {
                            if selected > 0 && selected < parts.len() {
                                parts.swap(selected - 1, selected);
                                selected = selected.saturating_sub(1);
                            }
                        }
                        KeyCode::Char('J') => {
                            if selected + 1 < parts.len() {
                                parts.swap(selected, selected + 1);
                                selected += 1;
                            }
                        }
                        KeyCode::Char('d') => {
                            if selected < parts.len() {
                                parts.remove(selected);
                                if selected >= parts.len() && selected > 0 {
                                    selected = selected.saturating_sub(1);
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            let one = vec![var.clone()];
                            if let Some(new_entry) = edit_env_var_dialog(terminal, &one, None)? {
                                parts.push(new_entry);
                                selected = parts.len().saturating_sub(1);
                            }
                        }
                        KeyCode::Char('e') => {
                            if selected < parts.len() {
                                let one = vec![var.clone()];
                                let current = parts.get(selected);
                                if let Some(new_entry) =
                                    edit_env_var_dialog(terminal, &one, current)?
                                {
                                    parts[selected] = new_entry;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    #[derive(PartialEq, Debug, Clone, Copy, Default)]
    pub enum FocusArea {
        #[default]
        Search,
        Options,
        Input,
    }

    #[derive(Default)]
    pub struct EnvVarEditorState {
        pub all_options: Vec<super::VarTypeOption>,
        pub search: String,
        pub filtered: Vec<super::VarTypeOption>,
        pub selected: usize,
        pub input: String,
        pub path: String,
        pub version: String,
        pub tool: String,
        pub active_input_field: usize,
        pub focus: FocusArea,
        last_search: String,
    }

    impl EnvVarEditorState {
        pub fn new(options: &[super::VarTypeOption], initial: Option<&Entry>) -> Self {
            let mut s = Self {
                all_options: options.to_vec(),
                search: String::new(),
                filtered: options.to_vec(),
                selected: 0,
                input: String::new(),
                path: String::new(),
                version: String::new(),
                tool: String::new(),
                active_input_field: 0,
                focus: FocusArea::Search,
                last_search: String::new(),
            };

            if let Some(e) = initial {
                let initial_name = e.var_name().into_owned();
                if let Some(pos) = s.all_options.iter().position(|o| o.name == initial_name) {
                    s.selected = pos;
                }
                match e {
                    Entry::Path(pe) => {
                        s.path = pe.path.clone();
                        s.version = pe.version.clone();
                        s.tool = pe.program.clone();
                        s.focus = FocusArea::Input;
                    }
                    Entry::CPath(v)
                    | Entry::CInclude(v)
                    | Entry::CPlusInclude(v)
                    | Entry::OBJCInclude(v)
                    | Entry::CPPFlag(v)
                    | Entry::CFlag(v)
                    | Entry::CXXFlag(v)
                    | Entry::LDFlag(v)
                    | Entry::LibraryPath(v)
                    | Entry::LDLibraryPath(v)
                    | Entry::LDRunPath(v)
                    | Entry::RanLib(v)
                    | Entry::CC(v)
                    | Entry::CXX(v)
                    | Entry::AR(v)
                    | Entry::Strip(v)
                    | Entry::GCCExecPrefix(v)
                    | Entry::CollectGCCOptions(v)
                    | Entry::Lang(v) => {
                        s.input = v.clone();
                        s.focus = FocusArea::Input;
                    }
                    Entry::CustomScalar { value, .. } | Entry::CustomPart { value, .. } => {
                        s.input = value.clone();
                        s.focus = FocusArea::Input;
                    }
                }
            }

            s
        }

        pub fn update_filter(&mut self) {
            if self.search == self.last_search {
                return;
            }
            self.last_search = self.search.clone();
            self.filtered = self
                .all_options
                .iter()
                .filter(|opt| {
                    opt.name
                        .to_lowercase()
                        .contains(&self.search.to_lowercase())
                })
                .cloned()
                .collect();
            if self.filtered.is_empty() {
                self.filtered = self.all_options.clone();
            }
            self.selected = 0;
        }
    }

    fn entry_from_state(opt: &super::VarTypeOption, state: &EnvVarEditorState) -> Entry {
        if is_path_part(opt) {
            return Entry::Path(PathEntry {
                path: state.path.clone(),
                version: state.version.clone(),
                program: state.tool.clone(),
            });
        }

        // Builtins
        match opt.name.as_str() {
            "CPATH" => return Entry::CPath(state.input.clone()),
            "C_INCLUDE_PATH" => return Entry::CInclude(state.input.clone()),
            "CPLUS_INCLUDE_PATH" => return Entry::CPlusInclude(state.input.clone()),
            "OBJC_INCLUDE_PATH" => return Entry::OBJCInclude(state.input.clone()),
            "CPPFLAGS" => return Entry::CPPFlag(state.input.clone()),
            "CFLAGS" => return Entry::CFlag(state.input.clone()),
            "CXXFLAGS" => return Entry::CXXFlag(state.input.clone()),
            "LDFLAGS" => return Entry::LDFlag(state.input.clone()),
            "LIBRARY_PATH" => return Entry::LibraryPath(state.input.clone()),
            "LD_LIBRARY_PATH" => return Entry::LDLibraryPath(state.input.clone()),
            "LD_RUN_PATH" => return Entry::LDRunPath(state.input.clone()),
            "RANLIB" => return Entry::RanLib(state.input.clone()),
            "CC" => return Entry::CC(state.input.clone()),
            "CXX" => return Entry::CXX(state.input.clone()),
            "AR" => return Entry::AR(state.input.clone()),
            "STRIP" => return Entry::Strip(state.input.clone()),
            "GCC_EXEC_PREFIX" => return Entry::GCCExecPrefix(state.input.clone()),
            "COLLECT_GCC_OPTIONS" => return Entry::CollectGCCOptions(state.input.clone()),
            "LANG" => return Entry::Lang(state.input.clone()),
            _ => {}
        }

        // Custom vars
        match opt.kind {
            VarKind::Scalar => Entry::CustomScalar {
                name: opt.name.clone(),
                value: state.input.clone(),
            },
            VarKind::List => Entry::CustomPart {
                name: opt.name.clone(),
                value: state.input.clone(),
                separator: opt.separator.clone(),
            },
        }
    }

    /// Launches the edit/create env var widget.
    /// Displays fuzzy search on the left and input fields on the right,
    /// with an integrated preview (using default Prepend mode) of the export command for the current variable.
    pub fn edit_env_var_dialog<B: Backend>(
        terminal: &mut Terminal<B>,
        options: &[super::VarTypeOption],
        initial: Option<&Entry>,
    ) -> Result<Option<Entry>> {
        let mut state = EnvVarEditorState::new(options, initial);

        loop {
            terminal.draw(|f| {
                let size = f.size();
                // Split into left (40%) for fuzzy search and right (60%) for input and preview.
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                    .split(super::centered_rect(80, 60, size));

                // Left pane: fuzzy search and options.
                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                    .split(chunks[0]);

                let search_style = if state.focus == FocusArea::Search {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                let search_para = Paragraph::new(state.search.as_ref()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(search_style)
                        .title("Search Type"),
                );
                f.render_widget(search_para, left_chunks[0]);

                state.update_filter();
                let items: Vec<ListItem> = state
                    .filtered
                    .iter()
                    .map(|opt| ListItem::new(opt.name.clone()))
                    .collect();
                let mut list_state = ListState::default();
                list_state.select(Some(state.selected));
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Options"))
                    .highlight_style(Style::default().bg(Color::Blue));
                f.render_stateful_widget(list, left_chunks[1], &mut list_state);

                // Right pane: input fields and preview.
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(7), Constraint::Min(0)].as_ref())
                    .split(chunks[1]);

                let input_style = if state.focus == FocusArea::Input {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                if state
                    .filtered
                    .get(state.selected)
                    .map(is_path_part)
                    .unwrap_or(false)
                {
                    // Multi-field input for types like PATH.
                    let field_titles = ["Path", "Version", "Tool Name"];
                    let values = [
                        state.path.clone(),
                        state.version.clone(),
                        state.tool.clone(),
                    ];
                    let field_items: Vec<ListItem> = field_titles
                        .iter()
                        .enumerate()
                        .map(|(i, title)| {
                            let indicator = if state.focus == FocusArea::Input
                                && state.active_input_field == i
                            {
                                "> "
                            } else {
                                "  "
                            };
                            ListItem::new(format!("{}{}: {}", indicator, title, values[i]))
                        })
                        .collect();
                    let fields_list = List::new(field_items).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(input_style)
                            .title("Multi-field Input"),
                    );
                    f.render_widget(fields_list, right_chunks[0]);
                } else {
                    // Single-field input.
                    let current_type = state
                        .filtered
                        .get(state.selected)
                        .map(|opt| opt.name.clone())
                        .unwrap_or_else(|| "...".to_string());
                    let title = format!("Enter value for {}", current_type);
                    let para = Paragraph::new(state.input.as_ref()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(input_style)
                            .title(title),
                    );
                    f.render_widget(para, right_chunks[0]);
                }

                // Bottom right: preview of export command for the current variable.
                let preview = if state
                    .filtered
                    .get(state.selected)
                    .map(is_path_part)
                    .unwrap_or(false)
                {
                    let pe = PathEntry {
                        path: state.path.clone(),
                        version: state.version.clone(),
                        program: state.tool.clone(),
                    };
                    let entry = Entry::Path(pe);
                    export::generate_export_line(&entry, OperationMode::Prepend)
                } else {
                    let opt = state
                        .filtered
                        .get(state.selected)
                        .cloned()
                        .unwrap_or_else(|| super::VarTypeOption {
                            name: "CFLAGS".to_string(),
                            kind: VarKind::List,
                            separator: " ".to_string(),
                            editor: super::EditorStyle::Single,
                        });
                    let entry = entry_from_state(&opt, &state);
                    export::generate_export_line(&entry, OperationMode::Prepend)
                };
                let preview_para = Paragraph::new(preview).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Export Preview for this Variable"),
                );
                f.render_widget(preview_para, right_chunks[1]);
            })?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Enter => {
                            let selected_opt = &state.filtered[state.selected];
                            let entry = entry_from_state(selected_opt, &state);
                            return Ok(Some(entry));
                        }
                        KeyCode::Tab => {
                            // Cycle focus among Search -> Options -> Input.
                            state.focus = match state.focus {
                                FocusArea::Search => FocusArea::Options,
                                FocusArea::Options => FocusArea::Input,
                                FocusArea::Input => FocusArea::Search,
                            };
                        }
                        KeyCode::Backspace => match state.focus {
                            FocusArea::Search => {
                                state.search.pop();
                                state.update_filter();
                            }
                            FocusArea::Options => {}
                            FocusArea::Input => {
                                if state
                                    .filtered
                                    .get(state.selected)
                                    .map(is_path_part)
                                    .unwrap_or(false)
                                {
                                    match state.active_input_field {
                                        0 => {
                                            state.path.pop();
                                        }
                                        1 => {
                                            state.version.pop();
                                        }
                                        2 => {
                                            state.tool.pop();
                                        }
                                        _ => {}
                                    }
                                } else {
                                    state.input.pop();
                                }
                            }
                        },
                        KeyCode::Char(c) => match state.focus {
                            FocusArea::Search => {
                                state.search.push(c);
                                state.update_filter();
                            }
                            FocusArea::Options => {}
                            FocusArea::Input => {
                                if state
                                    .filtered
                                    .get(state.selected)
                                    .map(is_path_part)
                                    .unwrap_or(false)
                                {
                                    match state.active_input_field {
                                        0 => state.path.push(c),
                                        1 => state.version.push(c),
                                        2 => state.tool.push(c),
                                        _ => {}
                                    }
                                } else {
                                    state.input.push(c);
                                }
                            }
                        },
                        KeyCode::Up => match state.focus {
                            FocusArea::Options => {
                                if state.selected > 0 {
                                    state.selected -= 1;
                                }
                            }
                            FocusArea::Input => {
                                if state
                                    .filtered
                                    .get(state.selected)
                                    .map(is_path_part)
                                    .unwrap_or(false)
                                    && state.active_input_field > 0
                                {
                                    state.active_input_field -= 1;
                                }
                            }
                            _ => {}
                        },
                        KeyCode::Down => match state.focus {
                            FocusArea::Options => {
                                if state.selected < state.filtered.len().saturating_sub(1) {
                                    state.selected += 1;
                                }
                            }
                            FocusArea::Input => {
                                if state
                                    .filtered
                                    .get(state.selected)
                                    .map(is_path_part)
                                    .unwrap_or(false)
                                    && state.active_input_field < 2
                                {
                                    state.active_input_field += 1;
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn update_filter_does_not_reset_selected_when_search_is_unchanged() {
            let options = super::super::builtin_var_options();
            let mut s = EnvVarEditorState::new(&options, None);

            // Force one filter refresh.
            s.search.push('c');
            s.update_filter();
            assert!(
                s.filtered.len() > 2,
                "expected enough options for selection test"
            );

            // Simulate user moving selection in the options list.
            s.selected = 2;

            // Subsequent draws call update_filter again; selection must not be reset.
            s.update_filter();
            assert_eq!(s.selected, 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;

    #[test]
    fn delete_profile_does_not_remove_last_profile() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        db::initialize_db(&conn)?;

        let default = EnvProfile::new("default");
        db::save_profile(&conn, &default)?;

        let mut app = AppState {
            active_tab: ActiveTab::EnvVars,
            conn,
            profiles: vec![default],
            active_profile_index: 0,
            env_list_state: ListState::default(),
            profile_list_state: ListState::default(),
            custom_var_defs: Vec::new(),
            var_options: builtin_var_options(),
        };

        app.delete_profile(0)?;

        assert_eq!(app.profiles.len(), 1);
        assert_eq!(app.profiles[0].name, "default");
        assert_eq!(app.active_profile_index, 0);

        let count: i64 = app
            .conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |row| row.get(0))?;
        assert_eq!(count, 1);

        Ok(())
    }
}
