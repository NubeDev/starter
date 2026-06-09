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
        crate::routes::query::run::run_query,
        crate::routes::query::kinds::list_query_kinds,
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
        crate::routes::dashboards::create::create_dashboard,
        crate::routes::dashboards::list::list_dashboards,
        crate::routes::dashboards::get::get_dashboard,
        crate::routes::dashboards::update::update_dashboard,
        crate::routes::dashboards::delete::delete_dashboard,
        crate::routes::dashboards::add_panel::add_panel,
        crate::routes::dashboards::update_panel::update_panel,
        crate::routes::dashboards::delete_panel::delete_panel,
        crate::routes::flows::list::list_flows,
        crate::routes::flows::create::create_flow,
        crate::routes::flows::get::get_flow,
        crate::routes::flows::update::update_flow,
        crate::routes::flows::delete::delete_flow,
        crate::routes::flows::start::start_flow,
        crate::routes::flows::stop::stop_flow,
        crate::routes::alerts::rules::list_rules,
        crate::routes::alerts::rules::create_rule,
        crate::routes::alerts::rules::get_rule,
        crate::routes::alerts::rules::update_rule,
        crate::routes::alerts::rules::delete_rule,
        crate::routes::alerts::events::list_events,
        crate::routes::alerts::channels::list_channels,
        crate::routes::alerts::channels::create_channel,
        crate::routes::alerts::channels::delete_channel,
        crate::routes::alerts::silences::list_silences,
        crate::routes::alerts::silences::create_silence,
        crate::routes::alerts::silences::delete_silence,
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
        crate::routes::variables::list::list_variables,
        crate::routes::variables::create::create_variable,
        crate::routes::variables::update::update_variable,
        crate::routes::variables::delete::delete_variable,
    ),
    components(schemas(
        // Preferences (WS-11) — starter-spi types referenced by the
        // `/api/v1/me/preferences` handlers. Registered here so the document's
        // `$ref`s resolve without nexus-spi re-exporting starter types.
        starter_spi::preferences::ResolvedPreferences,
        starter_spi::preferences::PreferencesPatch,
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
             one-shot queries, live SSE streams, dashboards, panels, flows, and alerts.",
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
