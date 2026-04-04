# wdttg - Where Did The Time Go?

A terminal time tracker for freelancers, built in Rust.

Ported from the [where-did-the-time-go](https://github.com/plattnum/where-did-the-time-go) Obsidian plugin.

## Features

- TUI for interactive time tracking with a vertical timeline view
- MCP server for AI agent integration (Claude Code, etc.)
- GFM markdown storage (one file per month in `~/.local/share/wdttg/data/`)
- Client/project/activity hierarchy with billable rates
- Overlap detection, time snapping, and report generation
- File watching for real-time sync between TUI and MCP

## Install

```bash
cargo install --path crates/wdttg-tui
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/wdttg
```

## Usage

### TUI

```bash
wdttg
```

On first run, creates `~/.config/wdttg/config.toml` with a default "Personal" client and `~/.local/share/wdttg/data/` for time entries.

### MCP Server

```bash
wdttg serve
```

Starts the MCP server on stdio transport. The server exposes time tracking operations as MCP tools for AI agents.

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
| `list_entries` | List time entries with date range, presets, and filters |
| `get_entry` | Get a single entry by ID or start/end timestamps |
| `create_entry` | Create a new time entry with validation and overlap checking |
| `update_entry` | Update an entry (supports partial updates) |
| `delete_entry` | Delete an entry by ID or timestamps |
| `check_overlaps` | Check if a proposed time range conflicts with existing entries |
| `generate_report` | Aggregated report by client/project/activity with billable amounts |
| `available_slots` | Find free time gaps within a datetime range |
| `list_clients` | List configured clients with projects and activities |
| `get_status` | Today/week totals, entry counts, config summary |

### Testing with MCP Inspector

Use the [MCP Inspector](https://modelcontextprotocol.io/docs/tools/inspector) to interactively test and debug the server:

```bash
# From source
npx -y @modelcontextprotocol/inspector cargo run -p wdttg-tui -- serve

# Or using the installed binary
npx -y @modelcontextprotocol/inspector wdttg serve
```

This opens a web UI (usually `http://localhost:6274`) where you can browse available tools, invoke them with custom inputs, and inspect results.

### How it works

The MCP server and TUI share the same data files in `~/.local/share/wdttg/data/`. The TUI watches for file changes, so entries created by an AI agent via MCP appear in the TUI within ~200ms.

File locking (advisory locks via `fs2`) prevents data corruption when both are writing simultaneously.

## Configuration

`~/.config/wdttg/config.toml` defines clients, projects, activities, and preferences.

```toml
[preferences]
time_format = "24h"
week_start = "monday"
day_start_hour = 6
day_end_hour = 22
snap_minutes = 15

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

## Workspace

| Crate | Purpose |
|-------|---------|
| `wdttg-core` | Library: parsing, validation, storage, reporting |
| `wdttg-tui` | Binary: terminal UI (ratatui + crossterm) |
| `wdttg-mcp` | Library: MCP server tool implementations |

## License

Private project.
