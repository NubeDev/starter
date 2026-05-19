//! Smoke test: the public re-exports compile from outside. Locks the
//! trait shapes so a careless change in `starter-spi` surfaces here
//! before it surfaces in downstream consumers.

use starter_spi::{
    ai::{
        AiRunner, CliCfg, Event, EventKind, PermissionMode, Provider, RestCfg, RunResult,
        RunnerError, RunnerInput, SessionId,
    },
    auth::{Authenticator, Principal, Role, Scope},
    filter::{Filter, Predicate},
    secrets::{Secret, SecretError, SecretStore},
    sort::{Direction, Sort},
    Cursor, Error, Id, Page, Result,
};

#[test]
fn types_are_reachable() {
    fn _accept_error(_: Error) {}
    fn _accept_result(_: Result<()>) {}
    fn _accept_page(_: Page<String>) {}
    fn _accept_cursor(_: Cursor) {}
    fn _accept_principal(_: Principal) {}
    fn _accept_role(_: Role) {}
    fn _accept_scope(_: Scope) {}
    fn _accept_sort(_: Sort) {}
    fn _accept_direction(_: Direction) {}
    fn _accept_filter(_: Filter) {}
    fn _accept_predicate(_: Predicate) {}
    fn _accept_secret(_: Secret) {}
    fn _accept_secret_err(_: SecretError) {}
    fn _accept_runner_err(_: RunnerError) {}
    fn _accept_provider(_: Provider) {}
    fn _accept_run_result(_: RunResult) {}
    fn _accept_event(_: Event) {}
    fn _accept_event_kind(_: EventKind) {}
    fn _accept_runner_input(_: RunnerInput) {}
    fn _accept_cli_cfg(_: CliCfg) {}
    fn _accept_rest_cfg(_: RestCfg) {}
    fn _accept_perm_mode(_: PermissionMode) {}
    fn _accept_session_id(_: SessionId) {}

    struct User;
    let _id: Id<User> = Id::new();
}

#[test]
fn principal_carries_role_and_scopes() {
    let p = Principal {
        subject: "u1".into(),
        role: Role::Admin,
        scopes: vec![Scope::new("read:metrics")],
        extra: serde_json::Value::Null,
    };
    assert_eq!(p.role, Role::Admin);
    assert_eq!(p.scopes.len(), 1);
}

#[test]
fn secret_redacts_in_debug() {
    let s = Secret::new("hunter2");
    assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
    assert_eq!(s.expose(), "hunter2");
}

#[test]
fn sort_builders() {
    let s = Sort::desc("created_at");
    assert_eq!(s.field, "created_at");
    assert_eq!(s.direction, Direction::Desc);
}

#[test]
fn filter_chains_predicates() {
    let f = Filter::new()
        .and(Predicate::Eq {
            field: "name".into(),
            value: serde_json::json!("alice"),
        })
        .and(Predicate::In {
            field: "role".into(),
            values: vec![serde_json::json!("admin")],
        });
    assert_eq!(f.predicates.len(), 2);
}

// Object-safety smoke checks: these trait objects must be constructible
// for the trait to be usable across an FFI / boxed-impl boundary.
fn _authenticator_is_object_safe(_: Box<dyn Authenticator>) {}
fn _secret_store_is_object_safe(_: Box<dyn SecretStore>) {}
fn _ai_runner_is_object_safe(_: Box<dyn AiRunner>) {}

// `ThemeStore` must stay object-safe so handlers can hold an
// `Arc<dyn ThemeStore>` without per-backend generics.
fn _theme_store_is_object_safe(_: Box<dyn starter_spi::ui::theme::ThemeStore>) {}

#[test]
fn theme_dtos_round_trip_json() {
    use starter_spi::ui::theme::{ShellConfig, ThemeDocument, ThemeSaveInput, ThemeStyles};
    let doc = ThemeDocument {
        theme_styles: ThemeStyles {
            light: [("primary".into(), "oklch(0.55 0.22 257)".into())]
                .into_iter()
                .collect(),
            dark: [("primary".into(), "oklch(0.72 0.18 257)".into())]
                .into_iter()
                .collect(),
        },
        shell: ShellConfig {
            nav_title: "My App".into(),
            hide_features: vec!["page-builder".into()],
        },
        logo_url: Some("/api/v1/ui/theme/logo".into()),
        favicon_url: None,
    };
    let json = serde_json::to_string(&doc).unwrap();
    let parsed: ThemeDocument = serde_json::from_str(&json).unwrap();
    assert_eq!(doc, parsed);

    // `favicon_url: None` is omitted from the serialised form.
    assert!(!json.contains("favicon_url"));

    // Save input is `{ theme_styles, shell }`.
    let input = ThemeSaveInput {
        theme_styles: doc.theme_styles.clone(),
        shell: doc.shell.clone(),
    };
    let saved = serde_json::to_string(&input).unwrap();
    assert!(saved.contains("\"theme_styles\""));
    assert!(saved.contains("\"shell\""));
}

#[test]
fn theme_validator_rejects_unsafe_values() {
    use starter_spi::ui::theme::{validate_save_input, ShellConfig, ThemeSaveInput, ThemeStyles};
    let bad = ThemeSaveInput {
        theme_styles: ThemeStyles {
            light: [("primary".into(), "url(http://evil)".into())]
                .into_iter()
                .collect(),
            ..Default::default()
        },
        shell: ShellConfig::default(),
    };
    let err = validate_save_input(&bad).unwrap_err();
    assert_eq!(err.fragment, "url(");
    assert_eq!(err.key, "light.primary");
}
