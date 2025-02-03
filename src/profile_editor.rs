use anyhow::Result;
use crossterm::event::{poll, read, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::time::Duration;

pub fn edit_profile_name_dialog<B: Backend>(
    terminal: &mut Terminal<B>,
    initial: Option<&str>,
) -> Result<Option<String>> {
    let mut name = initial.unwrap_or("").to_string();
    loop {
        terminal.draw(|f| {
            let area = centered_rect(50, 20, f.size());
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Profile Name (Enter: confirm, Esc: cancel)");
            let paragraph = Paragraph::new(format!("Profile Name: {}", name))
                .block(block)
                .style(Style::default());
            f.render_widget(paragraph, area);
        })?;
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = read()? {
                match key.code {
                    KeyCode::Enter => return Ok(Some(name)),
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Backspace => {
                        name.pop();
                    }
                    KeyCode::Char(c) => {
                        name.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
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
        .direction(ratatui::layout::Direction::Horizontal)
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

/// Displays a confirmation popup with the given message.
/// Returns true if the user presses Y, false if N or Esc.
pub fn confirm_dialog<B: Backend>(terminal: &mut Terminal<B>, message: &str) -> Result<bool> {
    loop {
        terminal.draw(|f| {
            let area = centered_rect(50, 20, f.size());
            let block = Block::default().borders(Borders::ALL).title("Confirmation");
            let paragraph = Paragraph::new(format!(
                "{}\n\nPress Y to confirm, N or Esc to cancel",
                message
            ))
            .block(block)
            .style(Style::default());
            f.render_widget(paragraph, area);
        })?;
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => return Ok(false),
                    _ => {}
                }
            }
        }
    }
}
