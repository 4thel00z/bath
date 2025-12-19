pub mod detail;
pub mod header;
pub mod main_list;

use crate::tui::state::{AppState, InputMode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub fn draw_main_ui<B: Backend>(f: &mut ratatui::Frame<B>, app: &mut AppState) {
    let size = f.size();
    // Paint full background so terminal default doesn't bleed through.
    f.render_widget(Block::default().style(app.theme.background()), size);

    let header_h = 3u16.min(size.height);
    let mut detail_h = (size.height / 3).max(7);
    detail_h = detail_h.min(size.height.saturating_sub(header_h));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_h),
            Constraint::Min(0),
            Constraint::Length(detail_h),
        ])
        .split(size);

    header::draw(f, chunks[0], app);
    main_list::draw(f, chunks[1], app);
    detail::draw(f, chunks[2], app);

    draw_overlays(f, size, app);
}

fn draw_overlays<B: Backend>(f: &mut ratatui::Frame<B>, size: Rect, app: &mut AppState) {
    if matches!(app.input_mode, InputMode::Command) {
        let sugg = app.command_suggestions.len().min(8);
        let overlay_h = (sugg + 2) as u16; // prompt + suggestions
        let overlay_h = overlay_h.min(size.height);
        let overlay = Rect {
            x: size.x,
            y: size.y + size.height.saturating_sub(overlay_h),
            width: size.width,
            height: overlay_h,
        };

        f.render_widget(Clear, overlay);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
            .split(overlay);

        let prompt = Paragraph::new(format!(":{}", app.command_input))
            .style(app.theme.text())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.theme.border())
                    .title("Command"),
            );
        f.render_widget(prompt, chunks[0]);

        let items: Vec<ListItem> = app
            .command_suggestions
            .iter()
            .take(sugg)
            .map(|s| ListItem::new(s.clone()))
            .collect();

        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(app.command_selected.min(items.len().saturating_sub(1))));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.theme.border())
                    .title("Suggestions (Tab to complete)"),
            )
            .style(app.theme.text())
            .highlight_style(app.theme.list_highlight())
            .highlight_symbol("» ");

        f.render_stateful_widget(list, chunks[1], &mut state);
    } else if matches!(app.input_mode, InputMode::Search) {
        let overlay_h = 3u16.min(size.height);
        let overlay = Rect {
            x: size.x,
            y: size.y + size.height.saturating_sub(overlay_h),
            width: size.width,
            height: overlay_h,
        };
        f.render_widget(Clear, overlay);

        let prompt = Paragraph::new(format!("/{}", app.command_input))
            .style(app.theme.text())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(app.theme.border())
                    .title("Filter (Enter: apply, Esc: cancel)"),
            );
        f.render_widget(prompt, overlay);
    }
}
