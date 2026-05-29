//! Router composition: assembles the final axum Router from booted
//! services, applying auth/authz/audit gates when a database is
//! configured.

use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use tracing::{info, warn};

use rubix_agent::boot::{self, runtime_canary::Canary};
use rubix_agent::{health, middleware, openapi as rubix_openapi_mod, routes};
use starter_changelog_postgres::PgChangeRecorder;
use starter_spi::changelog::ChangeRecorder;
use starter_store_postgres::pool::connect as pg_connect;

use super::services::BootedServices;

pub(crate) async fn compose_and_serve(
    svc: BootedServices,
    runtime_canary: Canary,
) -> Result<()> {
    let BootedServices {
        cfg,
        bundle: _,
        mcp_pool: _,
        ext_host_methods,
        ext_bundle,
        warehouse_client,
        tools,
        mut admin_state,
        mcp,
        flow_runtime,
        tools_registrar,
        flow_events_registrar,
        // Keep alive for process lifetime.
        _undo_sweep,
        _changelog_sweep,
        _scheduler,
        _flow_notify,
    } = svc;

    let _flow_runtime = flow_runtime;

    let mcp_routes = Router::new().nest("/api/v1", mcp.router);
    let openapi_doc_registrar =
        routes::openapi_doc::openapi_registrar(rubix_openapi_mod::rubix_openapi());
    let mut app: routes::RouteRegistrar = routes::RouteRegistrar::new()
        .merge(health::healthz_registrar())
        .merge(health::livez_registrar(runtime_canary.clone()))
        .merge_external(mcp_routes)
        .merge(openapi_doc_registrar)
        .merge(flow_events_registrar);

    if let Some(dsn) = cfg.database_url.as_deref() {
        let pool = pg_connect(dsn)
            .await
            .map_err(|e| anyhow::anyhow!("connect to RUBIX_DATABASE_URL: {e}"))?;
        let _t_auth = boot::pool_telemetry::spawn(pool.sqlx().clone(), "rubix-auth");
        app = app.merge(health::readyz_registrar(pool.sqlx().clone()));
        let auth = boot::build_auth(pool.clone());
        let auth_routes = routes::auth::auth_router(auth.state);
        let engine = boot::authz::build_engine()?;

        if let Some(wh_client) = warehouse_client.clone() {
            let explorer_router =
                starter_warehouse_explorer::router_with_auth(wh_client, auth.authenticator.clone());
            app = app.merge_external(explorer_router);
        }

        if let Some(bundle) = ext_bundle {
            let ext_router: Router =
                starter_ext_server::router_with_auth(bundle.admin, auth.authenticator.clone());
            app = app.merge_external(Router::new().nest("/api/v1", ext_router));

            if let Some(wh_client) = warehouse_client.clone() {
                use rubix_agent::extensions::{
                    with_caller_identity, CompositeRestDispatcher, RubixCapabilityFactory,
                    RubixEventBus, EXTENSION_REST_REQUEST_TIMEOUT,
                };
                use starter_ext_host::TemplateRegistry;
                use starter_ext_sdk::builtin::BuiltinTable;
                use starter_ext_server::{
                    rest_router, BuiltinRestDispatcher, CapabilityFactory, DispatcherCache,
                    KindCacheRegistry, ProcessRestDispatcher, RestDispatcher, RestRouterOptions,
                };
                use starter_server::auth::with_principal;

                let table = Arc::new(BuiltinTable::new());
                let mut tmpl = TemplateRegistry::builtin();
                for record in bundle.registry.iter_validated() {
                    match tmpl.extend_from_record(record) {
                        Ok(n) if n > 0 => {
                            info!(
                                target: "rubix.boot.extensions",
                                extension = %record.id_hint,
                                templates = n,
                                "registered contributed warehouse templates",
                            );
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!(
                                target: "rubix.boot.extensions",
                                extension = %record.id_hint,
                                error = %e,
                                "failed to register contributed warehouse templates",
                            );
                        }
                    }
                }
                let template_registry = Arc::new(tmpl);
                let event_bus = Arc::new(RubixEventBus::new());
                let dashboard_store: Arc<dyn rubix_spi::dashboard::DashboardStore> =
                    Arc::new(rubix_store_postgres::PgDashboardStore::new(pool.clone()));
                if let Some(host_methods) = ext_host_methods.as_ref() {
                    host_methods
                        .install_warehouse(wh_client.clone(), template_registry.clone());
                    host_methods.install_event_bus(event_bus.clone());
                }
                admin_state = admin_state.with_templates(template_registry.clone());
                let factory: Arc<dyn CapabilityFactory> = Arc::new(
                    RubixCapabilityFactory::new(wh_client, template_registry, event_bus)
                        .with_extension_registry(bundle.registry.clone())
                        .with_dashboard_store(dashboard_store)
                        .with_authz_engine(engine.clone()),
                );

                // Cache layer. Per-tenant cap defaults to 10k
                // entries; an operator can override with
                // `RUBIX_CACHE_PER_TENANT_MAX_ENTRIES` without
                // recompiling.
                let cache_layer = starter_cache::CacheLayer::new(
                    starter_cache::LayerConfig::from_env("RUBIX_CACHE"),
                );
                let mut cache_entries: Vec<(
                    (starter_ext_spi::ExtensionId, String),
                    starter_cache::CacheSpec,
                )> = Vec::new();
                for record in bundle.registry.iter_validated() {
                    let Some(ext_id) = record.id.clone() else {
                        continue;
                    };
                    let kinds_dir = record.bundle_dir.join("kinds");
                    if !kinds_dir.is_dir() {
                        continue;
                    }
                    match KindCacheRegistry::load_from_dir(&ext_id, &kinds_dir) {
                        Ok((reg, errors)) => {
                            for err in &errors {
                                warn!(
                                    target: "rubix.boot.extensions",
                                    extension = %ext_id.as_str(),
                                    path = %err.path.display(),
                                    error = %err.message,
                                    "cache sidecar failed to load — kind will be uncached",
                                );
                            }
                            for ent in std::fs::read_dir(&kinds_dir)
                                .into_iter()
                                .flatten()
                                .flatten()
                            {
                                let p = ent.path();
                                let name = match p.file_name().and_then(|n| n.to_str()) {
                                    Some(n) => n,
                                    None => continue,
                                };
                                let Some(stem) = name.strip_suffix(".cache.yaml") else {
                                    continue;
                                };
                                if let Some(spec) = reg.get(&ext_id, stem) {
                                    cache_entries.push((
                                        (ext_id.clone(), stem.to_string()),
                                        spec.clone(),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                target: "rubix.boot.extensions",
                                extension = %ext_id.as_str(),
                                error = %e,
                                "failed to scan kinds/ for cache sidecars",
                            );
                        }
                    }
                }
                if !cache_entries.is_empty() {
                    info!(
                        target: "rubix.boot.extensions",
                        kinds = cache_entries.len(),
                        "opt-in cache: registered kind specs",
                    );
                }
                let kind_cache =
                    KindCacheRegistry::from_entries(cache_entries.iter().cloned());

                let orphans = kind_cache.orphans(|ext| {
                    let mut ids: Vec<&str> = Vec::new();
                    if let Some(rec) = bundle.registry.get(ext) {
                        if let Some(m) = rec.manifest.as_ref() {
                            for t in &m.contributes.tools {
                                ids.push(t.id.as_str());
                            }
                            for r in &m.contributes.rest {
                                ids.push(r.id.as_str());
                            }
                            for c in &m.contributes.cli {
                                ids.push(c.id.as_str());
                            }
                            for g in &m.contributes.grpc {
                                ids.push(g.id.as_str());
                            }
                            for w in &m.contributes.workers {
                                ids.push(w.id.as_str());
                            }
                        }
                    }
                    ids.into_iter()
                });
                for o in &orphans {
                    warn!(
                        target: "rubix.boot.extensions",
                        extension = %o.extension.as_str(),
                        sidecar_stem = %o.contribute_id,
                        "cache sidecar references unknown contribute_id — \
                         the sidecar will silently never fire. Check for a \
                         typo or a stale file after a kind rename.",
                    );
                }

                let dispatcher_cache =
                    DispatcherCache::new(cache_layer.clone(), kind_cache.clone());
                admin_state = admin_state
                    .with_cache_layer(cache_layer.clone())
                    .with_cache_registry(kind_cache);

                let builtin = Arc::new(
                    BuiltinRestDispatcher::new(table, bundle.registry.clone())
                        .with_capability_factory(factory)
                        .with_cache(dispatcher_cache.clone()),
                );
                let process = Arc::new(
                    ProcessRestDispatcher::new(
                        bundle.process_handles.clone(),
                        EXTENSION_REST_REQUEST_TIMEOUT,
                    )
                    .with_cache(dispatcher_cache),
                );
                let dispatcher: Arc<dyn RestDispatcher> = Arc::new(CompositeRestDispatcher::new(
                    bundle.registry.clone(),
                    builtin,
                    process,
                ));
                match rest_router::<()>(
                    bundle.registry.clone(),
                    dispatcher,
                    RestRouterOptions::default(),
                ) {
                    Ok(adapter) => {
                        let gated = with_principal(
                            with_caller_identity(adapter),
                            auth.authenticator.clone(),
                        );
                        app = app.merge_external(Router::new().nest("/api/v1", gated));
                    }
                    Err(e) => {
                        warn!(
                            target: "rubix.boot.extensions",
                            error = %e,
                            "extension REST adapter failed to build; per-extension \
                             routes will not be served",
                        );
                    }
                }
            } else {
                info!(
                    target: "rubix.boot.extensions",
                    "warehouse_url unset — skipping extension REST adapter wiring",
                );
            }
        }

        // SDUI seed + router.
        let seed_registry = starter_authz::StaticRegistry::new();
        let inserted = boot::dashboards_seed::seed(Some(&pool), &seed_registry)
            .await
            .map_err(|e| anyhow::anyhow!("dashboards_seed::seed: {e}"))?;
        tracing::info!(inserted, "dashboards_definitions seed complete",);

        let sdui_router: Router =
            boot::build_sdui_router(&cfg, pool.clone(), warehouse_client.clone(), &tools);
        app = app.merge_external(sdui_router);

        // Dashboard events SSE.
        {
            use rubix_store_postgres::PgDashboardStore;
            use sqlx::postgres::PgPoolOptions;
            use starter_changelog_postgres::PgListenTail;
            use starter_server::auth::with_principal;
            use starter_store_postgres::pool::Pool;

            let listen_inner = PgPoolOptions::new()
                .max_connections(2)
                .connect(dsn)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("connect for dashboard_events listen pool: {e}")
                })?;
            let listen_pool = Pool::from_sqlx(listen_inner);
            let _t_dash_listen =
                boot::pool_telemetry::spawn(listen_pool.sqlx().clone(), "rubix-dash-listen");

            let tail = Arc::new(PgListenTail::new(listen_pool));
            let store = Arc::new(PgDashboardStore::new(pool.clone()));
            let de_registrar = routes::dashboard_events::registrar(
                routes::dashboard_events::DashboardEventsState { tail, store },
            )
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(de_registrar);
        }

        // Chat stream.
        {
            use starter_server::auth::with_principal;
            let chat_runner = boot::ai::build_runner(&cfg)
                .map_err(|e| anyhow::anyhow!("boot::ai::build_runner (chat): {e}"))?;
            let chat_registrar = routes::chat_stream::registrar(
                routes::chat_stream::ChatStreamState::from_env(chat_runner),
            )
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(chat_registrar);
        }

        // Flow run.
        {
            use starter_server::auth::with_principal;
            let flow_run_registrar = routes::flow_run::registrar(routes::flow_run::FlowRunState {
                tools: mcp.tools.clone(),
            })
            .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(flow_run_registrar);
        }

        // Tools with auth + audit.
        let recorder: Arc<dyn ChangeRecorder> = Arc::new(PgChangeRecorder::new(pool));
        let tool_audit_prefixes = vec![
            "/api/v1/tools/".to_owned(),
            "/api/v1/admin/registry/tools/".to_owned(),
        ];
        let tools_recorder = recorder.clone();
        let tools_audit_prefixes = tool_audit_prefixes.clone();
        let tools_gated = tools_registrar.map_router(|r| {
            let audited = middleware::changelog_layer(
                r,
                middleware::ChangelogState {
                    recorder: tools_recorder,
                    tool_path_prefixes: tools_audit_prefixes,
                },
            );
            middleware::gate_tools(audited, auth.authenticator.clone(), engine)
        });
        app = app
            .merge_external(Router::new().nest("/api/v1", auth_routes))
            .merge(tools_gated);

        // Admin surface.
        {
            use starter_server::auth::{with_principal, with_role, with_scope};
            use starter_spi::auth::{Role, Scope};
            let read = routes::admin::admin_registrar(admin_state.clone())
                .map_router(|r| with_role(r, Role::Admin));
            let admin_invoke_recorder = recorder.clone();
            let admin_invoke_prefixes = tool_audit_prefixes.clone();
            let invoke = routes::admin::admin_invoke_registrar(admin_state.clone())
                .map_router(move |r| {
                    let audited = middleware::changelog_layer(
                        r,
                        middleware::ChangelogState {
                            recorder: admin_invoke_recorder,
                            tool_path_prefixes: admin_invoke_prefixes,
                        },
                    );
                    with_scope(with_role(audited, Role::Admin), Scope::new("admin:invoke"))
                });
            let admin_stream_recorder = recorder.clone();
            let admin_stream_prefixes = tool_audit_prefixes.clone();
            let invoke_stream = routes::admin::admin_invoke_stream_registrar(admin_state.clone())
                .map_router(move |r| {
                    let audited = middleware::changelog_layer(
                        r,
                        middleware::ChangelogState {
                            recorder: admin_stream_recorder,
                            tool_path_prefixes: admin_stream_prefixes,
                        },
                    );
                    with_scope(with_role(audited, Role::Admin), Scope::new("admin:invoke"))
                });
            let gated_admin = read
                .merge(invoke)
                .merge(invoke_stream)
                .map_router(|r| with_principal(r, auth.authenticator.clone()));
            app = app.merge(gated_admin);
        }
    } else {
        warn!(
            target: "rubix.boot",
            "RUBIX_DATABASE_URL unset — mounting tools router without auth/authz/audit gates",
        );
        let flow_run_registrar = routes::flow_run::registrar(routes::flow_run::FlowRunState {
            tools: mcp.tools.clone(),
        });
        app = app.merge(tools_registrar).merge(flow_run_registrar);

        app = app
            .merge(routes::admin::admin_registrar(admin_state.clone()))
            .merge(routes::admin::admin_invoke_registrar(admin_state.clone()))
            .merge(routes::admin::admin_invoke_stream_registrar(admin_state.clone()));
    }

    // OpenAPI projection.
    let admin_openapi_doc = std::sync::Arc::new(routes::catalog_to_openapi(
        app.catalog(),
        routes::OpenApiInfo {
            title: "rubix-agent (admin projection)".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: Some(
                "Strict projection of routes mounted through RouteRegistrar — \
                 the live router and this doc cannot drift."
                    .to_owned(),
            ),
        },
    ));
    app = app.merge(routes::admin::admin_openapi_registrar(admin_openapi_doc));

    let app = app
        .into_router()
        .layer(tower_http::cors::CorsLayer::very_permissive());

    health::serve(&cfg.bind, app).await
}
