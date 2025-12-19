use crate::tui::state::AppState;
use crate::tui::view::View;
use ratatui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, area: Rect, app: &AppState) {
    let profile_name = app
        .profiles
        .get(app.active_profile_index)
        .map(|p| p.name.as_str())
        .unwrap_or("<none>");

    let var_name = app
        .selected_var_name
        .as_deref()
        .unwrap_or("<none>");

    let filter = active_filter(app);
    let filter_s = if filter.is_empty() {
        String::new()
    } else {
        format!(" | Filter: {filter}")
    };

    let context = format!(
        "Profile: {profile_name} | View: {} | Var: {var_name}{}",
        app.active_view.title(),
        filter_s
    );

    let hints = view_hints(app.active_view, app);
    let cmd_hints = command_hints();
    let status = if app.status.is_empty() {
        String::new()
    } else {
        format!(" | {}", app.status)
    };
    let line2 = format!("{hints}{status}");

    let text = vec![
        Spans::from(Span::styled(context, app.theme.text())),
        Spans::from(Span::styled(line2, app.theme.dim_text())),
        Spans::from(Span::styled(cmd_hints, app.theme.dim_text())),
    ];

    let p = Paragraph::new(text)
        .style(app.theme.text())
        .block(Block::default().borders(Borders::BOTTOM).border_style(app.theme.border()));
    f.render_widget(p, area);
}

fn active_filter(app: &AppState) -> &str {
    match app.active_view {
        View::Profiles => app.profiles_filter.as_str(),
        View::Vars => app.vars_filter.as_str(),
        View::Parts => app.parts_filter.as_str(),
        View::Items => app.items_filter.as_str(),
        View::Defs => app.defs_filter.as_str(),
        View::Preview | View::Export | View::Help => "",
    }
}

fn view_hints(view: View, _app: &AppState) -> String {
    match view {
        View::Profiles => {
            "A:add E:rename D:del Enter:use  j/k:move  G:top g:bot  /:filter  ::cmd  q:quit"
                .to_string()
        }
        View::Vars => "Enter:parts p:drop-held  j/k:move  G:top g:bot  /:filter  ::cmd  q:quit"
            .to_string(),
        View::Parts => "a:add e:edit d:del y:dup J/K:move m:pick p:drop  j/k:move  G:top g:bot  /:filter  ::cmd  q:quit"
            .to_string(),
        View::Items => "a:add e:edit d:del y:dup m:pick p:drop  j/k:move  G:top g:bot  /:filter  ::cmd  q:quit"
            .to_string(),
        View::Defs => "C:new-var-def  j/k:move  G:top g:bot  /:filter  ::cmd  q:quit".to_string(),
        View::Preview => "Shows preview for selected var  ::cmd  q:quit".to_string(),
        View::Export => "Shows export line for selected var  ::cmd  q:quit".to_string(),
        View::Help => "?:toggle-help  ::cmd  q:quit".to_string(),
    }
}

fn command_hints() -> String {
    // Keep this short-ish so it fits most terminals.
    "Commands: :profiles :vars :parts :items :defs :preview :export :themes :theme <name> :use <profile> :new-var :new-item :quit".to_string()
}

