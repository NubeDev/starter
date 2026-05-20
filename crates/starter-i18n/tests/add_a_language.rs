//! Stage 21 — "Add a language" smoke.
//!
//! The SCOPE Smoke-tests block "Add a language" reads: drop
//! `fr.json` into `starter-i18n/catalogs/starter/`, bump the i18n
//! crate's version, rebuild — the manifest gains the new language;
//! clients fetching it get the new bundle.
//!
//! Rather than literally bumping the version (which would couple
//! this test to a workspace-wide cargo state), the test exercises
//! the in-process equivalent the SCOPE describes:
//!
//! 1. Start from [`starter_i18n::platform::starter_bundle`] — the
//!    en + es bundle the crate ships compiled in.
//! 2. Parse a French translation as a fresh
//!    [`starter_i18n::catalog::Catalog`] (the JSON shape any drop-in
//!    `fr.json` would use).
//! 3. Insert it into the bundle under the `fr` [`LanguageTag`].
//! 4. Hit the `routes` feature's `GET /v1/i18n/manifest` and
//!    `GET /v1/i18n/catalogs/{lang}` endpoints.
//! 5. Assert the manifest now lists `fr` with a 16-char fingerprint
//!    and the catalog endpoint serves the French bytes back.
//!
//! Behaviour-wise this is identical to dropping the JSON on disk
//! and rebuilding — the loader path is the same `Catalog::from_json_str`
//! that `include_str!` feeds in `platform.rs`. The "no-on-disk-dir"
//! posture is intentional: the smoke verifies the wire surface, not
//! the loader's filesystem walk (which has its own unit tests on
//! `Catalog::from_file`).

#![cfg(feature = "routes")]

use std::sync::Arc;

use axum::body::Body;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use starter_i18n::bundle::MessageBundle;
use starter_i18n::catalog::Catalog;
use starter_i18n::platform::starter_bundle;
use starter_i18n::routes::router;
use starter_spi::i18n::LanguageTag;
use tower::ServiceExt;

/// A tiny French translation of two well-known starter keys. The
/// SCOPE Smoke-tests block does not constrain the contents — only
/// that the manifest gains the language and a client fetching it
/// gets the new bundle — so two entries are enough to prove the
/// load-and-serve round trip.
const FR_JSON: &str = r#"{
  "starter.auth.login.heading": "Connexion",
  "starter.auth.login.button.label": "Se connecter"
}"#;

fn fr_tag() -> LanguageTag {
    LanguageTag::parse("fr").expect("`fr` is a valid BCP-47 tag")
}

fn bundle_with_fr() -> Arc<MessageBundle> {
    let mut bundle: MessageBundle = starter_bundle();
    let fr = Catalog::from_json_str(FR_JSON).expect("FR_JSON parses as a Catalog");
    bundle.insert(fr_tag(), fr);
    Arc::new(bundle)
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("response body is JSON")
}

#[tokio::test]
async fn manifest_gains_fr_after_insert() {
    let app = router(bundle_with_fr());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/i18n/manifest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let obj = body.as_object().expect("manifest is a JSON object");

    // The seed `en` + `es` are still there — adding a language
    // must be additive, never a replace.
    assert!(obj.contains_key("en"), "en disappeared from manifest");
    assert!(obj.contains_key("es"), "es disappeared from manifest");

    // The new language landed with a 16-char hex fingerprint, the
    // shape SCOPE Phase 3 locks in.
    let fp = obj
        .get("fr")
        .expect("manifest does not list fr")
        .as_str()
        .expect("fr fingerprint is a string");
    assert_eq!(fp.len(), 16, "fr fingerprint must be 16 chars, got {fp:?}");
    assert!(
        fp.chars().all(|c| c.is_ascii_hexdigit()),
        "fr fingerprint must be lowercase hex, got {fp:?}"
    );
}

#[tokio::test]
async fn catalog_endpoint_serves_fr_bytes() {
    let app = router(bundle_with_fr());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/i18n/catalogs/fr")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "client fetching the new language must get a 200"
    );
    let body = body_json(resp).await;
    let obj = body.as_object().expect("catalog is a JSON object");
    assert_eq!(
        obj.get("starter.auth.login.heading")
            .and_then(|v| v.as_str()),
        Some("Connexion"),
        "client did not get the French bytes back"
    );
}

#[tokio::test]
async fn fingerprint_changes_when_catalog_changes() {
    // Stability of the immutable-cache contract: two bundles that
    // ship different French bytes must produce different fingerprints
    // so a CDN cannot serve stale catalog data under a now-incorrect
    // content-addressed URL.
    let app1 = router(bundle_with_fr());
    let resp1 = app1
        .oneshot(
            Request::builder()
                .uri("/v1/i18n/manifest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let fp1 = body_json(resp1).await["fr"].as_str().unwrap().to_owned();

    let other_fr =
        Catalog::from_json_str(r#"{ "starter.auth.login.heading": "Différent" }"#).unwrap();
    let mut bundle2 = starter_bundle();
    bundle2.insert(fr_tag(), other_fr);
    let app2 = router(Arc::new(bundle2));
    let resp2 = app2
        .oneshot(
            Request::builder()
                .uri("/v1/i18n/manifest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let fp2 = body_json(resp2).await["fr"].as_str().unwrap().to_owned();

    assert_ne!(
        fp1, fp2,
        "fr fingerprint must change when the catalog bytes change",
    );
}
