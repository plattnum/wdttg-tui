# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For releases prior to `0.5.0`, see the [GitHub Releases page](https://github.com/plattnum/wdttg-tui/releases).

## [0.5.0] - 2026-05-09

### Added

- **Right-side tag rendering for short entries.** A 15-minute timeline entry
  (one visible row) used to clip its tag chips off the bottom of the card.
  The block now narrows to fit just the time + duration, and the tag chips
  render in the grid space immediately to the right. Trailing chips drop
  cleanly if the row is too narrow to fit them all.
- **Configurable tag chip style** via a new `tag_style` preference in
  `~/.config/wdttg/config.toml`. Values:
  - `flat` (default) — original space-separated chips. Works on any terminal.
  - `powerline` — Powerline chevron glyphs (U+E0B0) that blend chip colors.
    **Requires a Powerline / Nerd Font** in your terminal — without one,
    the chevrons render as `?` boxes.
  - `triangle` — uses U+25B6 between chips. Works on any Unicode terminal
    but doesn't blend as smoothly as the Powerline glyph.
- **`T` key** in the TUI cycles through `flat → powerline → triangle` at
  runtime and persists the choice to `config.toml`, so it survives restarts.
  Also visible in the timeline header hints and the `?` help screen.

### Changed

- The config-file watcher now also reloads when `tag_style` is edited
  externally (previously it would skip the reload).
