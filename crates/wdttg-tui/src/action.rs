/// All possible user actions in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    SwitchToTimeline,
    SwitchToReports,
    SwitchToManage,
    ToggleHelp,
    ClosePopup,
    NavigateLeft,
    NavigateRight,
    NavigateUp,
    NavigateDown,
    Select,
    Create,
    Edit,
    Delete,
    JumpToToday,
    ScrollWeekLeft,
    ScrollWeekRight,
    PageUp,
    PageDown,
    ToggleArchive,
    /// Mark start/end of a time range on the timeline.
    MarkTime,
    /// Export the current report.
    Export,
}
