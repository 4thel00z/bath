use crate::config::{Entry, EnvProfile};
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

pub struct AppState {
    pub(crate) active_tab: ActiveTab,
    pub conn: Connection,
    pub profiles: Vec<EnvProfile>,
    pub active_profile_index: usize,
    pub env_list_state: ListState,
    pub profile_list_state: ListState,
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
        Ok(AppState {
            active_tab: ActiveTab::EnvVars,
            conn,
            profiles,
            active_profile_index: 0,
            env_list_state,
            profile_list_state,
        })
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
                        if let Some(new_entry) = editor::edit_env_var_dialog(&mut terminal)? {
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
                            if let Some(new_entry) = editor::edit_env_var_dialog(&mut terminal)? {
                                app.update_env_var(i, new_entry)?;
                            }
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
            "Env Vars for profile: {} (a: edit/add, d: delete, e: edit selected)",
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
    use crate::config::{Entry, PathEntry};
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

    #[derive(Clone)]
    pub struct VarTypeOption {
        pub name: &'static str,
        pub is_multi: bool,
    }

    pub const VAR_OPTIONS: &[VarTypeOption] = &[
        VarTypeOption {
            name: "PATH",
            is_multi: true,
        },
        VarTypeOption {
            name: "CPATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "C_INCLUDE_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "CPLUS_INCLUDE_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "OBJC_INCLUDE_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "CPPFLAGS",
            is_multi: false,
        },
        VarTypeOption {
            name: "CFLAGS",
            is_multi: false,
        },
        VarTypeOption {
            name: "CXXFLAGS",
            is_multi: false,
        },
        VarTypeOption {
            name: "LDFLAGS",
            is_multi: false,
        },
        VarTypeOption {
            name: "LIBRARY_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "LD_LIBRARY_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "LD_RUN_PATH",
            is_multi: false,
        },
        VarTypeOption {
            name: "RANLIB",
            is_multi: false,
        },
        VarTypeOption {
            name: "CC",
            is_multi: false,
        },
        VarTypeOption {
            name: "CXX",
            is_multi: false,
        },
        VarTypeOption {
            name: "AR",
            is_multi: false,
        },
        VarTypeOption {
            name: "STRIP",
            is_multi: false,
        },
        VarTypeOption {
            name: "GCC_EXEC_PREFIX",
            is_multi: false,
        },
        VarTypeOption {
            name: "COLLECT_GCC_OPTIONS",
            is_multi: false,
        },
        VarTypeOption {
            name: "LANG",
            is_multi: false,
        },
    ];

    #[derive(PartialEq, Debug, Clone, Copy)]
    pub enum FocusArea {
        Search,
        Options,
        Input,
    }

    impl Default for FocusArea {
        fn default() -> Self {
            FocusArea::Search
        }
    }

    #[derive(Default)]
    pub struct EnvVarEditorState {
        pub search: String,
        pub filtered: Vec<VarTypeOption>,
        pub selected: usize,
        pub input: String,
        pub path: String,
        pub version: String,
        pub tool: String,
        pub active_input_field: usize,
        pub focus: FocusArea,
    }

    impl EnvVarEditorState {
        pub fn new() -> Self {
            Self {
                search: String::new(),
                filtered: VAR_OPTIONS.to_vec(),
                selected: 0,
                input: String::new(),
                path: String::new(),
                version: String::new(),
                tool: String::new(),
                active_input_field: 0,
                focus: FocusArea::Search,
            }
        }

        pub fn update_filter(&mut self) {
            self.filtered = VAR_OPTIONS
                .iter()
                .filter(|opt| {
                    opt.name
                        .to_lowercase()
                        .contains(&self.search.to_lowercase())
                })
                .cloned()
                .collect();
            if self.filtered.is_empty() {
                self.filtered = VAR_OPTIONS.to_vec();
            }
            self.selected = 0;
        }
    }

    /// Launches the edit/create env var widget.
    /// Displays fuzzy search on the left and input fields on the right,
    /// with an integrated preview (using default Prepend mode) of the export command for the current variable.
    pub fn edit_env_var_dialog<B: Backend>(terminal: &mut Terminal<B>) -> Result<Option<Entry>> {
        let mut state = EnvVarEditorState::new();

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
                    .map(|opt| ListItem::new(opt.name))
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
                    .map(|o| o.is_multi)
                    .unwrap_or(false)
                {
                    // Multi-field input for types like PATH.
                    let field_titles = vec!["Path", "Version", "Tool Name"];
                    let values = vec![
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
                        .map(|opt| opt.name)
                        .unwrap_or("...");
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
                    .map(|o| o.is_multi)
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
                    let entry = match state.filtered.get(state.selected).map(|o| o.name) {
                        Some("CPATH") => Entry::CPath(state.input.clone()),
                        Some("C_INCLUDE_PATH") => Entry::CInclude(state.input.clone()),
                        Some("CPLUS_INCLUDE_PATH") => Entry::CPlusInclude(state.input.clone()),
                        Some("OBJC_INCLUDE_PATH") => Entry::OBJCInclude(state.input.clone()),
                        Some("CPPFLAGS") => Entry::CPPFlag(state.input.clone()),
                        Some("CFLAGS") => Entry::CFlag(state.input.clone()),
                        Some("CXXFLAGS") => Entry::CXXFlag(state.input.clone()),
                        Some("LDFLAGS") => Entry::LDFlag(state.input.clone()),
                        Some("LIBRARY_PATH") => Entry::LibraryPath(state.input.clone()),
                        Some("LD_LIBRARY_PATH") => Entry::LDLibraryPath(state.input.clone()),
                        Some("LD_RUN_PATH") => Entry::LDRunPath(state.input.clone()),
                        Some("RANLIB") => Entry::RanLib(state.input.clone()),
                        Some("CC") => Entry::CC(state.input.clone()),
                        Some("CXX") => Entry::CXX(state.input.clone()),
                        Some("AR") => Entry::AR(state.input.clone()),
                        Some("STRIP") => Entry::Strip(state.input.clone()),
                        Some("GCC_EXEC_PREFIX") => Entry::GCCExecPrefix(state.input.clone()),
                        Some("COLLECT_GCC_OPTIONS") => {
                            Entry::CollectGCCOptions(state.input.clone())
                        }
                        Some("LANG") => Entry::Lang(state.input.clone()),
                        _ => Entry::CFlag(state.input.clone()),
                    };
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
                            let entry = if selected_opt.is_multi {
                                Entry::Path(PathEntry {
                                    path: state.path.clone(),
                                    version: state.version.clone(),
                                    program: state.tool.clone(),
                                })
                            } else {
                                match selected_opt.name {
                                    "CPATH" => Entry::CPath(state.input.clone()),
                                    "C_INCLUDE_PATH" => Entry::CInclude(state.input.clone()),
                                    "CPLUS_INCLUDE_PATH" => {
                                        Entry::CPlusInclude(state.input.clone())
                                    }
                                    "OBJC_INCLUDE_PATH" => Entry::OBJCInclude(state.input.clone()),
                                    "CPPFLAGS" => Entry::CPPFlag(state.input.clone()),
                                    "CFLAGS" => Entry::CFlag(state.input.clone()),
                                    "CXXFLAGS" => Entry::CXXFlag(state.input.clone()),
                                    "LDFLAGS" => Entry::LDFlag(state.input.clone()),
                                    "LIBRARY_PATH" => Entry::LibraryPath(state.input.clone()),
                                    "LD_LIBRARY_PATH" => Entry::LDLibraryPath(state.input.clone()),
                                    "LD_RUN_PATH" => Entry::LDRunPath(state.input.clone()),
                                    "RANLIB" => Entry::RanLib(state.input.clone()),
                                    "CC" => Entry::CC(state.input.clone()),
                                    "CXX" => Entry::CXX(state.input.clone()),
                                    "AR" => Entry::AR(state.input.clone()),
                                    "STRIP" => Entry::Strip(state.input.clone()),
                                    "GCC_EXEC_PREFIX" => Entry::GCCExecPrefix(state.input.clone()),
                                    "COLLECT_GCC_OPTIONS" => {
                                        Entry::CollectGCCOptions(state.input.clone())
                                    }
                                    "LANG" => Entry::Lang(state.input.clone()),
                                    _ => Entry::CFlag(state.input.clone()),
                                }
                            };
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
                                    .map(|o| o.is_multi)
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
                                    .map(|o| o.is_multi)
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
                                    .map(|o| o.is_multi)
                                    .unwrap_or(false)
                                {
                                    if state.active_input_field > 0 {
                                        state.active_input_field -= 1;
                                    }
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
                                    .map(|o| o.is_multi)
                                    .unwrap_or(false)
                                {
                                    if state.active_input_field < 2 {
                                        state.active_input_field += 1;
                                    }
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
