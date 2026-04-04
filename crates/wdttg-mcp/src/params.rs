use serde::Deserialize;

/// Parameters for the `list_entries` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListEntriesParams {
    /// Start date (YYYY-MM-DD). Required unless preset is provided.
    pub start_date: Option<String>,
    /// End date (YYYY-MM-DD). Required unless preset is provided.
    pub end_date: Option<String>,
    /// Preset date range: today, yesterday, this_week, last_week, this_month, last_month.
    pub preset: Option<String>,
    /// Filter by client ID.
    pub client: Option<String>,
    /// Filter by project ID.
    pub project: Option<String>,
    /// Filter by activity ID.
    pub activity: Option<String>,
    /// Filter by description text (case-insensitive substring match).
    pub description_contains: Option<String>,
    /// Maximum number of entries to return.
    pub limit: Option<usize>,
    /// Number of entries to skip (for pagination).
    pub offset: Option<usize>,
}

/// Parameters for the `create_entry` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateEntryParams {
    /// Start datetime (YYYY-MM-DD HH:mm).
    pub start: String,
    /// End datetime (YYYY-MM-DD HH:mm).
    pub end: String,
    /// Description of the work done.
    pub description: String,
    /// Client ID (must exist in config).
    pub client: String,
    /// Project ID (must exist under the client in config).
    pub project: Option<String>,
    /// Activity ID (must exist under the client in config).
    pub activity: Option<String>,
    /// Additional notes.
    pub notes: Option<String>,
    /// Snap start/end to the configured time grid.
    pub snap_to_grid: Option<bool>,
}

/// Parameters for the `update_entry` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateEntryParams {
    /// Identify entry by computed ID (e_XXXXXXXX).
    pub entry_id: Option<String>,
    /// Identify entry by original start datetime (YYYY-MM-DD HH:mm).
    pub original_start: Option<String>,
    /// Identify entry by original end datetime (YYYY-MM-DD HH:mm).
    pub original_end: Option<String>,
    /// New start datetime.
    pub start: Option<String>,
    /// New end datetime.
    pub end: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New client ID.
    pub client: Option<String>,
    /// New project ID.
    pub project: Option<String>,
    /// New activity ID.
    pub activity: Option<String>,
    /// New notes.
    pub notes: Option<String>,
}

/// Parameters for the `delete_entry` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteEntryParams {
    /// Identify entry by computed ID (e_XXXXXXXX).
    pub entry_id: Option<String>,
    /// Identify entry by start datetime (YYYY-MM-DD HH:mm).
    pub start: Option<String>,
    /// Identify entry by end datetime (YYYY-MM-DD HH:mm).
    pub end: Option<String>,
}

/// Parameters for the `get_entry` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetEntryParams {
    /// Identify entry by computed ID (e_XXXXXXXX).
    pub entry_id: Option<String>,
    /// Identify entry by start datetime (YYYY-MM-DD HH:mm).
    pub start: Option<String>,
    /// Identify entry by end datetime (YYYY-MM-DD HH:mm).
    pub end: Option<String>,
}

/// Parameters for the `check_overlaps` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckOverlapsParams {
    /// Start datetime to check (YYYY-MM-DD HH:mm).
    pub start: String,
    /// End datetime to check (YYYY-MM-DD HH:mm).
    pub end: String,
    /// Entry ID to exclude from overlap check (for edit scenarios).
    pub exclude_entry_id: Option<String>,
}

/// Parameters for the `generate_report` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateReportParams {
    /// Start date (YYYY-MM-DD). Required unless preset is provided.
    pub start_date: Option<String>,
    /// End date (YYYY-MM-DD). Required unless preset is provided.
    pub end_date: Option<String>,
    /// Preset date range: today, yesterday, this_week, last_week, this_month, last_month.
    pub preset: Option<String>,
    /// Filter by client ID.
    pub client: Option<String>,
    /// Filter by project ID.
    pub project: Option<String>,
}

