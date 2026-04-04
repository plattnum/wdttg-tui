use wdttg_core::config::AppConfig;
use wdttg_core::model::{Activity, Client, Project};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagePane {
    Clients,
    Projects,
    Activities,
}

impl ManagePane {
    pub fn next(self) -> Self {
        match self {
            Self::Clients => Self::Projects,
            Self::Projects => Self::Activities,
            Self::Activities => Self::Clients,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Clients => Self::Activities,
            Self::Projects => Self::Clients,
            Self::Activities => Self::Projects,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTarget {
    Client,
    Project,
    Activity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Id,
    Name,
    Color,
    Rate,
    Currency,
}

impl FormField {
    pub fn next(self, target: EditTarget) -> Self {
        match (self, target) {
            (Self::Id, _) => Self::Name,
            (Self::Name, _) => Self::Color,
            (Self::Color, EditTarget::Client) => Self::Rate,
            (Self::Color, _) => Self::Rate, // will wrap at save for non-client
            (Self::Rate, EditTarget::Client) => Self::Currency,
            (Self::Rate, _) => Self::Id,
            (Self::Currency, _) => Self::Id,
        }
    }
}

pub struct EditForm {
    pub target: EditTarget,
    pub is_new: bool,
    pub id: String,
    pub name: String,
    pub color: String,
    pub rate: String,
    pub currency: String,
    pub focused: FormField,
    pub cursor_pos: usize,
    pub error: Option<String>,
}

impl EditForm {
    pub fn new_client() -> Self {
        Self {
            target: EditTarget::Client,
            is_new: true,
            id: String::new(),
            name: String::new(),
            color: "#4ECDC4".into(),
            rate: "0.0".into(),
            currency: "USD".into(),
            focused: FormField::Id,
            cursor_pos: 0,
            error: None,
        }
    }

    pub fn edit_client(client: &Client) -> Self {
        Self {
            target: EditTarget::Client,
            is_new: false,
            id: client.id.clone(),
            name: client.name.clone(),
            color: client.color.clone(),
            rate: client.rate.to_string(),
            currency: client.currency.clone(),
            focused: FormField::Name,
            cursor_pos: client.name.len(),
            error: None,
        }
    }

    pub fn new_project() -> Self {
        Self {
            target: EditTarget::Project,
            is_new: true,
            id: String::new(),
            name: String::new(),
            color: "#45B7D1".into(),
            rate: String::new(),
            currency: String::new(),
            focused: FormField::Id,
            cursor_pos: 0,
            error: None,
        }
    }

    pub fn edit_project(project: &Project) -> Self {
        Self {
            target: EditTarget::Project,
            is_new: false,
            id: project.id.clone(),
            name: project.name.clone(),
            color: project.color.clone(),
            rate: project
                .rate_override
                .map(|r| r.to_string())
                .unwrap_or_default(),
            currency: String::new(),
            focused: FormField::Name,
            cursor_pos: project.name.len(),
            error: None,
        }
    }

    pub fn new_activity() -> Self {
        Self {
            target: EditTarget::Activity,
            is_new: true,
            id: String::new(),
            name: String::new(),
            color: "#2ECC71".into(),
            rate: String::new(),
            currency: String::new(),
            focused: FormField::Id,
            cursor_pos: 0,
            error: None,
        }
    }

    pub fn edit_activity(activity: &Activity) -> Self {
        Self {
            target: EditTarget::Activity,
            is_new: false,
            id: activity.id.clone(),
            name: activity.name.clone(),
            color: activity.color.clone(),
            rate: String::new(),
            currency: String::new(),
            focused: FormField::Name,
            cursor_pos: activity.name.len(),
            error: None,
        }
    }

    pub fn next_field(&mut self) {
        self.focused = self.focused.next(self.target);
        self.update_cursor();
    }

    pub fn type_char(&mut self, ch: char) {
        let pos = self.cursor_pos;
        self.active_field_mut().insert(pos, ch);
        self.cursor_pos += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            let pos = self.cursor_pos;
            self.active_field_mut().remove(pos);
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.active_field().len() {
            self.cursor_pos += 1;
        }
    }

    fn active_field(&self) -> &str {
        match self.focused {
            FormField::Id => &self.id,
            FormField::Name => &self.name,
            FormField::Color => &self.color,
            FormField::Rate => &self.rate,
            FormField::Currency => &self.currency,
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.focused {
            FormField::Id => &mut self.id,
            FormField::Name => &mut self.name,
            FormField::Color => &mut self.color,
            FormField::Rate => &mut self.rate,
            FormField::Currency => &mut self.currency,
        }
    }

    fn update_cursor(&mut self) {
        self.cursor_pos = self.active_field().len();
    }

    pub fn validate(&mut self, config: &AppConfig, client_idx: Option<usize>) -> bool {
        if self.id.is_empty() {
            self.error = Some("ID is required".into());
            return false;
        }
        if self.name.is_empty() {
            self.error = Some("Name is required".into());
            return false;
        }

        // Check ID uniqueness
        match self.target {
            EditTarget::Client => {
                if self.is_new && config.clients.iter().any(|c| c.id == self.id) {
                    self.error = Some(format!("Client ID '{}' already exists", self.id));
                    return false;
                }
                if self.rate.parse::<f64>().is_err() {
                    self.error = Some("Rate must be a number".into());
                    return false;
                }
            }
            EditTarget::Project => {
                if let Some(ci) = client_idx {
                    if let Some(client) = config.clients.get(ci) {
                        if self.is_new && client.projects.iter().any(|p| p.id == self.id) {
                            self.error = Some(format!("Project ID '{}' already exists", self.id));
                            return false;
                        }
                    }
                }
            }
            EditTarget::Activity => {
                if let Some(ci) = client_idx {
                    if let Some(client) = config.clients.get(ci) {
                        if self.is_new && client.activities.iter().any(|a| a.id == self.id) {
                            self.error = Some(format!("Activity ID '{}' already exists", self.id));
                            return false;
                        }
                    }
                }
            }
        }

        self.error = None;
        true
    }

    pub fn apply_to_config(&self, config: &mut AppConfig, client_idx: usize) {
        match self.target {
            EditTarget::Client => {
                let rate = self.rate.parse::<f64>().unwrap_or(0.0);
                if self.is_new {
                    config.clients.push(Client {
                        id: self.id.clone(),
                        name: self.name.clone(),
                        color: self.color.clone(),
                        rate,
                        currency: self.currency.clone(),
                        archived: false,
                        address: None,
                        email: None,
                        tax_id: None,
                        payment_terms: None,
                        notes: None,
                        projects: vec![],
                        activities: vec![],
                    });
                } else if let Some(client) = config.clients.get_mut(client_idx) {
                    client.name = self.name.clone();
                    client.color = self.color.clone();
                    client.rate = rate;
                    client.currency = self.currency.clone();
                }
            }
            EditTarget::Project => {
                let rate_override = self.rate.parse::<f64>().ok();
                if let Some(client) = config.clients.get_mut(client_idx) {
                    if self.is_new {
                        client.projects.push(Project {
                            id: self.id.clone(),
                            name: self.name.clone(),
                            color: self.color.clone(),
                            rate_override,
                            archived: false,
                        });
                    } else if let Some(proj) = client.projects.iter_mut().find(|p| p.id == self.id)
                    {
                        proj.name = self.name.clone();
                        proj.color = self.color.clone();
                        proj.rate_override = rate_override;
                    }
                }
            }
            EditTarget::Activity => {
                if let Some(client) = config.clients.get_mut(client_idx) {
                    if self.is_new {
                        client.activities.push(Activity {
                            id: self.id.clone(),
                            name: self.name.clone(),
                            color: self.color.clone(),
                            archived: false,
                        });
                    } else if let Some(act) = client.activities.iter_mut().find(|a| a.id == self.id)
                    {
                        act.name = self.name.clone();
                        act.color = self.color.clone();
                    }
                }
            }
        }
    }
}

pub struct ManageState {
    pub active_pane: ManagePane,
    pub client_idx: usize,
    pub project_idx: usize,
    pub activity_idx: usize,
    pub edit_form: Option<EditForm>,
}

impl ManageState {
    pub fn new() -> Self {
        Self {
            active_pane: ManagePane::Clients,
            client_idx: 0,
            project_idx: 0,
            activity_idx: 0,
            edit_form: None,
        }
    }

    pub fn move_up(&mut self) {
        match self.active_pane {
            ManagePane::Clients => {
                if self.client_idx > 0 {
                    self.client_idx -= 1;
                    self.project_idx = 0;
                    self.activity_idx = 0;
                }
            }
            ManagePane::Projects => {
                if self.project_idx > 0 {
                    self.project_idx -= 1;
                }
            }
            ManagePane::Activities => {
                if self.activity_idx > 0 {
                    self.activity_idx -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self, config: &AppConfig) {
        match self.active_pane {
            ManagePane::Clients => {
                if self.client_idx + 1 < config.clients.len() {
                    self.client_idx += 1;
                    self.project_idx = 0;
                    self.activity_idx = 0;
                }
            }
            ManagePane::Projects => {
                if let Some(client) = config.clients.get(self.client_idx) {
                    if self.project_idx + 1 < client.projects.len() {
                        self.project_idx += 1;
                    }
                }
            }
            ManagePane::Activities => {
                if let Some(client) = config.clients.get(self.client_idx) {
                    if self.activity_idx + 1 < client.activities.len() {
                        self.activity_idx += 1;
                    }
                }
            }
        }
    }

    pub fn switch_pane(&mut self) {
        self.active_pane = self.active_pane.next();
    }

    pub fn open_add(&mut self) {
        self.edit_form = Some(match self.active_pane {
            ManagePane::Clients => EditForm::new_client(),
            ManagePane::Projects => EditForm::new_project(),
            ManagePane::Activities => EditForm::new_activity(),
        });
    }

    pub fn open_edit(&mut self, config: &AppConfig) {
        let form = match self.active_pane {
            ManagePane::Clients => config
                .clients
                .get(self.client_idx)
                .map(EditForm::edit_client),
            ManagePane::Projects => config
                .clients
                .get(self.client_idx)
                .and_then(|c| c.projects.get(self.project_idx))
                .map(EditForm::edit_project),
            ManagePane::Activities => config
                .clients
                .get(self.client_idx)
                .and_then(|c| c.activities.get(self.activity_idx))
                .map(EditForm::edit_activity),
        };
        self.edit_form = form;
    }

    pub fn delete_selected(&mut self, config: &mut AppConfig) {
        match self.active_pane {
            ManagePane::Clients => {
                if config.clients.len() > 1 && self.client_idx < config.clients.len() {
                    config.clients.remove(self.client_idx);
                    if self.client_idx >= config.clients.len() {
                        self.client_idx = config.clients.len().saturating_sub(1);
                    }
                    self.project_idx = 0;
                    self.activity_idx = 0;
                }
            }
            ManagePane::Projects => {
                if let Some(client) = config.clients.get_mut(self.client_idx) {
                    if self.project_idx < client.projects.len() {
                        client.projects.remove(self.project_idx);
                        if self.project_idx >= client.projects.len() {
                            self.project_idx = client.projects.len().saturating_sub(1);
                        }
                    }
                }
            }
            ManagePane::Activities => {
                if let Some(client) = config.clients.get_mut(self.client_idx) {
                    if self.activity_idx < client.activities.len() {
                        client.activities.remove(self.activity_idx);
                        if self.activity_idx >= client.activities.len() {
                            self.activity_idx = client.activities.len().saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_archive(&mut self, config: &mut AppConfig) {
        match self.active_pane {
            ManagePane::Clients => {
                if let Some(client) = config.clients.get_mut(self.client_idx) {
                    client.archived = !client.archived;
                }
            }
            ManagePane::Projects => {
                if let Some(client) = config.clients.get_mut(self.client_idx) {
                    if let Some(project) = client.projects.get_mut(self.project_idx) {
                        project.archived = !project.archived;
                    }
                }
            }
            ManagePane::Activities => {
                if let Some(client) = config.clients.get_mut(self.client_idx) {
                    if let Some(activity) = client.activities.get_mut(self.activity_idx) {
                        activity.archived = !activity.archived;
                    }
                }
            }
        }
    }
}
