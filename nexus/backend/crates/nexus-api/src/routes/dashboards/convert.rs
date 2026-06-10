//! Map dashboard/panel store records to their wire DTOs.

use nexus_spi::dto::dashboard::{DashboardDetail, DashboardSummary};
use nexus_spi::dto::panel::PanelDetail;
use nexus_store::dashboard::{DashboardRecord, PanelRecord};

/// List view: id + slug + name + appearance.
pub fn to_summary(r: &DashboardRecord) -> DashboardSummary {
    DashboardSummary {
        id: r.id,
        slug: r.slug.clone(),
        name: r.name.clone(),
        icon: r.icon.clone(),
        accent: r.accent.clone(),
        folder_id: r.folder_id,
        starred: r.starred,
    }
}

/// Detail view: the dashboard plus its panels.
pub fn to_detail(d: &DashboardRecord, panels: &[PanelRecord]) -> DashboardDetail {
    DashboardDetail {
        id: d.id,
        slug: d.slug.clone(),
        name: d.name.clone(),
        icon: d.icon.clone(),
        accent: d.accent.clone(),
        folder_id: d.folder_id,
        starred: d.starred,
        panels: panels.iter().map(to_panel).collect(),
    }
}

/// One panel record to its DTO.
pub fn to_panel(p: &PanelRecord) -> PanelDetail {
    PanelDetail {
        id: p.id,
        title: p.title.clone(),
        datasource_id: p.datasource_id,
        sql: p.sql.clone(),
        viz: p.viz.clone(),
        layout: p.layout.clone(),
    }
}
