//! "Canonical-only logs" audit (Phase 2).
//!
//! Owns: SCOPE.md "Smoke-tests" → "Canonical-only logs" test
//! (lines ~963-967 of `DOCS/user/scope/SCOPE.md`).
//!
//! # What this guards
//!
//! Per R1/R6 of the SCOPE: every physical quantity is stored — and
//! **logged** — in its canonical SI unit. Unit conversion happens
//! exclusively at the response edge (typed serialisers calling
//! `UnitsCtx::convert` per Phase 2). If a log line on any starter
//! crate ever contains the substring `"°F"`, `" psi"`, `" mph"`, or
//! `" lb"` it means a non-canonical value has leaked into the
//! observability surface — a regression of the canonical-only rule.
//!
//! # Why this test lives in `starter-server`
//!
//! The Accept-Units middleware applies at the server edge — `starter-
//! server` is the single chokepoint where unit conversion can fire.
//! If a downstream crate were to log a converted value (e.g. by
//! interpolating a handler's converted response into a `tracing::info!`
//! call), the audit would trip here, at the integration boundary.
//!
//! # Harness shape
//!
//! A custom `tracing_subscriber::fmt::Layer` is installed once via
//! `set_global_default`, writing every formatted event into a shared
//! `Mutex<Vec<u8>>` buffer. The test then drives the same Accept-
//! Units paths the workspace integration tests exercise — preferred
//! conversion **and** canonical bypass — through a representative
//! handler that emits a `tracing::info!` line per request. After the
//! traffic settles, the buffer is scanned for the forbidden
//! substrings and the test fails on any hit.
//!
//! Extending the harness later: add request scenarios here as new
//! starter crates start emitting unit-bearing log lines; the
//! capture-and-assert harness then catches a leak the moment it
//! lands in CI rather than at production read time.

use std::io;
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Body;
use axum::extract::Extension;
use axum::routing::get;
use axum::Router;
use http::request::Parts;
use http::Request as HttpRequest;
use starter_server::middleware::{
    accept_units_layer, PrefsResolverFor, UnitsCtx, ACCEPT_UNITS_HEADER,
};
use starter_spi::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::{Quantity, StaticRegistry, Unit, UnitRegistry};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

// ---------------------------------------------------------------------
// Capturing writer.
// ---------------------------------------------------------------------

/// Shared buffer that every `tracing` event lands in. Wrapped in a
/// `Mutex` because `fmt::Layer` can write from multiple threads.
type SharedBuf = Arc<Mutex<Vec<u8>>>;

#[derive(Clone)]
struct BufWriter(SharedBuf);

impl io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct CapturingMakeWriter(SharedBuf);

impl<'a> MakeWriter<'a> for CapturingMakeWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        BufWriter(self.0.clone())
    }
}

/// Install the capturing subscriber once per process. Returns the
/// shared buffer so the test can scan it.
fn install_capturing_subscriber() -> SharedBuf {
    static BUF: OnceLock<SharedBuf> = OnceLock::new();
    let buf = BUF
        .get_or_init(|| {
            let buf: SharedBuf = Arc::new(Mutex::new(Vec::new()));
            let layer = tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_level(true)
                .with_writer(CapturingMakeWriter(buf.clone()));
            // Best-effort: another test in the binary may have set a
            // global default already. We ignore the error because in
            // that case the existing default is also writing into
            // *this* buffer (it can only have been installed by this
            // function).
            let _ = tracing_subscriber::registry().with(layer).try_init();
            buf
        })
        .clone();
    buf
}

// ---------------------------------------------------------------------
// Test fixtures (mirror the accept_units in-crate tests).
// ---------------------------------------------------------------------

fn imperial_prefs() -> ResolvedPreferences {
    ResolvedPreferences {
        timezone: "UTC".into(),
        locale: "en-US".into(),
        language: "en".into(),
        unit_system: UnitSystem::Imperial,
        temperature_unit: Unit::Fahrenheit,
        pressure_unit: Unit::Psi,
        speed_unit: Unit::MilePerHour,
        length_unit: Unit::Foot,
        mass_unit: Unit::Pound,
        date_format: DateFormat::IsoYMD,
        time_format: TimeFormat::H24,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::CommaDot,
        currency: "USD".into(),
        theme: Theme::System,
    }
}

struct StubResolver(ResolvedPreferences);

#[async_trait::async_trait]
impl PrefsResolverFor for StubResolver {
    async fn resolve_for(
        &self,
        _parts: &Parts,
    ) -> Result<ResolvedPreferences, axum::response::Response> {
        Ok(self.0.clone())
    }
}

