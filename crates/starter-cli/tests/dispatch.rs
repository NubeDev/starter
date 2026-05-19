//! End-to-end: register the starter defaults, parse a fake CLI, and
//! verify the `health` subcommand actually hits a running test server.

use std::sync::Arc;

use prometheus::Registry;
use starter_cli::CommandRegistry;
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};

#[derive(Clone)]
struct EmptyState;

async fn spawn_test_server() -> TestApp {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry, metrics)
        .build();
    TestApp::spawn(router).await
}

fn root_command(registry: &CommandRegistry) -> clap::Command {
    clap::Command::new("starter").subcommands(registry.subcommands())
}

#[tokio::test]
async fn health_dispatch_against_real_server() {
    let app = spawn_test_server().await;
    let registry = CommandRegistry::new().register_starter_defaults();

    let matches = root_command(&registry).get_matches_from(vec![
        "starter",
        "health",
        "--base-url",
        &app.base_url,
    ]);
    registry.dispatch(&matches).await.expect("dispatch ok");

    app.shutdown().await;
}

#[tokio::test]
async fn openapi_dispatch_against_real_server() {
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));
    let doc = utoipa::openapi::OpenApiBuilder::new()
        .info(utoipa::openapi::InfoBuilder::new().title("clitest").build())
        .build();
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .with_metrics(registry, metrics)
        .with_openapi(doc)
        .build();
    let app = TestApp::spawn(router).await;

    let cli_registry = CommandRegistry::new().register_starter_defaults();
    let matches = root_command(&cli_registry).get_matches_from(vec![
        "starter",
        "openapi",
        "--base-url",
        &app.base_url,
    ]);
    cli_registry.dispatch(&matches).await.expect("dispatch ok");

    app.shutdown().await;
}

#[tokio::test]
async fn unknown_subcommand_user_facing_error() {
    let registry = CommandRegistry::new().register_starter_defaults();
    let matches = root_command(&registry)
        .try_get_matches_from(vec!["starter"])
        .expect("clap parses bare invocation");
    let err = registry.dispatch(&matches).await.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("no subcommand given"));
}
