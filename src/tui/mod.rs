pub mod app;
pub mod commands;
pub mod daisyui_themes;
pub mod dialogs;
pub mod editor;
pub mod input;
pub mod select;
pub mod state;
pub mod theme;
pub mod ui;
pub mod util;
pub mod view;

pub fn run() -> anyhow::Result<()> {
    app::run()
}
