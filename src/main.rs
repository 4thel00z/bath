// src/main.rs

use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;
mod db;
mod export;
mod tui_app;
mod profile_editor;

#[derive(Parser, Debug)]
#[command(
    name = "gcc-env-manager",
    about = "Manage GCC-related environment variable configurations"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Export a profile as export commands (which you can eval in your shell)
    Export {
        /// Profile name to export. If omitted, an interactive view lets you select one.
        profile: Option<String>,

        /// Operation mode: prepend, append, or replace (default is prepend)
        #[arg(
            short,
            long,
            default_value = "prepend",
            value_parser = ["prepend", "append", "replace"]
        )]
        mode: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Export { profile, mode }) => {
            let op_mode = match mode.as_str() {
                "prepend" => export::OperationMode::Prepend,
                "append" => export::OperationMode::Append,
                "replace" => export::OperationMode::Replace,
                _ => export::OperationMode::Prepend, // fallback
            };
            if let Some(profile_name) = profile {
                export::export_profile(&profile_name, op_mode)?;
            } else {
                // Launch interactive export selection if no profile was provided.
                export::interactive_export(op_mode)?;
            }
        }
        None => {
            // If no subcommand is provided, run the interactive TUI.
            tui_app::run()?;
        }
    }

    Ok(())
}
