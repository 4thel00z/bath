use crate::config::{CatalogItem, CustomVarDef, ItemKind, VarKind};
use crate::tui::util::centered_rect;
use anyhow::Result;
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    backend::Backend,
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};

pub fn create_custom_var_dialog<B: Backend>(
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

pub fn create_or_edit_item_dialog<B: Backend>(
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
                        if kind == ItemKind::Text && matches!(field, Field::Program | Field::Version)
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

