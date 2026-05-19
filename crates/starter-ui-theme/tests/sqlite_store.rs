//! End-to-end coverage of the sqlite ThemeStore: load default,
//! save round-trip, asset upload + read, delete asset.

#![cfg(feature = "sqlite")]

use starter_spi::ui::theme::{ShellConfig, ThemeSaveInput, ThemeStore, ThemeStyles};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};
use starter_ui_theme::store::SqliteThemeStore;

static UI_THEME_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/ui_theme_sqlite");

async fn fresh_store() -> SqliteThemeStore {
    let pool: Pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "ui_theme",
            migrator: &UI_THEME_MIGRATOR,
        })
        .run()
        .await
        .expect("ui_theme migration applies");
    SqliteThemeStore::new(pool)
}

#[tokio::test]
async fn load_on_empty_returns_default_document() {
    let store = fresh_store().await;
    let doc = store.load().await.expect("load");
    assert!(doc.theme_styles.light.is_empty());
    assert!(doc.theme_styles.dark.is_empty());
    assert_eq!(doc.shell.nav_title, "");
    assert!(doc.logo_url.is_none());
    assert!(doc.favicon_url.is_none());
}

#[tokio::test]
async fn save_then_load_round_trips() {
    let store = fresh_store().await;
    let input = ThemeSaveInput {
        theme_styles: ThemeStyles {
            light: [
                ("primary".into(), "oklch(0.55 0.22 257)".into()),
                ("background".into(), "oklch(1 0 0)".into()),
            ]
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
    };
    let saved = store.save(input.clone()).await.expect("save");
    assert_eq!(saved.theme_styles, input.theme_styles);
    assert_eq!(saved.shell, input.shell);

    let reloaded = store.load().await.expect("load");
    assert_eq!(reloaded.theme_styles, input.theme_styles);
    assert_eq!(reloaded.shell, input.shell);
}

#[tokio::test]
async fn save_then_overwrite_replaces_styles() {
    let store = fresh_store().await;
    store
        .save(ThemeSaveInput {
            theme_styles: ThemeStyles {
                light: [("primary".into(), "red".into())].into_iter().collect(),
                ..Default::default()
            },
            shell: ShellConfig::default(),
        })
        .await
        .unwrap();
    let second = ThemeSaveInput {
        theme_styles: ThemeStyles {
            light: [("primary".into(), "blue".into())].into_iter().collect(),
            ..Default::default()
        },
        shell: ShellConfig {
            nav_title: "Second".into(),
            hide_features: vec![],
        },
    };
    store.save(second.clone()).await.unwrap();
    let doc = store.load().await.unwrap();
    assert_eq!(
        doc.theme_styles.light.get("primary").map(String::as_str),
        Some("blue"),
    );
    assert_eq!(doc.shell.nav_title, "Second");
}

#[tokio::test]
async fn logo_upload_read_delete_round_trip() {
    let store = fresh_store().await;
    assert!(store.read_logo().await.unwrap().is_none());

    let url = store
        .put_logo(b"PNGDATA".to_vec(), "image/png")
        .await
        .expect("put logo");
    assert_eq!(url, "/api/v1/ui/theme/logo");

    let (bytes, mime) = store.read_logo().await.unwrap().expect("logo present");
    assert_eq!(bytes, b"PNGDATA");
    assert_eq!(mime, "image/png");

    // load() now reports the URL.
    let doc = store.load().await.unwrap();
    assert_eq!(doc.logo_url.as_deref(), Some("/api/v1/ui/theme/logo"));

    store.delete_logo().await.unwrap();
    assert!(store.read_logo().await.unwrap().is_none());
    let doc = store.load().await.unwrap();
    assert!(doc.logo_url.is_none());
}

#[tokio::test]
async fn favicon_upload_read_delete_round_trip() {
    let store = fresh_store().await;
    let url = store
        .put_favicon(b"ICODATA".to_vec(), "image/x-icon")
        .await
        .unwrap();
    assert_eq!(url, "/api/v1/ui/theme/favicon");
    let (bytes, mime) = store.read_favicon().await.unwrap().unwrap();
    assert_eq!(bytes, b"ICODATA");
    assert_eq!(mime, "image/x-icon");

    store.delete_favicon().await.unwrap();
    assert!(store.read_favicon().await.unwrap().is_none());
}

#[tokio::test]
async fn delete_logo_on_empty_is_no_op() {
    let store = fresh_store().await;
    // No row yet — delete must not error.
    store.delete_logo().await.expect("delete is idempotent");
    store.delete_favicon().await.expect("delete is idempotent");
}
