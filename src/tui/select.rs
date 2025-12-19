use crate::config::{Entry, PathEntry, VarKind};
use crate::tui::state::{AppState, EditorStyle, VarTypeOption};
use ratatui::widgets::ListState;

#[derive(Clone)]
pub struct VarRow {
    pub name: String,
    pub kind: VarKind,
    pub separator: String,
    pub count: usize,
}

pub fn clamp_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let selected = state.selected().unwrap_or(0);
    let next = selected.min(len.saturating_sub(1));
    state.select(Some(next));
}

pub fn compute_var_rows(app: &AppState) -> Vec<VarRow> {
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

pub fn current_var_parts(app: &AppState, var_name: &str) -> Vec<Entry> {
    let profile = &app.profiles[app.active_profile_index];
    profile
        .entries
        .iter()
        .filter(|e| e.var_name().as_ref() == var_name)
        .cloned()
        .collect()
}

pub fn visible_part_indices(app: &AppState, parts: &[Entry]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..parts.len()).collect();
    if !app.parts_filter.is_empty() {
        let q = app.parts_filter.to_lowercase();
        indices.retain(|i| parts[*i].to_string().to_lowercase().contains(&q));
    }
    indices
}

pub fn visible_item_indices(app: &AppState) -> Vec<usize> {
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

pub fn visible_profile_indices(app: &AppState) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..app.profiles.len()).collect();
    if !app.profiles_filter.is_empty() {
        let q = app.profiles_filter.to_lowercase();
        indices.retain(|i| app.profiles[*i].name.to_lowercase().contains(&q));
    }
    indices
}

pub fn selected_profile_index(app: &AppState) -> Option<usize> {
    let indices = visible_profile_indices(app);
    app.profile_list_state
        .selected()
        .and_then(|i| indices.get(i).copied())
}

pub fn selected_item_index(app: &AppState) -> Option<usize> {
    let indices = visible_item_indices(app);
    app.items_list_state
        .selected()
        .and_then(|i| indices.get(i).copied())
}

pub fn preview_value(e: &Entry) -> String {
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

pub fn make_part_entry(app: &AppState, var_name: &str, value: String) -> Option<Entry> {
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

pub fn var_option_for(app: &AppState, var_name: &str) -> VarTypeOption {
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