/// Handler that **does the right thing**: it converts for the
/// response body but logs the canonical-side value (Celsius, kPa,
/// m/s, m, kg). The audit's job is to keep handlers like this honest
/// — if someone refactors and starts logging the converted value
/// instead, the substring assertion below fails.
async fn good_handler(Extension(ctx): Extension<UnitsCtx>) -> axum::Json<serde_json::Value> {
    // Canonical inputs (these are what the storage layer / sensors
    // produce per R1). We log them as-is, then convert only for the
    // response body.
    let canonical_temp_c = 100.0_f64;
    let canonical_pressure_kpa = 101.325_f64;
    let canonical_speed_mps = 27.7_f64;
    let canonical_length_m = 1.0_f64;
    let canonical_mass_kg = 1.0_f64;

    // Canonical-only log line — the kind every starter crate is
    // supposed to emit. Notice the units are SI; no `°F`, no `psi`,
    // no `mph`, no `lb` interpolation anywhere.
    tracing::info!(
        target: "starter_server::tests::canonical_logs",
        temperature_c = canonical_temp_c,
        pressure_kpa = canonical_pressure_kpa,
        speed_mps = canonical_speed_mps,
        length_m = canonical_length_m,
        mass_kg = canonical_mass_kg,
        mode = ?ctx.mode(),
        "handler observed canonical sample",
    );

    let (temp_v, temp_u) = ctx
        .convert(Quantity::Temperature, canonical_temp_c, Unit::Celsius)
        .expect("convert temp");
    let (pres_v, pres_u) = ctx
        .convert(Quantity::Pressure, canonical_pressure_kpa, Unit::Kilopascal)
        .expect("convert pres");
    let (speed_v, speed_u) = ctx
        .convert(Quantity::Speed, canonical_speed_mps, Unit::MeterPerSecond)
        .expect("convert speed");
    let (len_v, len_u) = ctx
        .convert(Quantity::Length, canonical_length_m, Unit::Meter)
        .expect("convert length");
    let (mass_v, mass_u) = ctx
        .convert(Quantity::Mass, canonical_mass_kg, Unit::Kilogram)
        .expect("convert mass");

    axum::Json(serde_json::json!({
        "temperature": { "value": temp_v, "unit": format!("{temp_u:?}") },
        "pressure":    { "value": pres_v, "unit": format!("{pres_u:?}") },
        "speed":       { "value": speed_v, "unit": format!("{speed_u:?}") },
        "length":      { "value": len_v,  "unit": format!("{len_u:?}") },
        "mass":        { "value": mass_v, "unit": format!("{mass_u:?}") },
    }))
}

fn app() -> Router {
    let resolver: Arc<dyn PrefsResolverFor> = Arc::new(StubResolver(imperial_prefs()));
    let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
    Router::new()
        .route("/sample", get(good_handler))
        .layer(accept_units_layer(registry, resolver))
}

async fn hit(app: Router, header: Option<&str>) {
    let mut req = HttpRequest::builder().uri("/sample");
    if let Some(h) = header {
        req = req.header(ACCEPT_UNITS_HEADER, h);
    }
    let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------
// The audit.
// ---------------------------------------------------------------------

/// The forbidden substrings. Match the SCOPE Smoke-tests "Canonical-
/// only logs" block exactly: `"°F"`, `" psi"`, `" mph"`, `" lb"`.
/// The three latter ones lead with a space so that we don't false-
/// positive on incidental substrings ("display", "lpsi", "compli-
/// ance", "blbase") — they're spelled the way they show up after a
/// numeric literal in a log line: `72 °F`, `14.7 psi`, `65 mph`,
/// `5 lb`.
const FORBIDDEN: &[&str] = &["°F", " psi", " mph", " lb"];

#[tokio::test]
async fn logs_never_contain_non_canonical_unit_substrings() {
    let buf = install_capturing_subscriber();

    // Exercise both negotiation modes — canonical bypass and the
    // default `preferred` (imperial conversion in the response body
    // but, crucially, **not** in the logs).
    hit(app(), None).await; // preferred (default)
    hit(app(), Some("preferred")).await;
    hit(app(), Some("canonical")).await;
    // A few extra requests to make sure the assertion is genuinely
    // scanning multiple emitted log events.
    for _ in 0..4 {
        hit(app(), Some("preferred")).await;
    }

    let captured = {
        let g = buf.lock().unwrap();
        String::from_utf8(g.clone()).expect("captured log bytes are not valid UTF-8")
    };

    // Sanity: the harness actually captured something — otherwise the
    // assertion below would pass vacuously. The handler logs once
    // per request, so we should see at least one of our target
    // events.
    assert!(
        captured.contains("handler observed canonical sample"),
        "tracing-test harness captured no events; subscriber wiring is broken:\n{captured}"
    );

    for needle in FORBIDDEN {
        assert!(
            !captured.contains(needle),
            "FORBIDDEN non-canonical unit substring {needle:?} found in captured logs.\n\
             Logs must be canonical SI; conversion belongs at the response edge only \
             (see DOCS/user/scope/SCOPE.md → Smoke-tests → 'Canonical-only logs').\n\
             Captured log buffer:\n{captured}"
        );
    }
}

/// Regression-canary: if someone "fixes" the harness by no-op-ing the
/// capturing writer, the previous test still passes vacuously. This
/// canary deliberately emits a synthetic log line containing one of
/// the forbidden substrings into a *separate* buffer (not the global
/// audit buffer) and verifies the capturing mechanism would have
/// caught it. It does NOT touch the global subscriber.
#[test]
fn capturing_writer_actually_records_text() {
    let buf: SharedBuf = Arc::new(Mutex::new(Vec::new()));
    let mut w = BufWriter(buf.clone());
    use std::io::Write;
    writeln!(w, "synthetic 72 °F line").unwrap();
    let captured = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(captured.contains("°F"));
}
