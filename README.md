# wdttg - Where Did The Time Go?

A terminal time tracker for freelancers, built in Rust. No cloud, no subscriptions, no app store approval queues. Just markdown files, a TUI, and an MCP server because life's too short for time entry forms.

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/plattnum)

<img src="docs/images/wdttg_1_timeline_screenshot.png" alt="Timeline view" width="75%">

<details>
<summary>More screenshots</summary>

<img src="docs/images/wdttg_2_reports_screenshot.png" alt="Reports view" width="75%">

<img src="docs/images/wdttg_3_manage_screenshot.png" alt="Manage view" width="75%">

</details>

## Why this exists

I built the [where-did-the-time-go](https://github.com/plattnum/where-did-the-time-go) Obsidian plugin to scratch this itch the first time around. It worked great, but I got tired of being tied to a specific application platform. Plugin approval on Obsidian takes forever, and at the end of the day the data format was already just markdown tables.

Markdown is everywhere now — every IDE reads it, every AI agent works well with it. So instead of fighting an approval process, I pulled the concept out into its own thing: a lightweight TUI for visualization and an MCP server so AI agents can do the heavy lifting.

The core idea is dead simple: **I did X between A and B. Categorize it so I can report on it and bill from it.**

This is not a task manager. My tasks live in tickets, ad-hoc journal entries, kanban boards — wherever makes sense at the time. This tool only cares about what you *did* and when. A meeting with a client. A two-hour dev session. A quick call that ran long. Log it, tag it, move on.

## How it works with AI

The MCP server is the real unlock here. Instead of clicking through forms, I just tell Claude what I did:

> "I had a sprint planning meeting with Acme from 9 to 10:30 this morning"

Claude calls the MCP tools, validates the entry, checks for overlaps, and logs it. If I'm too vague, it asks for the missing pieces.

### Why MCP instead of letting the AI edit markdown directly?

Without the MCP server, the AI would have to read the raw markdown into its context, parse the table, mentally compare timestamps to check for overlaps, format a new row correctly, and write the whole file back. Any of those steps can go wrong — the LLM might miscalculate a time comparison, botch the table formatting, or hallucinate that a time slot is free when it isn't. And every entry eats context tokens just to read and rewrite the file.

With the MCP server, `create_entry` handles all of that programmatically — parsing, validation, overlap detection, chronological sorting, atomic file writes. The AI just says "create this entry" and gets back a definitive yes or a structured error. No guesswork, no wasted tokens, no risk of corrupting the file.

The TUI and MCP server share the same data files, so entries created by AI show up in the timeline within ~200ms.

## Features

- Infinite vertical timeline — endless scrolling into the past or future with keyboard and mouse
- MCP server for AI agent integration (Claude Code, Cursor, etc.)
- GFM markdown storage — one file per month, human-readable and editable
- Client/project/activity hierarchy with billable rates
- Overlap detection and time snapping
- Report generation with billable amounts
- File watching for real-time sync between TUI and MCP

## Install

### From GitHub Releases (recommended)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/plattnum/wdttg-tui/releases/latest/download/wdttg-tui-installer.sh | sh
```

### From source

Requires [Rust](https://rustup.rs/) 1.85+ (2024 edition).

```bash
git clone https://github.com/plattnum/wdttg-tui.git
cd wdttg-tui
cargo install --path crates/wdttg-tui
```

## Getting Started

### 1. Initialize

```bash
wdttg init
```

Interactive setup wizard — asks for time format (12h/24h), week start day, data directory, and your first client. Press Enter to accept defaults at each step.

```
Welcome to wdttg! Let's set things up.

Time format [24h/12h] (24h): 
Week start [monday/sunday] (monday): 
Data directory (/Users/you/.local/share/wdttg/data): 

Set up your first client:
  Client name (Personal): Acme Corp
  Hourly rate (0): 150
  Currency (USD): 

✓ Config saved to ~/.config/wdttg/config.toml
✓ Client data saved to ~/.local/share/wdttg/data/clients.toml
```

### 2. Launch the TUI

```bash
wdttg
```

### 3. Set up AI integration (optional)

The MCP server is launched automatically by your AI client — you don't run `wdttg serve` manually. Just add the config and your AI can create/query time entries for you.

See [MCP Server Setup](#mcp-server-setup) below.

### Reinitialize

```bash
wdttg init --force    # resets preferences and client data (keeps time entries)
```

### Upgrading

After installing a new version, **restart any AI clients** (Claude Desktop, Claude Code, Cursor, etc.) that run the MCP server. They cache the old binary — the MCP server won't pick up changes until the AI client restarts and re-launches `wdttg serve`.

```bash
cargo install --path crates/wdttg-tui   # or download new release
# Then restart Claude Desktop / reload Claude Code
```

## MCP Server Setup

### Claude Code

Add to your project's `.mcp.json` (or `~/.claude/mcp.json` for global access):

```json
{
  "mcpServers": {
    "wdttg": {
      "command": "wdttg",
      "args": ["serve"]
    }
  }
}
```

If running from source instead of an installed binary:

```json
{
  "mcpServers": {
    "wdttg": {
      "command": "cargo",
      "args": ["run", "-p", "wdttg-tui", "--", "serve"],
      "cwd": "/path/to/wdttg-tui"
    }
  }
}
```

### Available MCP Tools

| Tool | Description |
|------|-------------|
| **Time entries** | |
| `list_entries` | List time entries with date range, presets, and filters |
| `get_entry` | Get a single entry by ID or start/end timestamps |
| `create_entry` | Create a new time entry with validation and overlap checking |
| `update_entry` | Update an entry (supports partial updates) |
| `delete_entry` | Delete an entry by ID or timestamps |
| `check_overlaps` | Check if a proposed time range conflicts with existing entries |
| `available_slots` | Find free time gaps within a datetime range |
| **Reports** | |
| `generate_report` | Aggregated report by client/project/activity with billable amounts |
| `get_status` | Today/week totals, entry counts, config summary |
| **Clients & projects** | |
| `list_clients` | List clients with their projects and activities |
| `create_client` | Add a new client |
| `update_client` | Update client fields |
| `archive_client` | Archive/unarchive a client (cascades to projects + activities) |
| `create_project` | Add a project under a client |
| `update_project` | Update project fields |
| `archive_project` | Archive/unarchive a project |
| `create_activity` | Add an activity under a client |
| `update_activity` | Update activity fields |
| `archive_activity` | Archive/unarchive an activity |

### Testing with MCP Inspector

Use the [MCP Inspector](https://modelcontextprotocol.io/docs/tools/inspector) to interactively test and debug the server:

```bash
# From source
npx -y @modelcontextprotocol/inspector cargo run -p wdttg-tui -- serve

# Or using the installed binary
npx -y @modelcontextprotocol/inspector wdttg serve
```

This opens a web UI (usually `http://localhost:6274`) where you can browse available tools, invoke them with custom inputs, and inspect results.

## Configuration

Configuration is split into two files:

### Preferences — `~/.config/wdttg/config.toml`

Machine-local settings. Created by `wdttg init`.

```toml
[preferences]
time_format = "24h"
week_start = "monday"
day_start_hour = 6
day_end_hour = 22
snap_minutes = 15
# data_dir = "/custom/path"   # optional, overrides default data location
```

### Client data — `<data_dir>/clients.toml`

Clients, projects, activities, and billing info. Lives alongside time entries so the whole data directory is portable and syncable (git, Dropbox, etc.).

```toml
[bill_from]
name = "Jane Freelancer"
email = "jane@example.com"

[[clients]]
id = "acme"
name = "Acme Corp"
color = "#FF6B6B"
rate = 150.0
currency = "USD"

[[clients.projects]]
id = "webapp"
name = "Web App"
color = "#4ECDC4"

[[clients.activities]]
id = "dev"
name = "Development"
color = "#45B7D1"
```

## Data Format

Time entries are stored as GFM tables in `~/.local/share/wdttg/data/YYYY-MM.md`:

```markdown
# 2026-03

| Start | End | Description | Client | Project | Activity | Notes |
|-------|-----|-------------|--------|---------|----------|-------|
| 2026-03-15 09:00 | 2026-03-15 10:30 | Sprint planning | acme | webapp | meeting | |
| 2026-03-15 11:00 | 2026-03-15 13:00 | API auth refactor | acme | webapp | dev | ticket ACME-412 |
| 2026-03-15 14:00 | 2026-03-15 14:30 | Quick sync with design team | acme | webapp | meeting | |
| 2026-03-15 14:30 | 2026-03-15 17:00 | Frontend dashboard work | acme | webapp | dev | |
```

## Development

```bash
cargo build                    # Build everything
cargo test                     # Run all tests
cargo test -p wdttg-core       # Core library tests
cargo test -p wdttg-mcp        # MCP integration tests
cargo run -p wdttg-tui         # Run the TUI
cargo clippy --all-targets     # Lint
cargo fmt --check              # Check formatting
```

### Workspace

| Crate | Purpose |
|-------|---------|
| `wdttg-core` | Library: parsing, validation, storage, reporting |
| `wdttg-tui` | Binary: terminal UI (ratatui + crossterm) |
| `wdttg-mcp` | Library: MCP server tool implementations |

## Support

If this is useful to you, consider buying me a coffee.

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/plattnum)

## License

MIT - Do whatever you want with it.
