use crate::export;
use crate::tui::select;
use crate::tui::state::AppState;
use crate::tui::view::View;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &AppState) {
    let text = match app.active_view {
        View::Profiles => details_profiles(app),
        View::Vars => details_vars(app),
        View::Parts => details_parts(app),
        View::Items => details_items(app),
        View::Defs => details_defs(app),
        View::Preview => details_vars(app),
        View::Export => details_vars(app),
        View::Help => {
            "Use :profiles, :vars, :parts, :items, :defs\nUse / to filter the current view.\n"
                .to_string()
        }
    };

    let p = Paragraph::new(text).style(app.theme.text()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(app.theme.border())
            .title("Details"),
    );
    f.render_widget(p, area);
}

fn details_profiles(app: &AppState) -> String {
    let selected = select::selected_profile_index(app).unwrap_or(app.active_profile_index);
    let p = app.profiles.get(selected);
    let Some(p) = p else {
        return "No profiles.".to_string();
    };

    let full = export::generate_full_export(p, export::OperationMode::Prepend);
    let preview = full.lines().take(12).collect::<Vec<_>>().join("\n");

    format!(
        "Profile: {}\nEntries: {}\n\nExport (first lines):\n{}",
        p.name,
        p.entries.len(),
        preview
    )
}

fn details_vars(app: &AppState) -> String {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let parts = select::current_var_parts(app, &var);
    let sep = app
        .var_options
        .iter()
        .find(|o| o.name == var)
        .map(|o| o.separator.clone())
        .unwrap_or_else(|| ":".to_string());
    let joined = parts
        .iter()
        .map(select::preview_value)
        .collect::<Vec<_>>()
        .join(&sep);
    let profile = &app.profiles[app.active_profile_index];
    let export_all = export::generate_full_export(profile, export::OperationMode::Prepend);
    let export_line = export_all
        .lines()
        .find(|l| l.starts_with(&format!("export {var}=")))
        .unwrap_or("");

    format!(
        "Var: {var}\nParts: {}\nSeparator: '{}'\n\nPreview:\n{joined}\n\nExport:\n{export_line}\n",
        parts.len(),
        sep
    )
}

fn details_parts(app: &AppState) -> String {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let parts = select::current_var_parts(app, &var);
    let visible = select::visible_part_indices(app, &parts);
    let selected = app
        .parts_list_state
        .selected()
        .and_then(|i| visible.get(i).copied());

    let Some(idx) = selected else {
        return format!("Var: {var}\n(no selected part)\n");
    };

    let entry = parts.get(idx);
    let Some(entry) = entry else {
        return format!("Var: {var}\n(no selected part)\n");
    };

    format!(
        "Var: {var}\nIndex: {idx}\nEntry:\n{}\n\nValue:\n{}\n",
        entry,
        select::preview_value(entry)
    )
}

fn details_items(app: &AppState) -> String {
    let idx = select::selected_item_index(app);
    let Some(i) = idx else {
        return "No item selected.".to_string();
    };
    let Some(it) = app.items.get(i) else {
        return "No item selected.".to_string();
    };

    format!(
        "Kind: {:?}\nValue: {}\nProgram: {}\nVersion: {}\nTags: {}\n",
        it.kind,
        it.value,
        it.program.clone().unwrap_or_default(),
        it.version.clone().unwrap_or_default(),
        if it.tags.is_empty() {
            String::new()
        } else {
            it.tags.join(", ")
        }
    )
}

fn details_defs(app: &AppState) -> String {
    // Mirror the same ordering/filtering logic as the main list.
    let mut defs = app.var_options.clone();
    defs.sort_by(|a, b| a.name.cmp(&b.name));
    if !app.defs_filter.is_empty() {
        let q = app.defs_filter.to_lowercase();
        defs.retain(|d| d.name.to_lowercase().contains(&q));
    }

    let sel = app.defs_list_state.selected().unwrap_or(0);
    let Some(def) = defs.get(sel) else {
        return "No var definition selected.".to_string();
    };

    format!(
        "Name: {}\nKind: {:?}\nSeparator: '{}'\nEditor: {:?}\n",
        def.name, def.kind, def.separator, def.editor
    )
}
