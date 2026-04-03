mod entry_validator;
mod overlap;

pub use entry_validator::{find_activity, find_client, find_project, validate_new_entry};
pub use overlap::{OverlapInfo, OverlapResult, OverlapType, find_overlaps};
