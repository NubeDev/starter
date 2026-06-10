//! Map folder store records to their wire DTOs.

use nexus_spi::dto::folder::FolderSummary;
use nexus_store::folder::FolderRecord;

/// One folder record to its summary DTO.
pub fn to_summary(r: &FolderRecord) -> FolderSummary {
    FolderSummary {
        id: r.id,
        parent_id: r.parent_id,
        name: r.name.clone(),
    }
}
