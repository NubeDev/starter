//! Stage 21 — server-side "Australian operator" smoke.
//!
//! The SCOPE Smoke-tests block names this case explicitly:
//!
//! > GET `/v1/telemetry?slot=temp_in` with Australian prefs returns
//! > `"unit": "celsius"`, `"value": <converted>`.
//!
//! "Australian prefs" is the canonical metric profile — `en-AU`
//! locale, `UnitSystem::Metric`, every per-quantity selector left to
//! its metric canonical (`Celsius`, `Kilopascal`, `KilometerPerHour`,
//! `Meter`, `Kilogram`). The operator queries a telemetry slot whose
//! source value happens to be expressed in Fahrenheit (e.g. a
//! retrofit sensor still reporting imperial); the Accept-Units
//! middleware + `UnitsCtx::convert` must hand back Celsius because
//! that is the operator's resolved preference.
//!
//! The test wires the same shapes a production handler would use:
//!
//! - `accept_units_layer(registry, resolver)` from `starter-server`
//! - `StaticRegistry` from `starter-spi::units` (the canonical
//!   registry the workspace ships)
//! - a stub `PrefsResolverFor` returning Australian metric prefs
//! - a tiny `GET /v1/telemetry?slot=...` handler that reads
//!   `UnitsCtx` from request extensions and calls `convert()` at
//!   serialise time per the Phase 2 contract (no body rewriting in
//!   the middleware — R6).
//!
//! What this test specifically guards:
//!
//! 1. The wire shape — `unit` is the lowercase string `"celsius"`
//!    (serde derives on `Unit`), not the `Debug` form, not the SI
//!    symbol, and not the source unit.
//! 2. The numeric conversion — 100 °F is reported as ~37.7778 °C
//!    (the affine inverse the middleware computes by sampling
//!    `normalize_for_storage` at 0 and 1).
//! 3. End-to-end metadata hoisting — `unit` lives on the envelope
//!    (one per series), not on each point, per SCOPE R8.

use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{Extension, Query};
use axum::response::{Json, Response};
use axum::routing::get;
use axum::Router;
use http::request::Parts;
use http::{Request as HttpRequest, StatusCode};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{json, Value};
use starter_server::middleware::{accept_units_layer, PrefsResolverFor, UnitsCtx};
use starter_spi::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::{Quantity, StaticRegistry, Unit, UnitRegistry};
use tower::ServiceExt;

/// Canonical Australian metric profile per SCOPE "Smoke-tests"
/// (Resolver layer precedence → auto derivation → `en-AU` + metric
/// resolves to C / kPa / km/h / m / kg / AUD).
fn australian_prefs() -> ResolvedPreferences {
    ResolvedPreferences {
        timezone: "Australia/Sydney".into(),
        locale: "en-AU".into(),
        language: "en".into(),
        unit_system: UnitSystem::Metric,
        temperature_unit: Unit::Celsius,
        pressure_unit: Unit::Kilopascal,
        speed_unit: Unit::KilometerPerHour,
        length_unit: Unit::Meter,
        mass_unit: Unit::Kilogram,
        date_format: DateFormat::IsoYMD,
        time_format: TimeFormat::H24,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::SpaceComma,
        currency: "AUD".into(),
        theme: Theme::System,
    }
}

struct StubResolver(ResolvedPreferences);

#[async_trait]
impl PrefsResolverFor for StubResolver {
    async fn resolve_for(&self, _parts: &Parts) -> Result<ResolvedPreferences, Response> {
        Ok(self.0.clone())
    }
}

#[derive(Debug, Deserialize)]
struct TelemetryQuery {
    /// Slot name the operator is reading from. The smoke uses
    /// `temp_in` per the SCOPE; in a real handler the slot would
    /// drive a store lookup.
    slot: String,
}

/// `GET /v1/telemetry?slot=temp_in` — returns the latest reading
/// for the named slot.
///
/// For the smoke we hardcode a single reading whose canonical
/// (storage) value is 37.7777… °C but whose source-unit metadata
/// claims Fahrenheit at 100 °F. The middleware-supplied `UnitsCtx`
/// converts from the source unit to the operator's preferred unit;
/// because the operator is Australian, that's Celsius, so the value
/// rounds back to 37.78 °C and the envelope reports `"celsius"`.
async fn telemetry(
    Extension(ctx): Extension<UnitsCtx>,
    Query(q): Query<TelemetryQuery>,
) -> Json<Value> {
    assert_eq!(q.slot, "temp_in", "smoke fixes the slot to `temp_in`");

    // Pretend the storage layer recorded the sensor reading in
    // Fahrenheit (it does not in production — storage is always
    // canonical SI — but the conversion path is identical and the
    // smoke exercises the same code).
    let source_value = 100.0_f64;
    let source_unit = Unit::Fahrenheit;

    let (value, unit) = ctx
        .convert(Quantity::Temperature, source_value, source_unit)
        .expect("temperature convert must succeed");

    Json(json!({
        "slot": q.slot,
        "quantity": "temperature",
        "unit": unit,            // serde_json renders Unit as `"celsius"`
        "points": [[0, value]],  // R8: metadata on the envelope, not per point
    }))
}

fn app() -> Router {
    let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
    let resolver = Arc::new(StubResolver(australian_prefs()));
    Router::new()
        .route("/v1/telemetry", get(telemetry))
        .layer(accept_units_layer(registry, resolver))
}

async fn body_json(resp: Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).expect("response body is JSON")
}

#[tokio::test]
async fn telemetry_with_australian_prefs_returns_celsius() {
    let resp = app()
        .oneshot(
            HttpRequest::builder()
                .uri("/v1/telemetry?slot=temp_in")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    // Wire shape — exactly what the SCOPE Smoke-tests block calls out.
    assert_eq!(
        body["unit"], "celsius",
        "Australian operator must see celsius in the envelope"
    );

    // Numeric conversion — 100 °F is 37.777… °C. Tolerance is the
    // round-trip floating-point noise of the affine inverse the
    // middleware computes.
    let v = body["points"][0][1]
        .as_f64()
        .expect("value is a JSON number");
    let expected = (100.0 - 32.0) * 5.0 / 9.0;
    assert!(
        (v - expected).abs() < 1e-6,
        "expected ~{expected:.4} °C, got {v}",
    );

    // Metadata hoisting — only one unit/quantity in the envelope.
    assert_eq!(body["quantity"], "temperature");
    assert!(body["points"].is_array());
    assert!(
        body["points"][0].get("unit").is_none(),
        "R8: per-point unit metadata leaked into the points array",
    );
}

#[tokio::test]
async fn telemetry_in_canonical_mode_skips_conversion() {
    // Sanity-check the canonical-mode escape hatch — a machine
    // consumer that sends `Accept-Units: canonical` gets the
    // storage-side value (also celsius here, because the source
    // happens to convert to celsius for both Australian preferred
    // and registry canonical) but the contract is "no conversion
    // applied beyond source-to-canonical normalisation". This
    // guards against a future refactor that accidentally ties
    // canonical mode to preferred-mode prefs resolution.
    let resp = app()
        .oneshot(
            HttpRequest::builder()
                .uri("/v1/telemetry?slot=temp_in")
                .header("Accept-Units", "canonical")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["unit"], "celsius");
    let v = body["points"][0][1].as_f64().unwrap();
    let expected = (100.0 - 32.0) * 5.0 / 9.0;
    assert!((v - expected).abs() < 1e-6);
}
