#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Profiles,
    Vars,
    Parts,
    Items,
    Defs,
    Preview,
    Export,
    Help,
}

impl View {
    pub fn title(self) -> &'static str {
        match self {
            View::Profiles => "Profiles",
            View::Vars => "Vars",
            View::Parts => "Parts",
            View::Items => "Items",
            View::Defs => "Defs",
            View::Preview => "Preview",
            View::Export => "Export",
            View::Help => "Help",
        }
    }

    pub fn is_filterable(self) -> bool {
        matches!(
            self,
            View::Profiles | View::Vars | View::Parts | View::Items | View::Defs
        )
    }
}

