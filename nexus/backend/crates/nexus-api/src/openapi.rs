//! Assemble the full OpenAPI document: nexus-api's route paths merged with the
//! DTO schema surface published by `nexus-spi`.
//!
//! Paths are registered here as handlers land (each `#[utoipa::path]` handler is
//! added to [`Paths`]). The schema components come from `nexus_spi`, so the
//! type contract has a single source of truth that the frontend codegens from.

use utoipa::openapi::{InfoBuilder, OpenApi as OpenApiDoc};
use utoipa::OpenApi;

/// Route paths declared by nexus-api handlers. Each `#[utoipa::path]` handler is
/// listed here so it appears in the published document.
#[derive(OpenApi)]
#[openapi(
    info(title = "Nexus API", version = "0.1.0"),
    paths(
        crate::routes::me::get::get_me,
        crate::routes::me::preferences::get_me_preferences,
        crate::routes::me::preferences::patch_me_preferences,
        crate::routes::me::settings::get_me_settings,
        crate::routes::me::settings::set_me_settings,
        crate::routes::query::run::run_query,
        crate::routes::nexus_db::query::query_nexus_db,
        crate::routes::query::kinds::list_query_kinds,
        crate::routes::query_kinds::list::list_query_kinds_admin,
        crate::routes::query_kinds::create::create_query_kind,
        crate::routes::query_kinds::get::get_query_kind,
        crate::routes::query_kinds::update::update_query_kind,
        crate::routes::query_kinds::delete::delete_query_kind,
        crate::routes::query::history::list_query_history,
        crate::routes::query::history::star_query_history,
        crate::routes::streams::create::create_stream,
        crate::routes::streams::subscribe::subscribe_stream,
        crate::routes::datasources::create::create_datasource,
        crate::routes::datasources::list::list_datasources,
        crate::routes::datasources::get::get_datasource,
        crate::routes::datasources::update::update_datasource,
        crate::routes::datasources::delete::delete_datasource,
        crate::routes::datasources::query::query_datasource,
        crate::routes::datasources::schema::datasource_schema,
        crate::routes::datasources::test::test_datasource,
        crate::routes::datasources::test_connection::test_connection,
        crate::routes::datasources::kinds::list_datasource_kinds,
        crate::routes::dashboards::create::create_dashboard,
        crate::routes::dashboards::list::list_dashboards,
        crate::routes::dashboards::get::get_dashboard,
        crate::routes::dashboards::update::update_dashboard,
        crate::routes::dashboards::delete::delete_dashboard,
        crate::routes::dashboards::duplicate::duplicate_dashboard,
        crate::routes::dashboards::export::export_dashboard,
        crate::routes::dashboards::import::import_dashboard,
        crate::routes::dashboards::add_panel::add_panel,
        crate::routes::dashboards::update_panel::update_panel,
        crate::routes::dashboards::delete_panel::delete_panel,
        crate::routes::folders::list::list_folders,
        crate::routes::folders::create::create_folder,
        crate::routes::folders::update::update_folder,
        crate::routes::folders::delete::delete_folder,
        crate::routes::insights::list::list_insights,
        crate::routes::insights::create::create_insight,
        crate::routes::insights::preview::preview_insight,
        crate::routes::insights::functions::list_functions,
        crate::routes::insights::get::get_insight,
        crate::routes::insights::update::update_insight,
        crate::routes::insights::delete::delete_insight,
        crate::routes::nav::list::list_nav,
        crate::routes::nav::create::create_nav,
        crate::routes::nav::get::get_nav,
        crate::routes::nav::update::update_nav,
        crate::routes::nav::delete::delete_nav,
        crate::routes::flows::list::list_flows,
        crate::routes::flows::create::create_flow,
        crate::routes::flows::nodes::list_node_types,
        crate::routes::flows::dryrun::dry_run_flow,
        crate::routes::flows::get::get_flow,
        crate::routes::flows::update::update_flow,
        crate::routes::flows::delete::delete_flow,
        crate::routes::flows::start::start_flow,
        crate::routes::flows::stop::stop_flow,
        crate::routes::flows::export::export_flow,
        crate::routes::flows::import::import_flow,
        crate::routes::flows::debug::enable_flow_debug,
        crate::routes::flows::debug::disable_flow_debug,
        crate::routes::flows::debug::stream_flow_debug,
        crate::routes::flows::table_query::query_flow_table,
        crate::routes::ingest::push::push_ingest,
        crate::routes::detections::notify::list_channels,
        crate::routes::detections::notify::create_channel,
        crate::routes::detections::notify::delete_channel,
        crate::routes::detections::notify::list_silences,
        crate::routes::detections::notify::create_silence,
        crate::routes::detections::notify::delete_silence,
        crate::routes::detections::notify::list_notify_events,
        crate::routes::detections::crud::list_detections,
        crate::routes::detections::crud::create_detection,
        crate::routes::detections::crud::get_detection,
        crate::routes::detections::crud::update_detection,
        crate::routes::detections::crud::delete_detection,
        crate::routes::detections::crud::run_now,
        crate::routes::detections::crud::get_stats,
        crate::routes::detections::findings::list_findings,
        crate::routes::detections::findings::get_finding,
        crate::routes::detections::findings::ack_finding,
        crate::routes::detections::findings::resolve_finding,
        crate::routes::tags::set::set_tags,
        crate::routes::tags::get::get_tags,
        crate::routes::tags::keys::list_tag_keys,
        crate::routes::tags::list_entities::list_entities_with_tag,
        crate::routes::agents::list::list_agents,
        crate::routes::agents::create::create_agent,
        crate::routes::agents::get::get_agent,
        crate::routes::agents::update::update_agent,
        crate::routes::agents::delete::delete_agent,
        crate::routes::agents::create_session::create_agent_session,
        crate::routes::agents::list_sessions::list_agent_sessions,
        crate::routes::agents::get_session::get_agent_session,
        crate::routes::agents::events::subscribe_agent_session,
        crate::routes::ai::assist::ai_assist,
        crate::routes::variables::list::list_variables,
        crate::routes::variables::create::create_variable,
        crate::routes::variables::update::update_variable,
        crate::routes::variables::delete::delete_variable,
        crate::routes::audit::list::list_audit,
        crate::routes::audit::resource::resource_history,
        crate::routes::audit::forget::forget_subject,
        crate::routes::undo::apply::undo,
        crate::routes::undo::apply::redo,
    ),
    components(schemas(
        // The admin nexus-DB inspector's request body lives in the route module
        // (not nexus-spi), so register it here for the document's `$ref`.
        crate::routes::nexus_db::query::NexusDbQueryRequest,
        // Preferences (WS-11) — starter-spi types referenced by the
        // `/api/v1/me/preferences` handlers. Registered here so the document's
        // `$ref`s resolve without nexus-spi re-exporting starter types.
        starter_spi::preferences::ResolvedPreferences,
        starter_spi::preferences::PreferencesPatch,
        // Audit/undo (WS-12) — the changelog read model the `/api/v1/audit`
        // routes return. Registered here because these are starter-changelog /
        // starter-spi types, so nexus-spi need not re-export them for the
        // document's `$ref`s to resolve.
        starter_changelog::ChangePage,
        starter_spi::changelog::Change,
        starter_spi::changelog::Actor,
        starter_spi::changelog::Op,
    )),
)]
pub struct Paths;

/// The published document: nexus-api paths plus nexus-spi schemas.
pub fn document() -> OpenApiDoc {
    let mut doc = Paths::openapi();
    doc.info = InfoBuilder::new()
        .title("Nexus API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some(
            "Control plane for the Nexus observability/BI platform: datasources, \
             one-shot queries, live SSE streams, dashboards, panels, flows, and detections.",
        ))
        .build();
    doc.merge(nexus_spi::openapi::Schemas::openapi());
    doc
}

/// Newtype so the generator and server share one `OpenApi`-producing entry
/// point. `ApiDoc::openapi()` returns the fully merged document.
pub struct ApiDoc;

impl OpenApi for ApiDoc {
    fn openapi() -> OpenApiDoc {
        document()
    }
}
