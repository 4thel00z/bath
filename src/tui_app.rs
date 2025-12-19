use crate::config::{CatalogItem, CustomVarDef, Entry, EnvProfile, ItemKind, PathEntry, VarKind};
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
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Terminal,
};
use rusqlite::Connection;
use std::io::stdout;
//
// Application State and CRUD Operations
//

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Profiles,
    Vars,
    Defs,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Parts,
    Items,
    Preview,
    Export,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Command,
    Search,
}

#[derive(Clone)]
pub enum Holding {
    Item(CatalogItem),
    Part {
        var: String,
        from: usize,
        entry: Entry,
    },
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
    pub conn: Connection,
    pub profiles: Vec<EnvProfile>,
    pub active_profile_index: usize,
    pub profile_list_state: ListState,
    pub custom_var_defs: Vec<CustomVarDef>,
    pub var_options: Vec<VarTypeOption>,

    // UI state
    pub focus: FocusPane,
    pub bottom_tab: BottomTab,
    pub input_mode: InputMode,

    pub vars_list_state: ListState,
    pub defs_list_state: ListState,
    pub parts_list_state: ListState,
    pub items_list_state: ListState,

    pub selected_var_name: Option<String>,

    pub profiles_filter: String,
    pub vars_filter: String,
    pub defs_filter: String,
    pub parts_filter: String,
    pub items_filter: String,

    pub command_input: String,
    pub command_suggestions: Vec<String>,
    pub command_selected: usize,
    pub search_target: FocusPane,

    pub status: String,
    pub holding: Option<Holding>,

    pub items: Vec<CatalogItem>,
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
        let mut profile_list_state = ListState::default();
        profile_list_state.select(Some(0));
        let mut vars_list_state = ListState::default();
        vars_list_state.select(Some(0));
        let mut defs_list_state = ListState::default();
        defs_list_state.select(Some(0));
        let mut parts_list_state = ListState::default();
        parts_list_state.select(Some(0));
        let mut items_list_state = ListState::default();
        items_list_state.select(Some(0));

