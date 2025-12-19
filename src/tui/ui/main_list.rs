use crate::config::ItemKind;
use crate::tui::select;
use crate::tui::state::AppState;
use crate::tui::view::View;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    match app.active_view {
        View::Profiles => draw_profiles(f, area, app),
        View::Vars => draw_vars(f, area, app),
        View::Parts => draw_parts(f, area, app),
        View::Items => draw_items(f, area, app),
        View::Defs => draw_defs(f, area, app),
        View::Preview => draw_preview(f, area, app),
        View::Export => draw_export(f, area, app),
        View::Help => draw_help(f, area, app),
    }
}

fn draw_profiles<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    let indices = select::visible_profile_indices(app);
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

    select::clamp_list_state(&mut app.profile_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Profiles"),
        )
        .style(app.theme.text())
        .highlight_style(app.theme.list_highlight())
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.profile_list_state);
}

fn draw_vars<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &mut AppState) {
    let rows = select::compute_var_rows(app);
    let items: Vec<ListItem> = rows
        .iter()
        .map(|r| {
            let badge = match r.kind {
                crate::config::VarKind::Scalar => "S",
                crate::config::VarKind::List => "L",
            };
            ListItem::new(format!(
                "{}  {:<18}  {:>3}  sep='{}'",
                badge, r.name, r.count, r.separator
            ))
        })
        .collect();

    select::clamp_list_state(&mut app.vars_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Vars"),
        )
        .style(app.theme.text())
        .highlight_style(app.theme.list_highlight())
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.vars_list_state);
}

fn draw_parts<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let parts = select::current_var_parts(app, &var);
    let indices = select::visible_part_indices(app, &parts);
    let items: Vec<ListItem> = indices
        .iter()
        .map(|i| ListItem::new(parts[*i].to_string()))
        .collect();

    select::clamp_list_state(&mut app.parts_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title(format!("Parts for {var}")),
        )
        .style(app.theme.text())
        .highlight_style(app.theme.list_highlight())
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.parts_list_state);
}

fn draw_items<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    let indices = select::visible_item_indices(app);
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

    select::clamp_list_state(&mut app.items_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Items"),
        )
        .style(app.theme.text())
        .highlight_style(app.theme.list_highlight())
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.items_list_state);
}

fn draw_defs<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    let mut defs = app.var_options.clone();
    defs.sort_by(|a, b| a.name.cmp(&b.name));
    if !app.defs_filter.is_empty() {
        let q = app.defs_filter.to_lowercase();
        defs.retain(|d| d.name.to_lowercase().contains(&q));
    }
    let items: Vec<ListItem> = defs
        .iter()
        .map(|d| {
            let kind = match d.kind {
                crate::config::VarKind::Scalar => "Scalar",
                crate::config::VarKind::List => "List",
            };
            ListItem::new(format!("{:<18}  {:<6}  sep='{}'", d.name, kind, d.separator))
        })
        .collect();

    select::clamp_list_state(&mut app.defs_list_state, items.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Var defs"),
        )
        .style(app.theme.text())
        .highlight_style(app.theme.list_highlight())
        .highlight_symbol("» ");
    f.render_stateful_widget(list, area, &mut app.defs_list_state);
}

fn draw_preview<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
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
    let text = format!("{var} = {joined}\n\n(parts: {})", parts.len());
    let p = Paragraph::new(text)
        .style(app.theme.text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Preview"),
        );
    f.render_widget(p, area);
}

fn draw_export<B: ratatui::backend::Backend>(
    f: &mut ratatui::Frame<B>,
    area: Rect,
    app: &mut AppState,
) {
    let var = app
        .selected_var_name
        .clone()
        .unwrap_or_else(|| "PATH".to_string());
    let profile = &app.profiles[app.active_profile_index];
    let full = crate::export::generate_full_export(profile, crate::export::OperationMode::Prepend);
    let line = full
        .lines()
        .find(|l| l.starts_with(&format!("export {var}=")))
        .unwrap_or("");
    let p = Paragraph::new(line.to_string())
        .style(app.theme.text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Export"),
        );
    f.render_widget(p, area);
}

fn draw_help<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &AppState) {
    let mut theme_names = crate::tui::daisyui_themes::names();
    theme_names.sort();
    let themes_block = theme_names
        .chunks(6)
        .map(|chunk| chunk.join("  "))
        .collect::<Vec<_>>()
        .join("\n");

    let text = format!(
        r#"Navigation
  :  command palette (jump to views, actions)
  /  filter current view
  Tab  cycle views
  q  quit

Views
  profiles  vars  parts  items  defs  preview  export  help

Theme
  current: {current}
  preset:  {preset}
  :themes (count: {count})
  :theme <name>

Available DaisyUI themes:
{themes}

Notes
  Items can be picked with m and dropped with p (in Items/Vars/Parts depending on context).
"#,
        current = app.theme.name,
        preset = app.theme_preset,
        count = crate::tui::daisyui_themes::THEMES.len(),
        themes = themes_block
    );
    let p = Paragraph::new(text)
        .style(app.theme.text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(app.theme.border())
                .title("Help"),
        );
    f.render_widget(p, area);
}

