mod action;
mod app;
mod event;
mod input;
mod theme;
mod ui;

use std::io;

use clap::{Parser, Subcommand};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;

use wdttg_core::config::{config_path, data_dir, load_or_create_default};
use wdttg_core::storage::file_manager::FileManager;

use crate::app::App;

/// wdttg - Where Did The Time Go?
/// A terminal time tracker for freelancers.
#[derive(Parser)]
#[command(name = "wdttg", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the MCP server on stdio transport for AI agent integration.
    Serve,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Serve) => run_mcp_server()?,
        None => run_tui()?,
    }

    Ok(())
}

fn run_tui() -> color_eyre::Result<()> {
    let first_run = config_path().map(|p| !p.exists()).unwrap_or(false);
    let config = load_or_create_default()?;

    let mut terminal = setup_terminal()?;
    let result = App::new(config, first_run).run(&mut terminal);
    restore_terminal()?;

    result
}

fn run_mcp_server() -> color_eyre::Result<()> {
    let config = load_or_create_default()?;
    let data = data_dir(&config)?;
    let file_manager = FileManager::new(data);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        wdttg_mcp::run_server(config, file_manager)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("{e}"))
    })
}

fn setup_terminal() -> color_eyre::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> color_eyre::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