        let mut app = AppState {
            conn,
            profiles,
            active_profile_index: 0,
            profile_list_state,
            custom_var_defs: Vec::new(),
            var_options: Vec::new(),

            focus: FocusPane::Vars,
            bottom_tab: BottomTab::Parts,
            input_mode: InputMode::Normal,

            vars_list_state,
            defs_list_state,
            parts_list_state,
            items_list_state,

            selected_var_name: None,

            profiles_filter: String::new(),
            vars_filter: String::new(),
            defs_filter: String::new(),
            parts_filter: String::new(),
            items_filter: String::new(),

            command_input: String::new(),
            command_suggestions: Vec::new(),
            command_selected: 0,
            search_target: FocusPane::Vars,

            status: String::new(),
            holding: None,

            items: Vec::new(),
        };
        app.refresh_var_options()?;
        app.refresh_items()?;
        app.ensure_selected_var();
        Ok(app)
    }

    pub fn refresh_items(&mut self) -> Result<()> {
        self.items = db::load_items(&self.conn)?;
        Ok(())
    }

    pub fn ensure_selected_var(&mut self) {
        if self.selected_var_name.is_some() {
            return;
        }
        if let Some(first) = self.var_options.first() {
            self.selected_var_name = Some(first.name.clone());
        }
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

    #[allow(dead_code)]
    pub fn delete_env_var(&mut self, index: usize) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index < profile.entries.len() {
            profile.entries.remove(index);
            db::save_profile(&self.conn, profile)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_env_var(&mut self, index: usize, entry: Entry) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index < profile.entries.len() {
            profile.entries[index] = entry;
            db::save_profile(&self.conn, profile)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn move_env_var_up(&mut self, index: usize) -> Result<()> {
        let profile = &mut self.profiles[self.active_profile_index];
        if index == 0 || index >= profile.entries.len() {
            return Ok(());
        }
        profile.entries.swap(index - 1, index);
        db::save_profile(&self.conn, profile)?;
        Ok(())
    }

    #[allow(dead_code)]
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

fn create_or_edit_item_dialog<B: Backend>(
    terminal: &mut Terminal<B>,
    initial: Option<&CatalogItem>,
) -> Result<Option<CatalogItem>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Field {
        Kind,
        Value,
        Program,
        Version,
        Tags,
    }

    let mut kind = initial.map(|i| i.kind.clone()).unwrap_or(ItemKind::Text);
    let mut value = initial.map(|i| i.value.clone()).unwrap_or_default();
    let mut program = initial.and_then(|i| i.program.clone()).unwrap_or_default();
    let mut version = initial.and_then(|i| i.version.clone()).unwrap_or_default();
    let mut tags = initial.map(|i| i.tags.join(",")).unwrap_or_default();

    let mut field = Field::Value;
    let id = initial.and_then(|i| i.id);

    loop {
        terminal.draw(|f| {
            let area = centered_rect(80, 45, f.size());
            f.render_widget(Clear, area);

            let title = "🗃️ Item (Tab: next, t: toggle kind, Enter: save, Esc: cancel)";
            let block = Block::default().borders(Borders::ALL).title(title);

            let kind_s = match kind {
                ItemKind::Text => "Text",
                ItemKind::Path => "Path",
            };

            let prefix = |want: Field| if field == want { "> " } else { "  " };

            let program_line = if kind == ItemKind::Path {
                format!("{}Program: {}", prefix(Field::Program), program)
            } else {
                format!("{}Program: (n/a)", prefix(Field::Program))
            };
            let version_line = if kind == ItemKind::Path {
                format!("{}Version: {}", prefix(Field::Version), version)
            } else {
                format!("{}Version: (n/a)", prefix(Field::Version))
            };

            let text = format!(
                "{}Kind: {kind_s}\n{}Value: {value}\n{program_line}\n{version_line}\n{}Tags: {tags}\n\nTip: Use Tags to filter; drop items only works for list-like vars.",
                prefix(Field::Kind),
                prefix(Field::Value),
                prefix(Field::Tags),
            );
            let p = Paragraph::new(text).block(block);
            f.render_widget(p, area);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => {
                        let trimmed = value.trim();
                        if trimmed.is_empty() {
                            return Ok(None);
                        }
                        let tags_vec = tags
                            .split(',')
                            .map(|t| t.trim())
                            .filter(|t| !t.is_empty())
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>();
                        let out = CatalogItem {
                            id,
                            kind: kind.clone(),
                            value: trimmed.to_string(),
                            program: if kind == ItemKind::Path && !program.trim().is_empty() {
                                Some(program.trim().to_string())
                            } else {
                                None
                            },
                            version: if kind == ItemKind::Path && !version.trim().is_empty() {
                                Some(version.trim().to_string())
                            } else {
                                None
                            },
                            tags: tags_vec,
                        };
                        return Ok(Some(out));
                    }
                    KeyCode::Tab => {
                        field = match field {
                            Field::Kind => Field::Value,
                            Field::Value => Field::Program,
                            Field::Program => Field::Version,
                            Field::Version => Field::Tags,
                            Field::Tags => Field::Kind,
                        };
                        if kind == ItemKind::Text
                            && matches!(field, Field::Program | Field::Version)
                        {
                            field = Field::Tags;
                        }
                    }
                    KeyCode::Char('t') | KeyCode::Char('T') => {
                        kind = match kind {
                            ItemKind::Text => ItemKind::Path,
                            ItemKind::Path => ItemKind::Text,
                        };
                        if kind == ItemKind::Text {
                            program.clear();
                            version.clear();
                            if matches!(field, Field::Program | Field::Version) {
                                field = Field::Tags;
                            }
                        }
                    }
                    KeyCode::Backspace => match field {
                        Field::Kind => {}
                        Field::Value => {
                            value.pop();
                        }
                        Field::Program => {
                            if kind == ItemKind::Path {
                                program.pop();
                            }
                        }
                        Field::Version => {
                            if kind == ItemKind::Path {
                                version.pop();
                            }
                        }
                        Field::Tags => {
                            tags.pop();
                        }
                    },
                    KeyCode::Char(c) => match field {
                        Field::Kind => {}
                        Field::Value => value.push(c),
                        Field::Program => {
                            if kind == ItemKind::Path {
                                program.push(c);
                            }
                        }
                        Field::Version => {
                            if kind == ItemKind::Path {
                                version.push(c);
                            }
                        }
                        Field::Tags => tags.push(c),
                    },
                    _ => {}
                }
            }
        }
    }
}

//
// Main TUI: k9s-style multi-pane UI (always visible)
//
pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = AppState::new()?;

    loop {
        terminal.draw(|f| draw_main_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if handle_key_event(&mut terminal, &mut app, key.code)? {
                    break;
                };
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

#[derive(Clone)]
struct VarRow {
    name: String,
    kind: VarKind,
    separator: String,
    count: usize,
}

fn draw_main_ui<B: Backend>(f: &mut ratatui::Frame<B>, app: &mut AppState) {
    // Layout: top row (3 panes) + bottom row (tabs + content) + status bar
    let size = f.size();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(55),
                Constraint::Percentage(43),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(size);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(45),
                Constraint::Percentage(35),
            ]
            .as_ref(),
        )
        .split(outer[0]);

    let profiles_area = top[0];
    let vars_area = top[1];
    let defs_area = top[2];

    let bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(outer[1]);

    draw_profiles_pane(f, profiles_area, app);
    draw_vars_pane(f, vars_area, app);
    draw_defs_pane(f, defs_area, app);
    draw_bottom_pane(f, bottom[0], bottom[1], app);

    // Command/search overlay (k9s-style bottom bar + suggestions)
    if matches!(app.input_mode, InputMode::Command) {
        let sugg = app.command_suggestions.len().min(8);
        let overlay_h = (sugg + 2) as u16; // prompt + suggestions
        let overlay_h = overlay_h.min(size.height.saturating_sub(1));
        let overlay = Rect {
            x: size.x,
            y: size.height.saturating_sub(1 + overlay_h),
            width: size.width,
            height: overlay_h,
        };
        f.render_widget(Clear, overlay);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
            .split(overlay);
        let prompt = Paragraph::new(format!(":{}", app.command_input))
            .block(Block::default().borders(Borders::ALL).title("Command"));
        f.render_widget(prompt, chunks[0]);

        let items: Vec<ListItem> = app
            .command_suggestions
            .iter()
            .take(sugg)
            .map(|s| ListItem::new(s.clone()))
            .collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(
                app.command_selected.min(items.len().saturating_sub(1)),
            ));
        }
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Suggestions (Tab to complete)"),
            )
            .highlight_style(Style::default().bg(Color::Blue))
            .highlight_symbol("» ");
        f.render_stateful_widget(list, chunks[1], &mut state);
    } else if matches!(app.input_mode, InputMode::Search) {
        let overlay_h = 3u16.min(size.height.saturating_sub(1));
        let overlay = Rect {
            x: size.x,
            y: size.height.saturating_sub(1 + overlay_h),
            width: size.width,
            height: overlay_h,
        };
        f.render_widget(Clear, overlay);
        let prompt = Paragraph::new(format!("/{}", app.command_input)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Filter (Enter to apply, Esc to cancel)"),
        );
        f.render_widget(prompt, overlay);
    }

    // Status bar
    let mut status = app.status.clone();
    if let Some(h) = &app.holding {
        let holding_s = match h {
            Holding::Item(i) => format!("holding:item({})", i.value),
            Holding::Part { entry, .. } => format!("holding:part({entry})"),
        };
        if !status.is_empty() {
            status.push_str(" | ");
        }
        status.push_str(&holding_s);
    }
    let status_para = Paragraph::new(status);
    f.render_widget(status_para, outer[2]);
}

