//! Smoke test 3 — config-guarded construction.
//!
//! The consumer flips the integration off by changing one config value
//! (env var or config file) — never by recompiling. The pattern is the
//! `if`-around-`.register(...)` from R3:
//!
//! ```ignore
//! if cfg.slack.enabled {
//!     services = services.register(SlackSocketModeService::new(slack));
//! }
//! ```
//!
//! The compiled binary keeps both branches; runtime cost of disabling
//! Slack is one absent `Service` in the registry, nothing more.
//!
//! This test exercises that pattern directly: same binary, same env
//! var, two outcomes — registered with the var set, absent when unset.
//! If a provider crate ever required compile-time guarding (a Cargo
//! feature, a `cfg!` macro check), this test could not flip the result
//! without rebuilding.

use prometheus::Registry;
use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};
use starter_spi::service::ServiceRegistry;
use starter_spi::SecretString;

fn build_registry_from_env(env_var: &str) -> ServiceRegistry {
    // The body of this function mirrors a consumer's `main.rs`. Note
    // the absence of any cargo-feature check, the absence of any
    // dynamic loader. Just `std::env::var` + `if`.
    let mut services = ServiceRegistry::new();
    if std::env::var(env_var).map(|v| v == "1").unwrap_or(false) {
        services = services.register(SlackSocketModeService::new(SlackSocketModeConfig {
            app_token: SecretString::from("xapp-test".to_string()),
            base_url: SlackSocketModeConfig::default_base_url(),
        }));
    }
    services
}

#[test]
fn env_var_flips_registration_without_recompile() {
    // Use a per-test env var name so parallel tests don't trample
    // each other.
    let var = "STARTER_SMOKE_3_SLACK_ENABLED";

    // SAFETY: the harness owns the process env; tests in this binary
    // file run sequentially within this `#[test]` so the var is not
    // racing other smoke-3 cases.
    std::env::remove_var(var);
    let off = build_registry_from_env(var);
    assert!(
        off.is_empty(),
        "with env unset, Slack must NOT be registered; got {:?}",
        off.names(),
    );

    std::env::set_var(var, "1");
    let on = build_registry_from_env(var);
    assert_eq!(on.len(), 1, "with env set, Slack must be registered");
    assert!(on.names().contains(&starter_service_slack::SERVICE_NAME));

    // Cleanup so this var does not bleed into other tests in the
    // binary (cargo runs integration-test files in their own process,
    // but be defensive).
    std::env::remove_var(var);

    // Silence the unused-import warning when prometheus isn't pulled
    // into the on-path of this file.
    let _ = Registry::new();
}
