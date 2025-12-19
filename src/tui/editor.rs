use crate::config::{Entry, PathEntry, VarKind};
use crate::export::{self, OperationMode};
use crate::tui::util::centered_rect;
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

fn is_path_part(opt: &crate::tui::state::VarTypeOption) -> bool {
    opt.editor == crate::tui::state::EditorStyle::PathPart
}

#[allow(dead_code)]
pub fn edit_var_parts_dialog<B: Backend>(
    terminal: &mut Terminal<B>,
    var: &crate::tui::state::VarTypeOption,
    initial_parts: &[Entry],
) -> Result<Option<Vec<Entry>>> {
    let mut parts: Vec<Entry> = initial_parts.to_vec();
    let mut selected: usize = 0;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let area = centered_rect(85, 70, size);
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
                            if let Some(new_entry) = edit_env_var_dialog(terminal, &one, current)? {
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
    pub all_options: Vec<crate::tui::state::VarTypeOption>,
    pub search: String,
    pub filtered: Vec<crate::tui::state::VarTypeOption>,
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
    pub fn new(options: &[crate::tui::state::VarTypeOption], initial: Option<&Entry>) -> Self {
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

fn entry_from_state(opt: &crate::tui::state::VarTypeOption, state: &EnvVarEditorState) -> Entry {
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
    options: &[crate::tui::state::VarTypeOption],
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
                .split(centered_rect(80, 60, size));

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
                        let indicator =
                            if state.focus == FocusArea::Input && state.active_input_field == i {
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
                    .unwrap_or_else(|| crate::tui::state::VarTypeOption {
                        name: "CFLAGS".to_string(),
                        kind: VarKind::List,
                        separator: " ".to_string(),
                        editor: crate::tui::state::EditorStyle::Single,
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
        let options = crate::tui::state::builtin_var_options();
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