fn draw_profiles_pane<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let indices = visible_profile_indices(app);

    let items: Vec<ListItem> = indices
        .iter()
        .map(|i| {
            let p = &app.profiles[*i];
            let active = *i == app.active_profile_index;
            let name = if active {
                format!("* {}", p.name)
            } else {
                format!("  {}", p.name)
            };
            ListItem::new(name)
        })
        .collect();

    let title = if app.profiles_filter.is_empty() {
        "👤 Profiles".to_string()
    } else {
        format!("👤 Profiles  /{}", app.profiles_filter)
    };

    let mut block = Block::default().borders(Borders::ALL).title(title);
    if app.focus == FocusPane::Profiles {
        block = block.border_style(Style::default().fg(Color::Yellow));
    }

    clamp_list_state(&mut app.profile_list_state, items.len());
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.profile_list_state);
}

fn draw_vars_pane<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let rows = compute_var_rows(app);
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            let badge = match r.kind {
                VarKind::Scalar => "S",
                VarKind::List => "L",
            };
            ListItem::new(format!(
                "{}  {:<18}  {:>3}  sep='{}'",
                badge, r.name, r.count, r.separator
            ))
        })
        .collect();

    let title = if app.vars_filter.is_empty() {
        "🧩 Env Vars".to_string()
    } else {
        format!("🧩 Env Vars  /{}", app.vars_filter)
    };
    let mut block = Block::default().borders(Borders::ALL).title(title);
    if app.focus == FocusPane::Vars {
        block = block.border_style(Style::default().fg(Color::Yellow));
    }

    clamp_list_state(&mut app.vars_list_state, items.len());
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.vars_list_state);
}

fn draw_defs_pane<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let mut defs: Vec<&VarTypeOption> = app.var_options.iter().collect();
    defs.sort_by(|a, b| a.name.cmp(&b.name));
    if !app.defs_filter.is_empty() {
        let q = app.defs_filter.to_lowercase();
        defs.retain(|d| d.name.to_lowercase().contains(&q));
    }

    let items: Vec<ListItem> = defs
        .iter()
        .map(|d| {
            let kind = match d.kind {
                VarKind::Scalar => "Scalar",
                VarKind::List => "List",
            };
            ListItem::new(format!(
                "{:<18}  {:<6}  sep='{}'",
                d.name, kind, d.separator
            ))
        })
        .collect();

    let title = if app.defs_filter.is_empty() {
        "📚 Var Defs".to_string()
    } else {
        format!("📚 Var Defs  /{}", app.defs_filter)
    };

    let mut block = Block::default().borders(Borders::ALL).title(title);
    if app.focus == FocusPane::Defs {
        block = block.border_style(Style::default().fg(Color::Yellow));
    }

    clamp_list_state(&mut app.defs_list_state, items.len());
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.defs_list_state);
}

