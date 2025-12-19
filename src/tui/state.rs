use crate::config::{CatalogItem, CustomVarDef, Entry, EnvProfile, VarKind};
use crate::db;
use crate::tui::theme::{BathConfig, Theme};
use crate::tui::view::View;
use anyhow::Result;
use ratatui::widgets::ListState;
use rusqlite::Connection;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

pub fn builtin_var_options() -> Vec<VarTypeOption> {
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
    pub active_view: View,
    pub input_mode: InputMode,
    pub theme_preset: String,
    pub theme: Theme,
    pub config: BathConfig,

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
    pub search_target: View,

    pub status: String,
    pub holding: Option<Holding>,

    pub items: Vec<CatalogItem>,
}

impl AppState {
    pub fn new() -> Result<Self> {
        let conn = db::establish_connection()?;
        let config = crate::tui::theme::load_config().unwrap_or_default();
        let (theme, theme_preset) = crate::tui::theme::resolve_from_config(&config)
            .unwrap_or_else(|_| {
                let theme = crate::tui::theme::resolve_theme(
                    crate::tui::theme::default_preset(),
                    None,
                )
                .unwrap();
                (theme, crate::tui::theme::default_preset().to_string())
            });
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

            active_view: View::Vars,
            input_mode: InputMode::Normal,
            theme_preset,
            theme,
            config,

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
            search_target: View::Vars,

            status: String::new(),
            holding: None,

            items: Vec::new(),
        };
        app.refresh_var_options()?;
        app.refresh_items()?;
        app.ensure_selected_var();
        Ok(app)
    }

    pub fn set_theme_preset(&mut self, preset: &str, persist: bool) -> Result<()> {
        let preset = preset.trim();
        if preset.is_empty() {
            return Ok(());
        }

        self.theme_preset = preset.to_string();
        self.config
            .theme
            .get_or_insert_with(Default::default)
            .preset = Some(self.theme_preset.clone());

        self.theme = crate::tui::theme::resolve_theme(preset, self.config.theme.as_ref())?;

        if persist {
            crate::tui::theme::save_config(&self.config)?;
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

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
            active_view: View::Vars,
            input_mode: InputMode::Normal,
            theme_preset: crate::tui::theme::default_preset().to_string(),
            theme: crate::tui::theme::resolve_theme(crate::tui::theme::default_preset(), None)?,
            config: BathConfig::default(),
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
            search_target: View::Vars,
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

