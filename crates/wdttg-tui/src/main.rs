mod action;
mod app;
mod event;
mod input;
mod theme;
mod ui;

use std::io::{self, BufRead, Write as _};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;

use wdttg_core::config::{
    AppConfig, BillFrom, Preferences, config_path, data_dir, load_config, save_clients, save_config,
};
use wdttg_core::model::Client;
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
    /// Initialize wdttg with an interactive setup wizard.
    Init {
        /// Reinitialize preferences (keeps existing data and clients).
        #[arg(long)]
        force: bool,
    },
    /// Export time entries as CSV or aggregated report as JSON.
    Export {
        /// Output format: csv or json.
        #[arg(long, default_value = "csv")]
        format: String,
        /// Date range preset: today, yesterday, this-week, last-week, this-month, last-month.
        #[arg(long)]
        preset: Option<String>,
        /// Start date (YYYY-MM-DD). Used with --end for custom ranges.
        #[arg(long)]
        start: Option<String>,
        /// End date (YYYY-MM-DD). Used with --start for custom ranges.
        #[arg(long)]
        end: Option<String>,
        /// Filter by client ID.
        #[arg(long)]
        client: Option<String>,
    },
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Serve) => run_mcp_server()?,
        Some(Command::Init { force }) => run_init(force)?,
        Some(Command::Export {
            format,
            preset,
            start,
            end,
            client,
        }) => run_export(&format, preset, start, end, client)?,
        None => run_tui()?,
    }

    Ok(())
}