fn draw_bottom_pane<B: Backend>(
    f: &mut ratatui::Frame<B>,
    tabs_area: Rect,
    content_area: Rect,
    app: &mut AppState,
) {
    let tabs_titles = ["Parts", "Items", "Preview", "Export"]
        .iter()
        .map(|t| Spans::from(Span::raw(*t)))
        .collect::<Vec<_>>();
    let idx = match app.bottom_tab {
        BottomTab::Parts => 0,
        BottomTab::Items => 1,
        BottomTab::Preview => 2,
        BottomTab::Export => 3,
    };

    let mut tabs_block = Block::default().borders(Borders::ALL).title("🔎 Details");
    if app.focus == FocusPane::Bottom {
        tabs_block = tabs_block.border_style(Style::default().fg(Color::Yellow));
    }
    let tabs = Tabs::new(tabs_titles)
        .select(idx)
        .block(tabs_block)
        .highlight_style(Style::default().fg(Color::Yellow))
        .divider(Span::raw("|"));
    f.render_widget(tabs, tabs_area);

    match app.bottom_tab {
        BottomTab::Parts => draw_parts_tab(f, content_area, app),
        BottomTab::Items => draw_items_tab(f, content_area, app),
        BottomTab::Preview => draw_preview_tab(f, content_area, app),
        BottomTab::Export => draw_export_tab(f, content_area, app),
    }
}

fn draw_parts_tab<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let parts = current_var_parts(app, &var);
    let indices = visible_part_indices(app, &parts);
    let items: Vec<ListItem> = indices
        .iter()
        .map(|i| ListItem::new(parts[*i].to_string()))
        .collect();
    clamp_list_state(&mut app.parts_list_state, items.len());

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "🧱 Parts for {} (a:add e:edit d:del y:dup J/K:move)",
            var
        )))
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.parts_list_state);
}

fn draw_items_tab<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let indices = visible_item_indices(app);
    let items: Vec<ListItem> = indices
        .iter()
        .map(|i| {
            let it = &app.items[*i];
            let k = match it.kind {
                ItemKind::Text => "TXT",
                ItemKind::Path => "PATH",
            };
            let tags = if it.tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", it.tags.join(","))
            };
            ListItem::new(format!("{:<4} {}{}", k, it.value, tags))
        })
        .collect();

    clamp_list_state(&mut app.items_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🗃️ Items (a:add e:edit d:del y:dup m:pick p:drop)"),
        )
        .highlight_style(Style::default().bg(Color::Blue))
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.items_list_state);
}

fn draw_preview_tab<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let parts = current_var_parts(app, &var);
    let sep = app
        .var_options
        .iter()
        .find(|o| o.name == var)
        .map(|o| o.separator.clone())
        .unwrap_or_else(|| ":".to_string());
    let joined = parts
        .iter()
        .map(preview_value)
        .collect::<Vec<_>>()
        .join(&sep);
    let text = format!("{var} = {joined}\n\n(parts: {})", parts.len());
    let p = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("👀 Preview"));
    f.render_widget(p, area);
}

fn draw_export_tab<B: Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let profile = &app.profiles[app.active_profile_index];
    let full = export::generate_full_export(profile, OperationMode::Prepend);
    let line = full
        .lines()
        .find(|l| l.starts_with(&format!("export {var}=")))
        .unwrap_or("");
    let p = Paragraph::new(line.to_string())
        .block(Block::default().borders(Borders::ALL).title("📤 Export"));
    f.render_widget(p, area);
}

fn clamp_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let selected = state.selected().unwrap_or(0);
    let next = selected.min(len.saturating_sub(1));
    state.select(Some(next));
}

fn compute_var_rows(app: &AppState) -> Vec<VarRow> {
    let profile = &app.profiles[app.active_profile_index];
    let mut rows: Vec<VarRow> = app
        .var_options
        .iter()
        .map(|o| {
            let count = profile
                .entries
                .iter()
                .filter(|e| e.var_name().as_ref() == o.name)
                .count();
            VarRow {
                name: o.name.clone(),
                kind: o.kind.clone(),
                separator: o.separator.clone(),
                count,
            }
        })
        .collect();

    // Add any unknown vars that exist in the profile but have no definition row.
    for e in &profile.entries {
        let name = e.var_name().into_owned();
        if rows.iter().any(|r| r.name == name) {
            continue;
        }
        rows.push(VarRow {
            name: name.clone(),
            kind: VarKind::List,
            separator: e.separator().into_owned(),
            count: profile
                .entries
                .iter()
                .filter(|x| x.var_name().as_ref() == name)
                .count(),
        });
    }

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    if !app.vars_filter.is_empty() {
        let q = app.vars_filter.to_lowercase();
        rows.retain(|r| r.name.to_lowercase().contains(&q));
    }
    rows
}

fn current_var_parts(app: &AppState, var_name: &str) -> Vec<Entry> {
    let profile = &app.profiles[app.active_profile_index];
    profile
        .entries
        .iter()
        .filter(|e| e.var_name().as_ref() == var_name)
        .cloned()
        .collect()
}

fn visible_part_indices(app: &AppState, parts: &[Entry]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..parts.len()).collect();
    if !app.parts_filter.is_empty() {
        let q = app.parts_filter.to_lowercase();
        indices.retain(|i| parts[*i].to_string().to_lowercase().contains(&q));
    }
    indices
}

fn visible_item_indices(app: &AppState) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..app.items.len()).collect();
    if !app.items_filter.is_empty() {
        let q = app.items_filter.to_lowercase();
        indices.retain(|i| {
            let it = &app.items[*i];
            it.value.to_lowercase().contains(&q)
                || it.tags.iter().any(|t| t.to_lowercase().contains(&q))
        });
    }
    indices
}

