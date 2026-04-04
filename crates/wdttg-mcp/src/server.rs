use std::sync::{Arc, Mutex, RwLock};

use chrono::Local;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde_json::json;

use wdttg_core::config::{self, AppConfig};
use wdttg_core::model::{Activity, Client, DateRange, EntryFilter, NewEntry, Project};
use wdttg_core::reporting::generate_report;
use wdttg_core::storage::cache::MonthCache;
use wdttg_core::storage::file_manager::FileManager;
use wdttg_core::storage::{self as storage};
use wdttg_core::time_utils::{
    compute_available_slots, find_adjacent, format_duration, snap_to_grid,
};
use wdttg_core::validation::find_overlaps;

use crate::helpers::*;
use crate::params::*;

/// Shared state across MCP tool invocations.
pub struct McpState {
    pub config: RwLock<AppConfig>,
    pub file_manager: FileManager,
    pub cache: Mutex<MonthCache>,
}

/// The wdttg MCP server. Exposes time tracking operations as MCP tools.
#[derive(Clone)]
pub struct WdttgMcpServer {
    tool_router: ToolRouter<Self>,
    pub state: Arc<McpState>,
}

#[tool_router]
impl WdttgMcpServer {
    pub fn new(state: Arc<McpState>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            state,
        }
    }

    // --- Read-only tools ---

    #[tool(
        description = "List time entries within a date range. Supports preset ranges (today, this_week, this_month, etc.) and filtering by client, project, activity, or description text."
    )]
    fn list_entries(&self, Parameters(params): Parameters<ListEntriesParams>) -> String {
        let config = self.state.config.read().unwrap();
        let week_start = &config.preferences.week_start;
        let range = match resolve_date_range(
            &params.start_date,
            &params.end_date,
            &params.preset,
            week_start,
        ) {
            Ok(r) => r,
            Err(e) => return validation_error(&e),
        };

        let filter = EntryFilter {
            client: params.client,
            project: params.project,
            activity: params.activity,
            description_contains: params.description_contains,
        };

        let mut cache = self.state.cache.lock().unwrap();
        let entries =
            match storage::load_filtered(&range, &filter, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            };

        let total_count = entries.len();
        let total_minutes: i64 = entries.iter().map(|e| e.duration_minutes()).sum();

        // Apply pagination
        let offset = params.offset.unwrap_or(0);
        let paginated: Vec<_> = entries
            .iter()
            .skip(offset)
            .take(params.limit.unwrap_or(usize::MAX))
            .collect();

        let entries_json: Vec<_> = paginated.iter().map(|e| entry_to_json(e)).collect();

        json!({
            "entries": entries_json,
            "total_count": total_count,
            "total_minutes": total_minutes,
            "total_formatted": format_duration(total_minutes),
        })
        .to_string()
    }

    #[tool(
        description = "Get a single time entry by its ID or start/end timestamps. Returns the entry with full details plus adjacent entries for context."
    )]
    fn get_entry(&self, Parameters(params): Parameters<GetEntryParams>) -> String {
        let mut cache = self.state.cache.lock().unwrap();

        let entry = if let Some(ref id) = params.entry_id {
            // Search recent months (current + 2 back)
            let today = Local::now().date_naive();
            let range = DateRange::new(
                today - chrono::Months::new(3),
                today + chrono::Months::new(1),
            );
            match storage::find_entry_by_id(id, &range, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            }
        } else if let (Some(start_s), Some(end_s)) = (&params.start, &params.end) {
            let start = match parse_datetime(start_s) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            let end = match parse_datetime(end_s) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            let month_key = start.format("%Y-%m").to_string();
            let entries =
                match storage::load_month(&month_key, &self.state.file_manager, &mut cache) {
                    Ok(e) => e,
                    Err(e) => return error_json(&e),
                };
            match entries
                .into_iter()
                .find(|e| e.start == start && e.end == end)
            {
                Some(e) => e,
                None => return error_json(&wdttg_core::Error::NotFound),
            }
        } else {
            return validation_error(
                "provide either entry_id or both start and end to identify the entry",
            );
        };

        // Get adjacent entries from same month
        let month_key = entry.month_key();
        let month_entries =
            match storage::load_month(&month_key, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            };
        let adjacent = find_adjacent(entry.start, entry.end, &month_entries, Some(&entry));

        json!({
            "entry": entry_to_json(&entry),
            "previous": adjacent.previous.as_ref().map(entry_to_json),
            "next": adjacent.next.as_ref().map(entry_to_json),
        })
        .to_string()
    }

    #[tool(
        description = "Check if a proposed time range overlaps with existing entries. Use before creating or updating entries. Optionally exclude an entry (for edit scenarios)."
    )]
    fn check_overlaps(&self, Parameters(params): Parameters<CheckOverlapsParams>) -> String {
        let start = match parse_datetime(&params.start) {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };
        let end = match parse_datetime(&params.end) {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };

        let mut cache = self.state.cache.lock().unwrap();
        let month_key = start.format("%Y-%m").to_string();
        let entries = match storage::load_month(&month_key, &self.state.file_manager, &mut cache) {
            Ok(e) => e,
            Err(e) => return error_json(&e),
        };

        // If exclude_entry_id provided, find that entry to exclude
        let exclude = params
            .exclude_entry_id
            .as_ref()
            .and_then(|id| entries.iter().find(|e| e.entry_id() == *id));

        let result = find_overlaps(start, end, &entries, exclude);

        let overlaps_json: Vec<_> = result.overlaps.iter().map(overlap_to_json).collect();

        json!({
            "has_overlaps": result.has_overlaps,
            "overlaps": overlaps_json,
        })
        .to_string()
    }

    #[tool(
        description = "Generate an aggregated time report grouped by client, project, and activity. Supports date range presets and filtering."
    )]
    fn generate_report(&self, Parameters(params): Parameters<GenerateReportParams>) -> String {
        let config = self.state.config.read().unwrap();
        let week_start = &config.preferences.week_start;
        let range = match resolve_date_range(
            &params.start_date,
            &params.end_date,
            &params.preset,
            week_start,
        ) {
            Ok(r) => r,
            Err(e) => return validation_error(&e),
        };

        // Apply filters before aggregation
        let filter = EntryFilter {
            client: params.client,
            project: params.project,
            ..Default::default()
        };

        let mut cache = self.state.cache.lock().unwrap();
        let entries =
            match storage::load_filtered(&range, &filter, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            };

        let reports = generate_report(&range, &entries, &config);

        let total_minutes: i64 = reports.iter().map(|r| r.total_minutes).sum();
        let total_billable: f64 = reports.iter().map(|r| r.billable_amount).sum();

        let clients_json: Vec<_> = reports
            .iter()
            .map(|cr| {
                let projects: Vec<_> = cr
                    .project_breakdown
                    .iter()
                    .map(|pr| {
                        let activities: Vec<_> = pr
                            .activity_breakdown
                            .iter()
                            .map(|ar| {
                                json!({
                                    "activity_id": ar.activity_id,
                                    "name": ar.name,
                                    "total_minutes": ar.total_minutes,
                                    "total_formatted": format_duration(ar.total_minutes),
                                    "percentage": (ar.percentage * 10.0).round() / 10.0,
                                })
                            })
                            .collect();
                        json!({
                            "project_id": pr.project_id,
                            "name": pr.name,
                            "total_minutes": pr.total_minutes,
                            "total_formatted": format_duration(pr.total_minutes),
                            "billable_amount": pr.billable_amount,
                            "percentage": (pr.percentage * 10.0).round() / 10.0,
                            "activities": activities,
                        })
                    })
                    .collect();
                json!({
                    "client_id": cr.client_id,
                    "name": cr.name,
                    "rate": cr.rate,
                    "currency": cr.currency,
                    "total_minutes": cr.total_minutes,
                    "total_formatted": format_duration(cr.total_minutes),
                    "billable_amount": cr.billable_amount,
                    "percentage": (cr.percentage * 10.0).round() / 10.0,
                    "projects": projects,
                })
            })
            .collect();

        json!({
            "report": clients_json,
            "total_minutes": total_minutes,
            "total_formatted": format_duration(total_minutes),
            "total_billable": total_billable,
        })
        .to_string()
    }

    #[tool(
        description = "List all configured clients with their projects and activities. Optionally include archived clients."
    )]
    fn list_clients(&self, Parameters(params): Parameters<ListClientsParams>) -> String {
        let config = self.state.config.read().unwrap();
        let include_archived = params.include_archived.unwrap_or(false);

        let clients: Vec<_> = config
            .clients
            .iter()
            .filter(|c| include_archived || !c.archived)
            .map(|c| {
                let projects: Vec<_> = c
                    .projects
                    .iter()
                    .filter(|p| include_archived || !p.archived)
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "name": p.name,
                            "color": p.color,
                            "rate_override": p.rate_override,
                            "archived": p.archived,
                        })
                    })
                    .collect();
                let activities: Vec<_> = c
                    .activities
                    .iter()
                    .filter(|a| include_archived || !a.archived)
                    .map(|a| {
                        json!({
                            "id": a.id,
                            "name": a.name,
                            "color": a.color,
                            "archived": a.archived,
                        })
                    })
                    .collect();
                json!({
                    "id": c.id,
                    "name": c.name,
                    "color": c.color,
                    "rate": c.rate,
                    "currency": c.currency,
                    "archived": c.archived,
                    "projects": projects,
                    "activities": activities,
                })
            })
            .collect();

        json!({ "clients": clients }).to_string()
    }

    #[tool(
        description = "Get current status: today's date, today/this-week totals, entry counts, and configuration summary."
    )]
    fn get_status(&self) -> String {
        let config = self.state.config.read().unwrap();
        let today = Local::now().date_naive();
        let week_start = &config.preferences.week_start;

        let mut cache = self.state.cache.lock().unwrap();

        // Today's entries
        let today_range =
            DateRange::from_preset(wdttg_core::model::TimeRangePreset::Today, today, week_start);
        let today_entries =
            storage::load_date_range(&today_range, &self.state.file_manager, &mut cache)
                .unwrap_or_default();
        let today_minutes: i64 = today_entries.iter().map(|e| e.duration_minutes()).sum();

        // This week's entries
        let week_range = DateRange::from_preset(
            wdttg_core::model::TimeRangePreset::ThisWeek,
            today,
            week_start,
        );
        let week_entries =
            storage::load_date_range(&week_range, &self.state.file_manager, &mut cache)
                .unwrap_or_default();
        let week_minutes: i64 = week_entries.iter().map(|e| e.duration_minutes()).sum();

        json!({
            "today": today.format("%Y-%m-%d").to_string(),
            "today_total_minutes": today_minutes,
            "today_total_formatted": format_duration(today_minutes),
            "today_entry_count": today_entries.len(),
            "this_week_total_minutes": week_minutes,
            "this_week_total_formatted": format_duration(week_minutes),
            "this_week_entry_count": week_entries.len(),
            "config_summary": {
                "data_dir": self.state.file_manager.data_dir().to_string_lossy(),
                "client_count": config.clients.len(),
                "snap_minutes": config.preferences.snap_minutes,
                "time_format": config.preferences.time_format,
                "week_start": config.preferences.week_start,
            },
        })
        .to_string()
    }

    #[tool(
        description = "Find available (unoccupied) time slots within a datetime range. Returns gaps between existing entries. Use this instead of listing all entries when you need to find free time for scheduling. Supports filtering so only specific client/project entries count as occupied."
    )]
    fn available_slots(&self, Parameters(params): Parameters<AvailableSlotsParams>) -> String {
        let range_start = match parse_datetime(&params.start)
            .or_else(|_| parse_date(&params.start).map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
        {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };
        let range_end = match parse_datetime(&params.end)
            .or_else(|_| parse_date(&params.end).map(|d| d.and_hms_opt(23, 59, 0).unwrap()))
        {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };

        if range_end <= range_start {
            return validation_error("end must be after start");
        }

        // Build a DateRange that covers the full span (expand by 1 day to catch overnight entries)
        let start_date = range_start.date();
        let end_date = range_end.date();
        let load_start = start_date.pred_opt().unwrap_or(start_date);
        let range = DateRange::new(load_start, end_date);

        let filter = EntryFilter {
            client: params.client,
            project: params.project,
            activity: params.activity,
            ..Default::default()
        };

        let mut cache = self.state.cache.lock().unwrap();
        let entries =
            match storage::load_filtered(&range, &filter, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            };

        let slots = compute_available_slots(
            range_start,
            range_end,
            &entries,
            params.min_duration_minutes,
        );

        let total_available: i64 = slots.iter().map(|s| s.duration_minutes).sum();

        let slots_json: Vec<_> = slots
            .iter()
            .map(|s| {
                json!({
                    "start": s.start.format("%Y-%m-%d %H:%M").to_string(),
                    "end": s.end.format("%Y-%m-%d %H:%M").to_string(),
                    "duration_minutes": s.duration_minutes,
                    "duration_formatted": format_duration(s.duration_minutes),
                })
            })
            .collect();

        json!({
            "available_slots": slots_json,
            "total_slots": slots.len(),
            "total_available_minutes": total_available,
            "total_available_formatted": format_duration(total_available),
        })
        .to_string()
    }

    // --- Mutation tools ---

    #[tool(
        description = "Create a new time entry. Validates client/project/activity against config and checks for overlaps. Supports snap_to_grid for rounding times."
    )]
    fn create_entry(&self, Parameters(params): Parameters<CreateEntryParams>) -> String {
        let config = self.state.config.read().unwrap();
        let mut start = match parse_datetime(&params.start) {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };
        let mut end = match parse_datetime(&params.end) {
            Ok(dt) => dt,
            Err(e) => return validation_error(&e),
        };

        if params.snap_to_grid.unwrap_or(false) {
            let snap = config.preferences.snap_minutes;
            start = snap_to_grid(start, snap);
            end = snap_to_grid(end, snap);
        }

        let new = NewEntry {
            start,
            end,
            description: params.description,
            client: params.client,
            project: params.project,
            activity: params.activity,
            notes: params.notes,
        };

        let mut cache = self.state.cache.lock().unwrap();
        match storage::create_entry(new, &config, &self.state.file_manager, &mut cache) {
            Ok(entry) => json!({ "entry": entry_to_json(&entry) }).to_string(),
            Err(e) => error_json(&e),
        }
    }

    #[tool(
        description = "Update an existing time entry. Identify by entry_id or original start/end timestamps. Supports partial updates: omitted fields keep their current values."
    )]
    fn update_entry(&self, Parameters(params): Parameters<UpdateEntryParams>) -> String {
        let config = self.state.config.read().unwrap();
        let mut cache = self.state.cache.lock().unwrap();

        // Resolve the existing entry
        let existing = if let Some(ref id) = params.entry_id {
            let today = Local::now().date_naive();
            let range = DateRange::new(
                today - chrono::Months::new(3),
                today + chrono::Months::new(1),
            );
            match storage::find_entry_by_id(id, &range, &self.state.file_manager, &mut cache) {
                Ok(e) => e,
                Err(e) => return error_json(&e),
            }
        } else if let (Some(os), Some(oe)) = (&params.original_start, &params.original_end) {
            let start = match parse_datetime(os) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            let end = match parse_datetime(oe) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            let month_key = start.format("%Y-%m").to_string();
            let entries =
                match storage::load_month(&month_key, &self.state.file_manager, &mut cache) {
                    Ok(e) => e,
                    Err(e) => return error_json(&e),
                };
            match entries
                .into_iter()
                .find(|e| e.start == start && e.end == end)
            {
                Some(e) => e,
                None => return error_json(&wdttg_core::Error::NotFound),
            }
        } else {
            return validation_error(
                "provide either entry_id or both original_start and original_end to identify the entry",
            );
        };

        let original_start = existing.start;
        let original_end = existing.end;

        // Merge: provided fields override existing values
        let new_start = if let Some(ref s) = params.start {
            match parse_datetime(s) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            }
        } else {
            existing.start
        };

        let new_end = if let Some(ref e) = params.end {
            match parse_datetime(e) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            }
        } else {
            existing.end
        };

        let updated = NewEntry {
            start: new_start,
            end: new_end,
            description: params.description.unwrap_or(existing.description),
            client: params.client.unwrap_or(existing.client),
            project: params.project.or(existing.project),
            activity: params.activity.or(existing.activity),
            notes: params.notes.or(existing.notes),
        };

        match storage::update_entry(
            original_start,
            original_end,
            updated,
            &config,
            &self.state.file_manager,
            &mut cache,
        ) {
            Ok(entry) => json!({ "entry": entry_to_json(&entry) }).to_string(),
            Err(e) => error_json(&e),
        }
    }

    #[tool(description = "Delete a time entry. Identify by entry_id or start/end timestamps.")]
    fn delete_entry(&self, Parameters(params): Parameters<DeleteEntryParams>) -> String {
        let mut cache = self.state.cache.lock().unwrap();

        let (start, end) = if let Some(ref id) = params.entry_id {
            let today = Local::now().date_naive();
            let range = DateRange::new(
                today - chrono::Months::new(3),
                today + chrono::Months::new(1),
            );
            match storage::find_entry_by_id(id, &range, &self.state.file_manager, &mut cache) {
                Ok(e) => (e.start, e.end),
                Err(e) => return error_json(&e),
            }
        } else if let (Some(s), Some(e)) = (&params.start, &params.end) {
            let start = match parse_datetime(s) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            let end = match parse_datetime(e) {
                Ok(dt) => dt,
                Err(e) => return validation_error(&e),
            };
            (start, end)
        } else {
            return validation_error(
                "provide either entry_id or both start and end to identify the entry",
            );
        };

        match storage::delete_entry(start, end, &self.state.file_manager, &mut cache) {
            Ok(()) => json!({ "deleted": true, "start": start.format("%Y-%m-%d %H:%M").to_string(), "end": end.format("%Y-%m-%d %H:%M").to_string() }).to_string(),
            Err(e) => error_json(&e),
        }
    }

    // --- Config management tools ---

    #[tool(description = "Create a new client. Requires a unique ID, name, color, and rate.")]
    fn create_client(&self, Parameters(params): Parameters<CreateClientParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        if config.clients.iter().any(|c| c.id == params.id) {
            return validation_error(&format!("client ID '{}' already exists", params.id));
        }
        let client = Client {
            id: params.id,
            name: params.name,
            color: params.color,
            rate: params.rate,
            currency: params.currency.unwrap_or_else(|| "USD".into()),
            archived: false,
            address: params.address,
            email: params.email,
            tax_id: params.tax_id,
            payment_terms: params.payment_terms,
            notes: params.notes,
            projects: vec![],
            activities: vec![],
        };
        let result = json!({ "client": { "id": client.id, "name": client.name } });
        config.clients.push(client);
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(
        description = "Update an existing client's fields. Only provided fields are changed; omitted fields keep their current values."
    )]
    fn update_client(&self, Parameters(params): Parameters<UpdateClientParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.id) else {
            return validation_error(&format!("client '{}' not found", params.id));
        };
        if let Some(name) = params.name {
            client.name = name;
        }
        if let Some(color) = params.color {
            client.color = color;
        }
        if let Some(rate) = params.rate {
            client.rate = rate;
        }
        if let Some(currency) = params.currency {
            client.currency = currency;
        }
        if let Some(address) = params.address {
            client.address = Some(address);
        }
        if let Some(email) = params.email {
            client.email = Some(email);
        }
        if let Some(tax_id) = params.tax_id {
            client.tax_id = Some(tax_id);
        }
        if let Some(payment_terms) = params.payment_terms {
            client.payment_terms = Some(payment_terms);
        }
        if let Some(notes) = params.notes {
            client.notes = Some(notes);
        }
        let result = json!({ "updated": true, "id": client.id, "name": client.name });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(
        description = "Archive or unarchive a client. Archived clients are hidden from entry form dropdowns but historical data is preserved."
    )]
    fn archive_client(&self, Parameters(params): Parameters<ArchiveClientParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.id) else {
            return validation_error(&format!("client '{}' not found", params.id));
        };
        client.archived = params.archived;
        let result = json!({ "id": client.id, "archived": client.archived });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(
        description = "Create a new project under a client. Requires a unique ID within the client."
    )]
    fn create_project(&self, Parameters(params): Parameters<CreateProjectParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        if client.projects.iter().any(|p| p.id == params.id) {
            return validation_error(&format!(
                "project ID '{}' already exists under client '{}'",
                params.id, params.client_id
            ));
        }
        let project = Project {
            id: params.id,
            name: params.name,
            color: params.color,
            rate_override: params.rate_override,
            archived: false,
        };
        let result = json!({ "project": { "id": project.id, "name": project.name, "client_id": params.client_id } });
        client.projects.push(project);
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(description = "Update an existing project's fields. Only provided fields are changed.")]
    fn update_project(&self, Parameters(params): Parameters<UpdateProjectParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        let Some(project) = client.projects.iter_mut().find(|p| p.id == params.id) else {
            return validation_error(&format!("project '{}' not found", params.id));
        };
        if let Some(name) = params.name {
            project.name = name;
        }
        if let Some(color) = params.color {
            project.color = color;
        }
        if let Some(rate_override) = params.rate_override {
            project.rate_override = Some(rate_override);
        }
        let result = json!({ "updated": true, "id": project.id, "name": project.name });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(description = "Archive or unarchive a project under a client.")]
    fn archive_project(&self, Parameters(params): Parameters<ArchiveProjectParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        let Some(project) = client.projects.iter_mut().find(|p| p.id == params.id) else {
            return validation_error(&format!("project '{}' not found", params.id));
        };
        project.archived = params.archived;
        let result = json!({ "id": project.id, "archived": project.archived });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(
        description = "Create a new activity under a client. Requires a unique ID within the client."
    )]
    fn create_activity(&self, Parameters(params): Parameters<CreateActivityParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        if client.activities.iter().any(|a| a.id == params.id) {
            return validation_error(&format!(
                "activity ID '{}' already exists under client '{}'",
                params.id, params.client_id
            ));
        }
        let activity = Activity {
            id: params.id,
            name: params.name,
            color: params.color,
            archived: false,
        };
        let result = json!({ "activity": { "id": activity.id, "name": activity.name, "client_id": params.client_id } });
        client.activities.push(activity);
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(description = "Update an existing activity's fields. Only provided fields are changed.")]
    fn update_activity(&self, Parameters(params): Parameters<UpdateActivityParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        let Some(activity) = client.activities.iter_mut().find(|a| a.id == params.id) else {
            return validation_error(&format!("activity '{}' not found", params.id));
        };
        if let Some(name) = params.name {
            activity.name = name;
        }
        if let Some(color) = params.color {
            activity.color = color;
        }
        let result = json!({ "updated": true, "id": activity.id, "name": activity.name });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }

    #[tool(description = "Archive or unarchive an activity under a client.")]
    fn archive_activity(&self, Parameters(params): Parameters<ArchiveActivityParams>) -> String {
        let mut config = self.state.config.write().unwrap();
        let Some(client) = config.clients.iter_mut().find(|c| c.id == params.client_id) else {
            return validation_error(&format!("client '{}' not found", params.client_id));
        };
        let Some(activity) = client.activities.iter_mut().find(|a| a.id == params.id) else {
            return validation_error(&format!("activity '{}' not found", params.id));
        };
        activity.archived = params.archived;
        let result = json!({ "id": activity.id, "archived": activity.archived });
        if let Err(e) = config::save_config(&config) {
            return error_json(&e);
        }
        result.to_string()
    }
}

#[tool_handler]
impl ServerHandler for WdttgMcpServer {
    fn get_info(&self) -> ServerInfo {
        let data_path = self.state.file_manager.data_dir().display();
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(format!("wdttg time tracking server. Manages freelancer time entries stored in {data_path} as GFM markdown tables."))
    }
}