fn require_config() -> color_eyre::Result<AppConfig> {
    match load_config() {
        Ok(config) => Ok(config),
        Err(wdttg_core::Error::NotFound) => {
            eprintln!("wdttg is not initialized. Run 'wdttg init' first.");
            std::process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

fn run_tui() -> color_eyre::Result<()> {
    let config = require_config()?;

    let mut terminal = setup_terminal()?;
    let result = App::new(config, false).run(&mut terminal);
    restore_terminal()?;

    result
}

fn run_export(
    format: &str,
    preset: Option<String>,
    start: Option<String>,
    end: Option<String>,
    client: Option<String>,
) -> color_eyre::Result<()> {
    use chrono::NaiveDate;
    use wdttg_core::model::{DateRange, EntryFilter, TimeRangePreset};
    use wdttg_core::reporting::{entries_to_csv, generate_report, report_to_json};
    use wdttg_core::storage::cache::MonthCache;

    let config = require_config()?;
    let data = data_dir(&config)?;
    let file_manager = FileManager::new(data);
    let mut cache = MonthCache::default();

    let today = chrono::Local::now().date_naive();
    let week_start = &config.preferences.week_start;

    // Resolve date range
    let range = if let Some(ref preset_str) = preset {
        let preset = match preset_str.as_str() {
            "today" => TimeRangePreset::Today,
            "yesterday" => TimeRangePreset::Yesterday,
            "this-week" => TimeRangePreset::ThisWeek,
            "last-week" => TimeRangePreset::LastWeek,
            "this-month" => TimeRangePreset::ThisMonth,
            "last-month" => TimeRangePreset::LastMonth,
            other => {
                eprintln!(
                    "Unknown preset: {other}. Use: today, yesterday, this-week, last-week, this-month, last-month"
                );
                std::process::exit(1);
            }
        };
        DateRange::from_preset(preset, today, week_start)
    } else if let (Some(s), Some(e)) = (&start, &end) {
        let start_date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| color_eyre::eyre::eyre!("Invalid start date: {s}. Use YYYY-MM-DD"))?;
        let end_date = NaiveDate::parse_from_str(e, "%Y-%m-%d")
            .map_err(|_| color_eyre::eyre::eyre!("Invalid end date: {e}. Use YYYY-MM-DD"))?;
        if start_date > end_date {
            return Err(color_eyre::eyre::eyre!(
                "Start date must be before or equal to end date"
            ));
        }
        DateRange::new(start_date, end_date)
    } else if start.is_some() || end.is_some() {
        return Err(color_eyre::eyre::eyre!(
            "--start and --end must both be provided together"
        ));
    } else {
        // Default to this month
        DateRange::from_preset(TimeRangePreset::ThisMonth, today, week_start)
    };

    let filter = EntryFilter {
        client,
        ..Default::default()
    };

    let entries = wdttg_core::storage::load_filtered(&range, &filter, &file_manager, &mut cache)?;

    match format {
        "csv" => print!("{}", entries_to_csv(&entries)),
        "json" => {
            let reports = generate_report(&range, &entries, &config);
            let total_minutes: i64 = reports.iter().map(|r| r.total_minutes).sum();
            println!("{}", report_to_json(&reports, total_minutes));
        }
        other => {
            eprintln!("Unknown format: {other}. Use: csv, json");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn run_mcp_server() -> color_eyre::Result<()> {
    let config = require_config()?;
    let data = data_dir(&config)?;
    let file_manager = FileManager::new(data);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        wdttg_mcp::run_server(config, file_manager)
            .await
            .map_err(|e| color_eyre::eyre::eyre!("{e}"))
    })
}

// --- Init wizard ---

fn run_init(force: bool) -> color_eyre::Result<()> {
    let cfg_path = config_path()?;

    // Check if already initialized
    if cfg_path.exists() && !force {
        let config = load_config()?;
        let data = data_dir(&config)?;
        println!("wdttg is already initialized.");
        println!("  Config: {}", cfg_path.display());
        println!("  Data:   {}", data.display());
        println!();
        println!("Run with --force to reinitialize (keeps existing data, resets preferences).");
        return Ok(());
    }

    if force && cfg_path.exists() {
        let config = load_config()?;
        let data = data_dir(&config)?;
        println!("⚠ This will overwrite your existing configuration:");
        println!("  Config:  {}", cfg_path.display());
        println!("  Clients: {}/clients.toml", data.display());
        println!("  (Time entries will NOT be deleted)");
        println!();
        let confirm = prompt("Continue? [y/N]", "N")?;
        if confirm.to_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    println!("Welcome to wdttg! Let's set things up.");
    println!();

    // Time format
    let time_format = prompt("Time format [24h/12h]", "24h")?;
    let time_format = if time_format == "12h" {
        "12h".to_string()
    } else {
        "24h".to_string()
    };

    // Week start
    let week_start = prompt("Week start [monday/sunday]", "monday")?;
    let week_start = if week_start == "sunday" {
        "sunday".to_string()
    } else {
        "monday".to_string()
    };

    // Data directory
    println!();
    println!("Where should wdttg store your data? (time entries, clients, etc.)");
    println!("Tip: use a Dropbox, Google Drive, or git-synced folder to back up");
    println!("     and share your data across devices.");
    let default_data_dir = default_data_dir_display();
    let data_dir_override = loop {
        let data_input = prompt("Data directory", &default_data_dir)?;
        if data_input == default_data_dir {
            break None;
        }
        let path = PathBuf::from(&data_input);
        if !path.is_absolute() {
            println!("  Path must be absolute (start with /). Try again.");
            continue;
        }
        match std::fs::create_dir_all(&path) {
            Ok(_) => break Some(path),
            Err(e) => {
                println!("  Invalid path: {e}. Try again.");
                continue;
            }
        }
    };

    // Build preferences
    let preferences = Preferences {
        time_format,
        week_start,
        data_dir: data_dir_override,
        ..Preferences::default()
    };

    // Build a temporary config to compute data_dir
    let mut config = AppConfig {
        preferences,
        bill_from: BillFrom::default(),
        clients: vec![],
    };

    // First client
    {
        println!();
        println!("Set up your first client:");
        let client_name = prompt("  Client name", "Personal")?;
        let rate_str = prompt("  Hourly rate", "0")?;
        let rate: f64 = rate_str.parse().unwrap_or(0.0);
        let currency = prompt("  Currency", "USD")?;

        let client = Client {
            id: slugify(&client_name),
            name: client_name,
            color: "#4F46E5".into(),
            rate,
            currency,
            archived: false,
            address: None,
            email: None,
            tax_id: None,
            payment_terms: None,
            notes: None,
            projects: vec![],
            activities: vec![],
        };
        config.clients = vec![client];
    }

    // Create directories
    wdttg_core::config::ensure_directories(&config)?;

    // Save preferences to config.toml
    save_config(&config)?;
    println!();
    println!("✓ Config saved to {}", cfg_path.display());

    // Save clients to clients.toml (only if we have new clients)
    if !config.clients.is_empty() {
        save_clients(&config)?;
        let data = data_dir(&config)?;
        println!("✓ Client data saved to {}/clients.toml", data.display());
    }

    println!();
    println!("Run 'wdttg' to start tracking time!");

    Ok(())
}

fn prompt(label: &str, default: &str) -> color_eyre::Result<String> {
    print!("{label} ({default}): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

fn slugify(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        "client".to_string()
    } else {
        slug
    }
}

fn default_data_dir_display() -> String {
    // Try to get the default XDG data dir for display
    let config = AppConfig {
        preferences: Preferences::default(),
        bill_from: BillFrom::default(),
        clients: vec![],
    };
    data_dir(&config)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "~/.local/share/wdttg/data".into())
}

// --- Terminal setup ---

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