/// Parameters for the `available_slots` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AvailableSlotsParams {
    /// Start datetime (YYYY-MM-DD HH:mm) or start date (YYYY-MM-DD) of the range to check.
    pub start: String,
    /// End datetime (YYYY-MM-DD HH:mm) or end date (YYYY-MM-DD) of the range to check.
    pub end: String,
    /// Minimum slot duration in minutes. Slots shorter than this are excluded. Default: 0 (all slots).
    pub min_duration_minutes: Option<i64>,
    /// Filter: only consider entries from this client as "occupied" time.
    pub client: Option<String>,
    /// Filter: only consider entries from this project as "occupied" time.
    pub project: Option<String>,
    /// Filter: only consider entries with this activity as "occupied" time.
    pub activity: Option<String>,
}

/// Parameters for the `list_clients` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListClientsParams {
    /// Include archived clients in results.
    pub include_archived: Option<bool>,
}

/// Parameters for the `create_client` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateClientParams {
    /// Unique client ID (lowercase, no spaces).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Hex color (e.g. "#FF6B6B").
    pub color: String,
    /// Hourly rate. Use 0.0 for non-billable.
    pub rate: f64,
    /// Currency code (defaults to "USD").
    pub currency: Option<String>,
    /// Client address.
    pub address: Option<String>,
    /// Client email.
    pub email: Option<String>,
    /// Tax ID.
    pub tax_id: Option<String>,
    /// Payment terms (e.g. "Net 30").
    pub payment_terms: Option<String>,
    /// Notes about the client.
    pub notes: Option<String>,
}

/// Parameters for the `update_client` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateClientParams {
    /// Client ID to update.
    pub id: String,
    /// New display name.
    pub name: Option<String>,
    /// New hex color.
    pub color: Option<String>,
    /// New hourly rate.
    pub rate: Option<f64>,
    /// New currency code.
    pub currency: Option<String>,
    /// New address.
    pub address: Option<String>,
    /// New email.
    pub email: Option<String>,
    /// New tax ID.
    pub tax_id: Option<String>,
    /// New payment terms.
    pub payment_terms: Option<String>,
    /// New notes.
    pub notes: Option<String>,
}

/// Parameters for the `archive_client` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveClientParams {
    /// Client ID.
    pub id: String,
    /// Set to true to archive, false to unarchive.
    pub archived: bool,
}

/// Parameters for the `create_project` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateProjectParams {
    /// Client ID that owns this project.
    pub client_id: String,
    /// Unique project ID within the client (lowercase, no spaces).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Hex color.
    pub color: String,
    /// Override the client's hourly rate for this project.
    pub rate_override: Option<f64>,
}

/// Parameters for the `update_project` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateProjectParams {
    /// Client ID that owns this project.
    pub client_id: String,
    /// Project ID to update.
    pub id: String,
    /// New display name.
    pub name: Option<String>,
    /// New hex color.
    pub color: Option<String>,
    /// New rate override (use null to clear).
    pub rate_override: Option<f64>,
}

/// Parameters for the `archive_project` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveProjectParams {
    /// Client ID that owns this project.
    pub client_id: String,
    /// Project ID.
    pub id: String,
    /// Set to true to archive, false to unarchive.
    pub archived: bool,
}

/// Parameters for the `create_activity` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateActivityParams {
    /// Client ID that owns this activity.
    pub client_id: String,
    /// Unique activity ID within the client (lowercase, no spaces).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Hex color.
    pub color: String,
}

/// Parameters for the `update_activity` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateActivityParams {
    /// Client ID that owns this activity.
    pub client_id: String,
    /// Activity ID to update.
    pub id: String,
    /// New display name.
    pub name: Option<String>,
    /// New hex color.
    pub color: Option<String>,
}

/// Parameters for the `archive_activity` tool.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ArchiveActivityParams {
    /// Client ID that owns this activity.
    pub client_id: String,
    /// Activity ID.
    pub id: String,
    /// Set to true to archive, false to unarchive.
    pub archived: bool,
}
