use crate::config::{EnvProfile, VarKind};
use crate::db;
use crate::profile_editor::{confirm_dialog, edit_profile_name_dialog};
use crate::tui::{commands, dialogs, editor, select};
use crate::tui::state::{AppState, Holding, InputMode};
use crate::tui::view::View;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::backend::Backend;
use ratatui::Terminal;

pub fn handle_key_event<B: Backend>(
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

fn cycle_view(app: &mut AppState) {
    app.active_view = match app.active_view {
        View::Profiles => View::Vars,
        View::Vars => View::Parts,
        View::Parts => View::Items,
        View::Items => View::Defs,
        View::Defs => View::Preview,
        View::Preview => View::Export,
        View::Export => View::Help,
        View::Help => View::Profiles,
    };
}

fn handle_normal_key<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    code: KeyCode,
) -> Result<bool> {
    match code {
        // Quit
        KeyCode::Char('q') => return Ok(true),

        // Vim-ish movement keys
        KeyCode::Char('j') => move_selection(app, 1),
        KeyCode::Char('k') => move_selection(app, -1),
        KeyCode::Char('G') | KeyCode::Home => jump_to_top(app),
        KeyCode::Char('g') | KeyCode::End => jump_to_bottom(app),
        KeyCode::PageUp => move_selection(app, -10),
        KeyCode::PageDown => move_selection(app, 10),

        KeyCode::Esc => {
            // Global cancel for in-progress part move.
            if let Some(Holding::Part { var, from, entry }) = app.holding.take() {
                let mut parts = select::current_var_parts(app, &var);
                let insert_at = from.min(parts.len());
                parts.insert(insert_at, entry);
                app.replace_var_parts(&var, parts)?;
                app.status = "cancelled move".to_string();
            } else {
                app.status.clear();
            }
        }

        KeyCode::Tab => cycle_view(app),

        KeyCode::Char('?') => app.active_view = View::Help,

        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            app.command_selected = 0;
            commands::refresh_command_suggestions(app);
        }

        KeyCode::Char('/') => {
            if app.active_view.is_filterable() {
                app.input_mode = InputMode::Search;
                app.search_target = app.active_view;
                app.command_input.clear();
            }
        }

        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),

        KeyCode::Enter => activate_selection(app),

        // Profiles view actions
        KeyCode::Char('A') if app.active_view == View::Profiles => {
            if let Some(new_name) = edit_profile_name_dialog(terminal, None)? {
                let new_profile = EnvProfile::new(&new_name);
                app.add_profile(new_profile)?;
                app.active_profile_index = app.profiles.len().saturating_sub(1);
                app.status = format!("added profile: {new_name}");
            }
        }
        KeyCode::Char('E') if app.active_view == View::Profiles => {
            if let Some(i) = select::selected_profile_index(app) {
                let current_name = app.profiles[i].name.clone();
                if let Some(new_name) = edit_profile_name_dialog(terminal, Some(&current_name))? {
                    app.update_profile(i, new_name.clone())?;
                    app.status = format!("renamed profile: {current_name} -> {new_name}");
                }
            }
        }
        KeyCode::Char('D') if app.active_view == View::Profiles => {
            if let Some(i) = select::selected_profile_index(app) {
                if confirm_dialog(terminal, "Delete profile?")? {
                    let name = app.profiles[i].name.clone();
                    app.delete_profile(i)?;
                    app.status = format!("deleted profile: {name}");
                }
            }
        }

        // Defs view actions
        KeyCode::Char('C') if app.active_view == View::Defs => {
            if let Some(def) = dialogs::create_custom_var_dialog(terminal)? {
                db::save_custom_var_def(&app.conn, &def)?;
                app.refresh_var_options()?;
                app.status = format!("saved var def: {}", def.name);
            }
        }

        // Items view actions
        KeyCode::Char('a') if app.active_view == View::Items => {
            if let Some(mut item) = dialogs::create_or_edit_item_dialog(terminal, None)? {
                db::save_item(&app.conn, &mut item)?;
                app.refresh_items()?;
                app.status = format!("saved item: {}", item.value);
            }
        }
        KeyCode::Char('e') if app.active_view == View::Items => {
            if let Some(i) = select::selected_item_index(app) {
                if let Some(initial) = app.items.get(i).cloned() {
                    if let Some(mut edited) =
                        dialogs::create_or_edit_item_dialog(terminal, Some(&initial))?
                    {
                        db::save_item(&app.conn, &mut edited)?;
                        app.refresh_items()?;
                        app.status = format!("updated item: {}", edited.value);
                    }
                }
            }
        }
        KeyCode::Char('d') if app.active_view == View::Items => {
            if let Some(i) = select::selected_item_index(app) {
                if let Some(id) = app.items.get(i).and_then(|it| it.id) {
                    if confirm_dialog(terminal, "Delete item?")? {
                        db::delete_item(&app.conn, id)?;
                        app.refresh_items()?;
                        app.status = "deleted item".to_string();
                    }
                }
            }
        }
        KeyCode::Char('y') if app.active_view == View::Items => {
            if let Some(i) = select::selected_item_index(app) {
                if let Some(orig) = app.items.get(i).cloned() {
                    let mut dup = orig.clone();
                    dup.id = None;
                    db::save_item(&app.conn, &mut dup)?;
                    app.refresh_items()?;
                    app.status = "duplicated item".to_string();
                }
            }
        }
        KeyCode::Char('m') if app.active_view == View::Items => {
            if let Some(i) = select::selected_item_index(app) {
                if let Some(it) = app.items.get(i).cloned() {
                    app.holding = Some(Holding::Item(it));
                    app.status = "picked item".to_string();
                }
            }
        }
        KeyCode::Char('p') if app.active_view == View::Items => {
            // Drop selected item into current var context.
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = select::var_option_for(app, &var);
            if opt.kind != VarKind::List {
                app.status = "cannot drop into scalar var".to_string();
            } else if let Some(i) = select::selected_item_index(app) {
                if let Some(it) = app.items.get(i).cloned() {
                    if let Some(e) = select::make_part_entry(app, &var, it.value) {
                        app.add_env_var(e)?;
                        app.status = format!("dropped into {var}");
                    }
                }
            }
        }

        // Vars view actions
        KeyCode::Char('p') if app.active_view == View::Vars => {
            // Drop held item into selected var (append).
            if let Some(Holding::Item(it)) = app.holding.clone() {
                let rows = select::compute_var_rows(app);
                if let Some(i) = app.vars_list_state.selected() {
                    if let Some(row) = rows.get(i) {
                        if row.kind != VarKind::List {
                            app.status = "cannot drop into scalar var".to_string();
                        } else if let Some(e) =
                            select::make_part_entry(app, &row.name, it.value.clone())
                        {
                            app.add_env_var(e)?;
                            app.holding = None;
                            app.selected_var_name = Some(row.name.clone());
                            app.status = format!("dropped into {}", row.name);
                        }
                    }
                }
            }
        }

        // Parts view actions
        KeyCode::Char('a') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = select::var_option_for(app, &var);
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
        KeyCode::Char('e') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = select::var_option_for(app, &var);
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    if let Some(initial) = parts.get(part_i).cloned() {
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
        KeyCode::Char('d') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    parts.remove(part_i);
                    app.replace_var_parts(&var, parts)?;
                    app.status = format!("deleted part from {var}");
                }
            }
        }
        KeyCode::Char('y') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
            if let Some(sel) = app.parts_list_state.selected() {
                if let Some(part_i) = visible.get(sel).copied() {
                    let dup = parts[part_i].clone();
                    parts.insert(part_i + 1, dup);
                    app.replace_var_parts(&var, parts)?;
                    app.status = format!("duplicated part in {var}");
                }
            }
        }
        KeyCode::Char('K') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
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
        KeyCode::Char('J') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
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
        KeyCode::Char('m') if app.active_view == View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let mut parts = select::current_var_parts(app, &var);
            let visible = select::visible_part_indices(app, &parts);
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
        KeyCode::Char('p') if app.active_view == View::Parts => {
            // Drop held item/part into parts list at cursor.
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let opt = select::var_option_for(app, &var);
            if opt.kind != VarKind::List {
                app.status = "cannot drop into scalar var".to_string();
            } else if let Some(holding) = app.holding.clone() {
                let mut parts = select::current_var_parts(app, &var);
                let visible = select::visible_part_indices(app, &parts);
                let insert_at = app
                    .parts_list_state
                    .selected()
                    .and_then(|sel| visible.get(sel).copied())
                    .unwrap_or(parts.len());

                match holding {
                    Holding::Item(it) => {
                        if let Some(e) = select::make_part_entry(app, &var, it.value.clone()) {
                            parts.insert(insert_at, e);
                            app.replace_var_parts(&var, parts)?;
                            app.holding = None;
                            app.status = format!("dropped into {var}");
                        }
                    }
                    Holding::Part { entry, .. } => {
                        // If moving between different vars, convert by value.
                        let value = select::preview_value(&entry);
                        if let Some(e) = select::make_part_entry(app, &var, value) {
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
    // Clamp selection to visible list bounds at input-time. Otherwise the selection index can grow
    // unbounded (e.g. holding Down at end), making it take many Up presses to get back in range.
    let len = {
        let a: &AppState = &*app;
        match a.active_view {
            View::Profiles => select::visible_profile_indices(a).len(),
            View::Vars => select::compute_var_rows(a).len(),
            View::Parts => {
                let var = a
                    .selected_var_name
                    .clone()
                    .unwrap_or_else(|| "PATH".to_string());
                let parts = select::current_var_parts(a, &var);
                select::visible_part_indices(a, &parts).len()
            }
            View::Items => select::visible_item_indices(a).len(),
            View::Defs => {
                let mut defs = a.var_options.clone();
                defs.sort_by(|x, y| x.name.cmp(&y.name));
                if !a.defs_filter.is_empty() {
                    let q = a.defs_filter.to_lowercase();
                    defs.retain(|d| d.name.to_lowercase().contains(&q));
                }
                defs.len()
            }
            View::Preview | View::Export | View::Help => 0,
        }
    };

    let state = match app.active_view {
        View::Profiles => Some(&mut app.profile_list_state),
        View::Vars => Some(&mut app.vars_list_state),
        View::Defs => Some(&mut app.defs_list_state),
        View::Parts => Some(&mut app.parts_list_state),
        View::Items => Some(&mut app.items_list_state),
        View::Preview | View::Export | View::Help => None,
    };

    let Some(state) = state else { return; };

    if len == 0 {
        state.select(None);
        return;
    }

    let cur = state.selected().unwrap_or(0).min(len - 1) as isize;
    let next = (cur + delta).clamp(0, (len - 1) as isize) as usize;
    state.select(Some(next));

    // Keep var context aligned with highlighted row when in Vars view.
    if app.active_view == View::Vars {
        let rows = select::compute_var_rows(app);
        if let Some(i) = app.vars_list_state.selected() {
            if let Some(row) = rows.get(i) {
                app.selected_var_name = Some(row.name.clone());
            }
        }
    }
}

fn activate_selection(app: &mut AppState) {
    match app.active_view {
        View::Profiles => {
            if let Some(i) = select::selected_profile_index(app) {
                app.active_profile_index = i;
                app.status = format!("profile: {}", app.profiles[i].name);
            }
        }
        View::Vars => {
            let rows = select::compute_var_rows(app);
            if let Some(i) = app.vars_list_state.selected() {
                if let Some(row) = rows.get(i) {
                    app.selected_var_name = Some(row.name.clone());
                    app.active_view = View::Parts;
                    app.status = format!("selected var: {}", row.name);
                }
            }
        }
        _ => {}
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
            let exec = commands::pick_command_to_execute(app);
            let quit = commands::execute_command(terminal, app, &exec)?;
            app.input_mode = InputMode::Normal;
            if quit {
                return Ok(true);
            }
        }
        KeyCode::Tab => {
            if let Some(s) = app.command_suggestions.get(app.command_selected).cloned() {
                app.command_input = s;
                commands::refresh_command_suggestions(app);
            }
        }
        KeyCode::Up => {
            app.command_selected = app.command_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            if !app.command_suggestions.is_empty() {
                app.command_selected = (app.command_selected + 1).min(app.command_suggestions.len() - 1);
            }
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            commands::refresh_command_suggestions(app);
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
            commands::refresh_command_suggestions(app);
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
        View::Profiles => app.profiles_filter = q,
        View::Vars => app.vars_filter = q,
        View::Defs => app.defs_filter = q,
        View::Parts => app.parts_filter = q,
        View::Items => app.items_filter = q,
        View::Preview | View::Export | View::Help => {}
    }
}

fn jump_to_top(app: &mut AppState) {
    let state = match app.active_view {
        View::Profiles => Some(&mut app.profile_list_state),
        View::Vars => Some(&mut app.vars_list_state),
        View::Defs => Some(&mut app.defs_list_state),
        View::Parts => Some(&mut app.parts_list_state),
        View::Items => Some(&mut app.items_list_state),
        View::Preview | View::Export | View::Help => None,
    };
    if let Some(state) = state {
        state.select(Some(0));
    }
}

fn jump_to_bottom(app: &mut AppState) {
    let (len, state) = match app.active_view {
        View::Profiles => {
            let len = select::visible_profile_indices(app).len();
            (len, Some(&mut app.profile_list_state))
        }
        View::Vars => {
            let len = select::compute_var_rows(app).len();
            (len, Some(&mut app.vars_list_state))
        }
        View::Parts => {
            let var = app
                .selected_var_name
                .clone()
                .unwrap_or_else(|| "PATH".to_string());
            let parts = select::current_var_parts(app, &var);
            let len = select::visible_part_indices(app, &parts).len();
            (len, Some(&mut app.parts_list_state))
        }
        View::Items => {
            let len = select::visible_item_indices(app).len();
            (len, Some(&mut app.items_list_state))
        }
        View::Defs => {
            let mut defs = app.var_options.clone();
            defs.sort_by(|a, b| a.name.cmp(&b.name));
            if !app.defs_filter.is_empty() {
                let q = app.defs_filter.to_lowercase();
                defs.retain(|d| d.name.to_lowercase().contains(&q));
            }
            (defs.len(), Some(&mut app.defs_list_state))
        }
        View::Preview | View::Export | View::Help => (0, None),
    };

    if let Some(state) = state {
        if len > 0 {
            state.select(Some(len - 1));
        }
    }
}