fn visible_profile_indices(app: &AppState) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..app.profiles.len()).collect();
    if !app.profiles_filter.is_empty() {
        let q = app.profiles_filter.to_lowercase();
        indices.retain(|i| app.profiles[*i].name.to_lowercase().contains(&q));
    }
    indices
}

fn selected_profile_index(app: &AppState) -> Option<usize> {
    let indices = visible_profile_indices(app);
    app.profile_list_state
        .selected()
        .and_then(|i| indices.get(i).copied())
}

fn selected_item_index(app: &AppState) -> Option<usize> {
    let indices = visible_item_indices(app);
    app.items_list_state
        .selected()
        .and_then(|i| indices.get(i).copied())
}

fn preview_value(e: &Entry) -> String {
    match e {
        Entry::Path(PathEntry { path, .. }) => path.clone(),
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
        | Entry::Lang(v) => v.clone(),
        Entry::CustomScalar { value, .. } => value.clone(),
        Entry::CustomPart { value, .. } => value.clone(),
    }
}

fn make_part_entry(app: &AppState, var_name: &str, value: String) -> Option<Entry> {
    // Builtins
    match var_name {
        "PATH" => {
            return Some(Entry::Path(PathEntry {
                path: value,
                program: String::new(),
                version: String::new(),
            }))
        }
        "CPATH" => return Some(Entry::CPath(value)),
        "C_INCLUDE_PATH" => return Some(Entry::CInclude(value)),
        "CPLUS_INCLUDE_PATH" => return Some(Entry::CPlusInclude(value)),
        "OBJC_INCLUDE_PATH" => return Some(Entry::OBJCInclude(value)),
        "CPPFLAGS" => return Some(Entry::CPPFlag(value)),
        "CFLAGS" => return Some(Entry::CFlag(value)),
        "CXXFLAGS" => return Some(Entry::CXXFlag(value)),
        "LDFLAGS" => return Some(Entry::LDFlag(value)),
        "LIBRARY_PATH" => return Some(Entry::LibraryPath(value)),
        "LD_LIBRARY_PATH" => return Some(Entry::LDLibraryPath(value)),
        "LD_RUN_PATH" => return Some(Entry::LDRunPath(value)),
        "RANLIB" => return Some(Entry::RanLib(value)),
        "CC" => return Some(Entry::CC(value)),
        "CXX" => return Some(Entry::CXX(value)),
        "AR" => return Some(Entry::AR(value)),
        "STRIP" => return Some(Entry::Strip(value)),
        "GCC_EXEC_PREFIX" => return Some(Entry::GCCExecPrefix(value)),
        "COLLECT_GCC_OPTIONS" => return Some(Entry::CollectGCCOptions(value)),
        "LANG" => return Some(Entry::Lang(value)),
        _ => {}
    }

    // Custom vars: look up definition; only List accepts parts.
    if let Some(def) = app.custom_var_defs.iter().find(|d| d.name == var_name) {
        return match def.kind {
            VarKind::Scalar => Some(Entry::CustomScalar {
                name: def.name.clone(),
                value,
            }),
            VarKind::List => Some(Entry::CustomPart {
                name: def.name.clone(),
                value,
                separator: def.separator.clone(),
            }),
        };
    }

    // Unknown: treat as list-part with ':' separator.
    Some(Entry::CustomPart {
        name: var_name.to_string(),
        value,
        separator: ":".to_string(),
    })
}

fn var_option_for(app: &AppState, var_name: &str) -> VarTypeOption {
    if let Some(o) = app.var_options.iter().find(|o| o.name == var_name) {
        return o.clone();
    }
    // Unknown: default to list/':' so it can accept items/parts.
    VarTypeOption {
        name: var_name.to_string(),
        kind: VarKind::List,
        separator: ":".to_string(),
        editor: EditorStyle::PartsList,
    }
}

fn all_commands() -> Vec<String> {
    vec![
        "profiles".to_string(),
        "vars".to_string(),
        "defs".to_string(),
        "parts".to_string(),
        "items".to_string(),
        "preview".to_string(),
        "export".to_string(),
        "use".to_string(),
        "new-var".to_string(),
        "new-item".to_string(),
        "help".to_string(),
    ]
}

