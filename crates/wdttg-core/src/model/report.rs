/// Hierarchical report structures: Client -> Project -> Activity.

#[derive(Debug, Clone)]
pub struct ClientReport {
    pub client_id: String,
    pub name: String,
    pub color: String,
    pub rate: f64,
    pub currency: String,
    pub total_minutes: i64,
    pub billable_amount: f64,
    pub percentage: f64,
    pub project_breakdown: Vec<ProjectReport>,
}

#[derive(Debug, Clone)]
pub struct ProjectReport {
    pub project_id: String,
    pub name: String,
    pub color: String,
    pub total_minutes: i64,
    pub billable_amount: f64,
    pub percentage: f64,
    pub activity_breakdown: Vec<ActivityReport>,
}

#[derive(Debug, Clone)]
pub struct ActivityReport {
    pub activity_id: String,
    pub name: String,
    pub color: String,
    pub total_minutes: i64,
    pub percentage: f64,
}
