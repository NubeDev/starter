//! Typed ids for the control-plane resources.
//!
//! Internally every id is a v4 UUID carried by `starter_spi::id::Id<T>`, whose
//! phantom marker stops a dashboard id being passed where a panel id is wanted.
//! `Id<T>` serializes transparently as the raw UUID string but does not itself
//! implement `utoipa::ToSchema`; DTOs that cross the wire therefore expose ids
//! as `uuid::Uuid` and convert at the store boundary. These aliases name the
//! phantom marker per resource so internal code stays type-checked.

use starter_spi::id::Id;

/// Phantom marker for a datasource id.
pub enum Datasource {}
/// Phantom marker for a dashboard id.
pub enum Dashboard {}
/// Phantom marker for a panel id.
pub enum Panel {}
/// Phantom marker for a live-stream id.
pub enum Stream {}
/// Phantom marker for a saved-flow id.
pub enum Flow {}

pub type DatasourceId = Id<Datasource>;
pub type DashboardId = Id<Dashboard>;
pub type PanelId = Id<Panel>;
pub type StreamId = Id<Stream>;
pub type FlowId = Id<Flow>;