fn refresh_command_suggestions(app: &mut AppState) {
    let input = app.command_input.trim_start();
    let mut suggestions = Vec::new();

    if input.starts_with("use ") {
        let q = input.trim_start_matches("use ").trim().to_lowercase();
        for p in &app.profiles {
            if q.is_empty() || p.name.to_lowercase().contains(&q) {
                suggestions.push(format!("use {}", p.name));
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

fn pick_command_to_execute(app: &AppState) -> String {
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

fn execute_command<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    cmd: &str,
) -> Result<()> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return Ok(());
    }

    if cmd == "profiles" {
        app.focus = FocusPane::Profiles;
        return Ok(());
    }
    if cmd == "vars" {
        app.focus = FocusPane::Vars;
        return Ok(());
    }
    if cmd == "defs" {
        app.focus = FocusPane::Defs;
        return Ok(());
    }
    if cmd == "parts" {
        app.focus = FocusPane::Bottom;
        app.bottom_tab = BottomTab::Parts;
        return Ok(());
    }
    if cmd == "items" {
        app.focus = FocusPane::Bottom;
        app.bottom_tab = BottomTab::Items;
        return Ok(());
    }
    if cmd == "preview" {
        app.focus = FocusPane::Bottom;
        app.bottom_tab = BottomTab::Preview;
        return Ok(());
    }
    if cmd == "export" {
        app.focus = FocusPane::Bottom;
        app.bottom_tab = BottomTab::Export;
        return Ok(());
    }
    if cmd == "new-var" {
        if let Some(def) = create_custom_var_dialog(terminal)? {
            db::save_custom_var_def(&app.conn, &def)?;
            app.refresh_var_options()?;
            app.status = format!("saved var def: {}", def.name);
        }
        return Ok(());
    }
    if cmd == "new-item" {
        if let Some(mut item) = create_or_edit_item_dialog(terminal, None)? {
            db::save_item(&app.conn, &mut item)?;
            app.refresh_items()?;
            app.status = format!("saved item: {}", item.value);
        }
        return Ok(());
    }
    if cmd == "help" {
        app.status = "Commands: profiles vars defs parts items preview export use <profile> new-var new-item".to_string();
        return Ok(());
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
        return Ok(());
    }

    Ok(())
}

fn handle_key_event<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    code: KeyCode,
) -> Result<bool> {
    match app.input_mode {
        InputMode::Normal => handle_normal_key(terminal, app, code),
        InputMode::Command => handle_command_key(terminal, app, code),
        InputMode::Search => handle_search_key(app, code),
    }
}

fn cycle_focus(app: &mut AppState) {
    app.focus = match app.focus {
        FocusPane::Profiles => FocusPane::Vars,
        FocusPane::Vars => FocusPane::Defs,
        FocusPane::Defs => FocusPane::Bottom,
        FocusPane::Bottom => FocusPane::Profiles,
    };
}

fn handle_normal_key<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    code: KeyCode,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc => {
            if let Some(Holding::Part { var, from, entry }) = app.holding.take() {
                let mut parts = current_var_parts(app, &var);
                let insert_at = from.min(parts.len());
                parts.insert(insert_at, entry);
                app.replace_var_parts(&var, parts)?;
                app.status = "cancelled move".to_string();
            } else {
                app.status.clear();
            }
        }
        KeyCode::Tab => {
            cycle_focus(app);
        }
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            app.command_selected = 0;
            refresh_command_suggestions(app);
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_target = app.focus;
            app.command_input.clear();
        }
        KeyCode::Char('1') => app.bottom_tab = BottomTab::Parts,
        KeyCode::Char('2') => app.bottom_tab = BottomTab::Items,
        KeyCode::Char('3') => app.bottom_tab = BottomTab::Preview,
        KeyCode::Char('4') => app.bottom_tab = BottomTab::Export,
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),
        KeyCode::Enter => {
            activate_selection(app);
        }
        KeyCode::Char('C') => {
            if let Some(def) = create_custom_var_dialog(terminal)? {
                db::save_custom_var_def(&app.conn, &def)?;
                app.refresh_var_options()?;
                app.status = format!("saved var def: {}", def.name);
            }
        }
        KeyCode::Char('A') if app.focus == FocusPane::Profiles => {
            if let Some(new_name) = edit_profile_name_dialog(terminal, None)? {
                let new_profile = EnvProfile::new(&new_name);
                app.add_profile(new_profile)?;
                app.active_profile_index = app.profiles.len().saturating_sub(1);
                app.status = format!("added profile: {new_name}");
            }
        }
        KeyCode::Char('E') if app.focus == FocusPane::Profiles => {
            if let Some(i) = selected_profile_index(app) {
                let current_name = app.profiles[i].name.clone();
                if let Some(new_name) = edit_profile_name_dialog(terminal, Some(&current_name))? {
                    app.update_profile(i, new_name.clone())?;
                    app.status = format!("renamed profile: {current_name} -> {new_name}");
                }
            }
        }
        KeyCode::Char('D') if app.focus == FocusPane::Profiles => {
            if let Some(i) = selected_profile_index(app) {
                if confirm_dialog(terminal, "Delete profile?")? {
                    let name = app.profiles[i].name.clone();
                    app.delete_profile(i)?;
                    app.status = format!("deleted profile: {name}");
                }
            }
        }
        KeyCode::Char('a')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            let mut item = create_or_edit_item_dialog(terminal, None)?;
            if let Some(mut item) = item.take() {
                db::save_item(&app.conn, &mut item)?;
                app.refresh_items()?;
                app.status = format!("saved item: {}", item.value);
            }
        }
        KeyCode::Char('e')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            if let Some(i) = selected_item_index(app) {
                let initial = app.items.get(i).cloned();
                if let Some(initial) = initial {
                    if let Some(mut edited) = create_or_edit_item_dialog(terminal, Some(&initial))?
                    {
                        db::save_item(&app.conn, &mut edited)?;
                        app.refresh_items()?;
                        app.status = format!("updated item: {}", edited.value);
                    }
                }
            }
        }
        KeyCode::Char('d')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            if let Some(i) = selected_item_index(app) {
                if let Some(id) = app.items.get(i).and_then(|it| it.id) {
                    if confirm_dialog(terminal, "Delete item?")? {
                        db::delete_item(&app.conn, id)?;
                        app.refresh_items()?;
                        app.status = "deleted item".to_string();
                    }
                }
            }
        }
        KeyCode::Char('y')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            if let Some(i) = selected_item_index(app) {
                if let Some(orig) = app.items.get(i).cloned() {
                    let mut dup = orig.clone();
                    dup.id = None;
                    db::save_item(&app.conn, &mut dup)?;
                    app.refresh_items()?;
                    app.status = "duplicated item".to_string();
                }
            }
        }
        KeyCode::Char('m')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            if let Some(i) = selected_item_index(app) {
                if let Some(it) = app.items.get(i).cloned() {
                    app.holding = Some(Holding::Item(it));
                    app.status = "picked item".to_string();
                }
            }
        }
        KeyCode::Char('p')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Items =>
        {
            // Drop selected item into currently selected var.
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = var_option_for(app, &var);
            if opt.kind != VarKind::List {
                app.status = "cannot drop into scalar var".to_string();
            } else if let Some(i) = selected_item_index(app) {
                if let Some(it) = app.items.get(i).cloned() {
                    if let Some(e) = make_part_entry(app, &var, it.value) {
                        app.add_env_var(e)?;
                        app.status = format!("dropped into {var}");
                    }
                }
            }
        }
        KeyCode::Char('p') if app.focus == FocusPane::Vars => {
            // Drop held item into selected var (append).
            if let Some(Holding::Item(it)) = app.holding.clone() {
                let rows = compute_var_rows(app);
                if let Some(i) = app.vars_list_state.selected() {
                    if let Some(row) = rows.get(i) {
                        if row.kind != VarKind::List {
                            app.status = "cannot drop into scalar var".to_string();
                        } else if let Some(e) = make_part_entry(app, &row.name, it.value.clone()) {
                            app.add_env_var(e)?;
                            app.holding = None;
                            app.status = format!("dropped into {}", row.name);
                        }
                    }
                }
            }
        }
        KeyCode::Char('a')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = var_option_for(app, &var);
            if let Some(new_entry) =
                editor::edit_env_var_dialog(terminal, std::slice::from_ref(&opt), None)?
            {
                if opt.kind == VarKind::Scalar {
                    app.replace_var_parts(&var, vec![new_entry])?;
                } else {
                    app.add_env_var(new_entry)?;
                }
                app.status = format!("added part to {var}");
            }
        }
        KeyCode::Char('e')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = var_option_for(app, &var);
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    let initial = parts.get(part_i).cloned();
                    if let Some(initial) = initial {
                        if let Some(new_entry) = editor::edit_env_var_dialog(
                            terminal,
                            std::slice::from_ref(&opt),
                            Some(&initial),
                        )? {
                            parts[part_i] = new_entry;
                            app.replace_var_parts(&var, parts)?;
                            app.status = format!("edited part in {var}");
                        }
                    }
                }
            }
        }
        KeyCode::Char('d')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    parts.remove(part_i);
                    app.replace_var_parts(&var, parts)?;
                    app.status = format!("deleted part from {var}");
                }
            }
        }
        KeyCode::Char('y')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    let dup = parts[part_i].clone();
                    parts.insert(part_i + 1, dup);
                    app.replace_var_parts(&var, parts)?;
                    app.status = format!("duplicated part in {var}");
                }
            }
        }
        KeyCode::Char('K')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    if part_i > 0 {
                        parts.swap(part_i - 1, part_i);
                        app.replace_var_parts(&var, parts)?;
                        app.parts_list_state.select(Some(sel.saturating_sub(1)));
                        app.status = format!("moved part up in {var}");
                    }
                }
            }
        }
        KeyCode::Char('J')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    if part_i + 1 < parts.len() {
                        parts.swap(part_i, part_i + 1);
                        app.replace_var_parts(&var, parts)?;
                        app.parts_list_state.select(Some(sel + 1));
                        app.status = format!("moved part down in {var}");
                    }
                }
            }
        }
        KeyCode::Char('m')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = current_var_parts(app, &var);
            let visible = visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    let entry = parts.remove(part_i);
                    app.replace_var_parts(&var, parts)?;
                    app.holding = Some(Holding::Part {
                        var: var.clone(),
                        from: part_i,
                        entry,
                    });
                    app.status = "moving part (navigate, p:drop, Esc:cancel)".to_string();
                }
            }
        }
        KeyCode::Char('p')
            if app.focus == FocusPane::Bottom && app.bottom_tab == BottomTab::Parts =>
        {
            // Drop held item into parts list at cursor.
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = var_option_for(app, &var);
            if opt.kind != VarKind::List {
                app.status = "cannot drop into scalar var".to_string();
            } else if let Some(holding) = app.holding.clone() {
                let mut parts = current_var_parts(app, &var);
                let visible = visible_part_indices(app, &parts);
                let insert_at = app
                    .parts_list_state
                    .selected()
                    .and_then(|sel| visible.get(sel).copied())
                    .unwrap_or(parts.len());

                match holding {
                    Holding::Item(it) => {
                        if let Some(e) = make_part_entry(app, &var, it.value.clone()) {
                            parts.insert(insert_at, e);
                            app.replace_var_parts(&var, parts)?;
                            app.holding = None;
                            app.status = format!("dropped into {var}");
                        }
                    }
                    Holding::Part { entry, .. } => {
                        // If moving between different vars, convert by value.
                        let value = preview_value(&entry);
                        if let Some(e) = make_part_entry(app, &var, value) {
                            parts.insert(insert_at, e);
                            app.replace_var_parts(&var, parts)?;
                            app.holding = None;
                            app.status = format!("moved part into {var}");
                        } else {
                            app.status = "cannot drop into target var".to_string();
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

fn move_selection(app: &mut AppState, delta: isize) {
    let state = match app.focus {
        FocusPane::Profiles => &mut app.profile_list_state,
        FocusPane::Vars => &mut app.vars_list_state,
        FocusPane::Defs => &mut app.defs_list_state,
        FocusPane::Bottom => match app.bottom_tab {
            BottomTab::Parts => &mut app.parts_list_state,
            BottomTab::Items => &mut app.items_list_state,
            BottomTab::Preview | BottomTab::Export => return,
        },
    };
    let cur = state.selected().unwrap_or(0) as isize;
    let next = (cur + delta).max(0) as usize;
    state.select(Some(next));
}

fn activate_selection(app: &mut AppState) {
    match app.focus {
        FocusPane::Profiles => {
            if let Some(i) = selected_profile_index(app) {
                app.active_profile_index = i;
                app.status = format!("profile: {}", app.profiles[i].name);
            }
        }
        FocusPane::Vars => {
            let rows = compute_var_rows(app);
            if let Some(i) = app.vars_list_state.selected() {
                if let Some(row) = rows.get(i) {
                    app.selected_var_name = Some(row.name.clone());
                    app.bottom_tab = BottomTab::Parts;
                    app.focus = FocusPane::Bottom;
                    app.status = format!("selected var: {}", row.name);
                }
            }
        }
        FocusPane::Defs => {
            // no-op for now
        }
        FocusPane::Bottom => {}
    }
}

fn handle_command_key<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    code: KeyCode,
) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            let exec = pick_command_to_execute(app);
            execute_command(terminal, app, &exec)?;
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Tab => {
            if let Some(s) = app.command_suggestions.get(app.command_selected).cloned() {
                app.command_input = s;
                refresh_command_suggestions(app);
            }
        }
        KeyCode::Up => {
            app.command_selected = app.command_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            if !app.command_suggestions.is_empty() {
                app.command_selected =
                    (app.command_selected + 1).min(app.command_suggestions.len() - 1);
            }
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            refresh_command_suggestions(app);
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
            refresh_command_suggestions(app);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_search_key(app: &mut AppState, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.command_input.clear();
            apply_live_filter(app, "");
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.command_input.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            let q = app.command_input.clone();
            apply_live_filter(app, &q);
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
            let q = app.command_input.clone();
            apply_live_filter(app, &q);
        }
        _ => {}
    }
    Ok(false)
}

fn apply_live_filter(app: &mut AppState, q: &str) {
    let q = q.to_string();
    match app.search_target {
        FocusPane::Profiles => app.profiles_filter = q,
        FocusPane::Vars => app.vars_filter = q,
        FocusPane::Defs => app.defs_filter = q,
        FocusPane::Bottom => match app.bottom_tab {
            BottomTab::Parts => app.parts_filter = q,
            BottomTab::Items => app.items_filter = q,
            BottomTab::Preview | BottomTab::Export => {}
        },
    }
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

    #[allow(dead_code)]
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
            conn,
            profiles: vec![default],
            active_profile_index: 0,
            profile_list_state: ListState::default(),
            custom_var_defs: Vec::new(),
            var_options: builtin_var_options(),
            focus: FocusPane::Vars,
            bottom_tab: BottomTab::Parts,
            input_mode: InputMode::Normal,
            vars_list_state: ListState::default(),
            defs_list_state: ListState::default(),
            parts_list_state: ListState::default(),
            items_list_state: ListState::default(),
            selected_var_name: Some("PATH".to_string()),
            profiles_filter: String::new(),
            vars_filter: String::new(),
            defs_filter: String::new(),
            parts_filter: String::new(),
            items_filter: String::new(),
            command_input: String::new(),
            command_suggestions: Vec::new(),
            command_selected: 0,
            search_target: FocusPane::Vars,
            status: String::new(),
            holding: None,
            items: Vec::new(),
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
