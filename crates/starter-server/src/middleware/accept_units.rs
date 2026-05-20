//! `Accept-Units` content-negotiation middleware (Phase 2).
//!
//! Owns: SCOPE.md "Middleware" — Accept-Units negotiation and
//! `UnitsCtx` threading — and the conversion contract laid out in
//! the Phase 2 decision lock (per-series wire shape per R8;
//! handler-level [`UnitsCtx::convert`] call shape; **no** response
//! body rewriting per R6).
//!
//! # What the middleware does
//!
//! 1. Parses the `Accept-Units` request header. Two values are
//!    recognised:
//!    - `preferred` (default when the header is missing) — caller
//!      wants response values in their resolved unit preferences.
//!    - `canonical` — caller wants raw canonical SI values (e.g. an
//!      audit / "canonical-only logs" consumer per the SCOPE Phase 2
//!      audit block); the middleware disables conversion and
//!      handlers emit canonical units verbatim.
//! 2. Resolves the caller's preferences **once** via the injected
//!    [`PrefsResolverFor`] (typically a thin adapter over
//!    `starter-prefs`' resolver + store). One DB round-trip per
//!    request; the resolved view is then handed to every handler
//!    through request extensions.
//! 3. Inserts a [`UnitsCtx`] into request extensions for the
//!    downstream handler to read. The middleware never touches
//!    response bodies — per R6 conversion is a handler concern that
//!    typed serialisers opt in to by calling
//!    [`UnitsCtx::convert`].
//! 4. Appends `Accept-Units` to the response `Vary` header so caches
//!    key the response on the negotiation axis (matches the
//!    SCOPE Phase 2 CDN-cache caveat — without `Vary`, a CDN would
//!    happily serve a canonical body to a `preferred` client).
//!
//! # What the middleware does NOT do
//!
//! Per R6 of `DOCS/user/scope/SCOPE.md`: response bodies are emitted
//! by typed serialisers, not rewritten in middleware. SSE / streaming
//! responses are explicitly out of scope here. Handlers that emit
//! unit-bearing values call [`UnitsCtx::convert`] at serialisation
//! time; anything that doesn't is presumed unit-agnostic.

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use axum::Router;
use futures::future::BoxFuture;
use http::header::{HeaderName, HeaderValue, VARY};
use http::request::Parts;
use starter_spi::preferences::ResolvedPreferences;
use starter_spi::units::{normalize_for_storage, Quantity, Unit, UnitError, UnitRegistry};
use tower::{Layer, Service};

/// Header name driving the negotiation: `Accept-Units`.
pub const ACCEPT_UNITS_HEADER: HeaderName = HeaderName::from_static("accept-units");

/// Negotiated response mode for the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitsMode {
    /// Handlers convert values into the caller's resolved unit
    /// preferences (default — missing header lands here per the
    /// Phase 2 lock).
    Preferred,
    /// Handlers emit canonical SI values verbatim; conversion is
    /// suppressed. The "canonical-only logs" audit path per the
    /// SCOPE Phase 2 audit block.
    Canonical,
}

impl UnitsMode {
    /// Parse the raw `Accept-Units` header value. Unknown values
    /// fall back to [`Self::Preferred`] — defensive default; the
    /// alternative (415-style rejection) would break clients that
    /// send the header for forward-compat reasons.
    pub fn parse(raw: &str) -> Self {
        // Tolerate surrounding whitespace and case differences. The
        // header is a single token per the SCOPE Phase 2 spec — we
        // do not honour quality values.
        if raw.trim().eq_ignore_ascii_case("canonical") {
            Self::Canonical
        } else {
            Self::Preferred
        }
    }
}

/// Resolves preferences for the in-flight request. Implementors
/// typically read a `Principal` (or equivalent) from request
/// extensions and delegate to `starter-prefs`' resolver + store.
///
/// Returning `Err(Response)` short-circuits the request with the
/// supplied response (e.g. `401` for missing principal). On `Ok` the
/// middleware builds a [`UnitsCtx`] and continues.
#[async_trait]
pub trait PrefsResolverFor: Send + Sync {
    /// Resolve preferences for the request whose parts are `parts`.
    /// `parts` is read-only — handlers add their own extensions
    /// after the middleware returns.
    async fn resolve_for(&self, parts: &Parts) -> Result<ResolvedPreferences, Response>;
}

/// Per-request units context. Stashed in request extensions by
/// [`accept_units_layer`]; handlers read it with
/// `axum::Extension<UnitsCtx>` (cheap to clone — three `Arc`s plus a
/// copy of [`UnitsMode`]).
#[derive(Clone)]
pub struct UnitsCtx {
    mode: UnitsMode,
    prefs: Arc<ResolvedPreferences>,
    registry: Arc<dyn UnitRegistry + Send + Sync>,
}

impl std::fmt::Debug for UnitsCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnitsCtx")
            .field("mode", &self.mode)
            .field("prefs", &*self.prefs)
            .finish_non_exhaustive()
    }
}

impl UnitsCtx {
    /// Construct directly — primarily for tests + adapters that want
    /// to forge a context without running the middleware.
    pub fn new(
        mode: UnitsMode,
        prefs: Arc<ResolvedPreferences>,
        registry: Arc<dyn UnitRegistry + Send + Sync>,
    ) -> Self {
        Self {
            mode,
            prefs,
            registry,
        }
    }

    /// Negotiated mode for this request.
    pub fn mode(&self) -> UnitsMode {
        self.mode
    }

    /// Caller's resolved preferences (one DB round-trip per request).
    pub fn prefs(&self) -> &ResolvedPreferences {
        &self.prefs
    }

    /// The caller's preferred unit for `quantity`, ignoring mode.
    /// Useful when a handler wants to label a canonical value with
    /// the user's preferred unit for display alongside.
    pub fn user_unit(&self, quantity: Quantity) -> Unit {
        match quantity {
            Quantity::Temperature => self.prefs.temperature_unit,
            Quantity::Pressure => self.prefs.pressure_unit,
            Quantity::Speed => self.prefs.speed_unit,
            Quantity::Length => self.prefs.length_unit,
            Quantity::Mass => self.prefs.mass_unit,
        }
    }

    /// Target unit for a converted response value: registry canonical
    /// in [`UnitsMode::Canonical`], user's preference in
    /// [`UnitsMode::Preferred`].
    pub fn target_unit(&self, quantity: Quantity) -> Unit {
        match self.mode {
            UnitsMode::Canonical => self
                .registry
                .get(quantity)
                .map(|d| d.canonical)
                .unwrap_or_else(|| self.user_unit(quantity)),
            UnitsMode::Preferred => self.user_unit(quantity),
        }
    }

    /// Convert `value` (expressed in `source_unit`) into the caller's
    /// target unit, returning the converted numeric and the unit it
    /// is now expressed in. Typed serialisers call this at emit
    /// time per the Phase 2 decision lock — the middleware itself
    /// never rewrites bodies per R6.
    ///
    /// Internally: `source_unit → canonical` via
    /// [`starter_spi::units::normalize_for_storage`] (the "in
    /// reverse" half of the round trip described in the SCOPE Phase
    /// 2 block), then `canonical → target_unit` via the affine
    /// inverse derived by sampling `normalize_for_storage` at
    /// `0.0` and `1.0`. The inverse is exact for every conversion
    /// `uom` exposes here (linear scale + optional zero-point
    /// offset — temperatures being the only offset case in the v1
    /// registry).
    pub fn convert(
        &self,
        quantity: Quantity,
        value: f64,
        source_unit: Unit,
    ) -> Result<(f64, Unit), UnitError> {
        let canonical = normalize_for_storage(quantity, value, source_unit)?;
        let target = self.target_unit(quantity);
        // Identity short-circuit: target already canonical → no
        // inverse needed. Saves two `normalize_for_storage` calls
        // and dodges any floating-point round-trip noise on the hot
        // path (canonical-mode responses, by far the more common
        // case for machine consumers).
        if self
            .registry
            .get(quantity)
            .is_some_and(|d| d.canonical == target)
        {
            return Ok((canonical, target));
        }
        let zero = normalize_for_storage(quantity, 0.0, target)?;
        let one = normalize_for_storage(quantity, 1.0, target)?;
        let scale = one - zero;
        Ok(((canonical - zero) / scale, target))
    }
}

/// Tower [`Layer`] wiring the Accept-Units middleware. Construct via
/// [`accept_units_layer`].
#[derive(Clone)]
pub struct AcceptUnitsLayer {
    registry: Arc<dyn UnitRegistry + Send + Sync>,
    prefs: Arc<dyn PrefsResolverFor>,
}

/// Build the Accept-Units layer. `registry` supplies canonical-unit
/// lookup; `prefs` resolves per-request preferences (one
/// round-trip per request — see [`PrefsResolverFor`]).
pub fn accept_units_layer(
    registry: Arc<dyn UnitRegistry + Send + Sync>,
    prefs: Arc<dyn PrefsResolverFor>,
) -> AcceptUnitsLayer {
    AcceptUnitsLayer { registry, prefs }
}

/// Convenience: apply the layer to `router` and return the wrapped
/// router, mirroring [`super::with_request_id`] / [`super::with_latency`].
pub fn with_accept_units(
    router: Router,
    registry: Arc<dyn UnitRegistry + Send + Sync>,
    prefs: Arc<dyn PrefsResolverFor>,
) -> Router {
    router.layer(accept_units_layer(registry, prefs))
}

impl<S> Layer<S> for AcceptUnitsLayer {
    type Service = AcceptUnitsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AcceptUnitsService {
            inner,
            registry: self.registry.clone(),
            prefs: self.prefs.clone(),
        }
    }
}

/// Tower [`Service`] produced by [`AcceptUnitsLayer`].
#[derive(Clone)]
pub struct AcceptUnitsService<S> {
    inner: S,
    registry: Arc<dyn UnitRegistry + Send + Sync>,
    prefs: Arc<dyn PrefsResolverFor>,
}

impl<S> Service<Request> for AcceptUnitsService<S>
where
    S: Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Response, Infallible>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // Standard tower pattern: clone the (poll-ready) inner into
        // the future, swap a fresh clone back into self so the next
        // call gets its own poll_ready cycle.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let registry = self.registry.clone();
        let prefs = self.prefs.clone();

        Box::pin(async move {
            let (mut parts, body) = req.into_parts();
            let mode = parts
                .headers
                .get(&ACCEPT_UNITS_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(UnitsMode::parse)
                .unwrap_or(UnitsMode::Preferred);

            let resolved = match prefs.resolve_for(&parts).await {
                Ok(p) => p,
                Err(resp) => {
                    // Resolver short-circuit — return the resolver's
                    // response, still tagging Vary so a cache won't
                    // promote a 401 across modes.
                    let mut resp = resp;
                    append_vary(&mut resp, "Accept-Units");
                    return Ok(resp);
                }
            };

            let ctx = UnitsCtx::new(mode, Arc::new(resolved), registry);
            parts.extensions.insert(ctx);
            let req = Request::from_parts(parts, body);

            let mut resp = inner.call(req).await?;
            append_vary(&mut resp, "Accept-Units");
            Ok(resp)
        })
    }
}

/// Append `value` to the response `Vary` header without clobbering
/// pre-existing entries. Idempotent if the value is already present.
fn append_vary(resp: &mut Response<Body>, value: &str) {
    let headers = resp.headers_mut();
    if let Some(existing) = headers.get(VARY) {
        if let Ok(existing_str) = existing.to_str() {
            if existing_str
                .split(',')
                .any(|p| p.trim().eq_ignore_ascii_case(value))
            {
                return;
            }
            if let Ok(new) = HeaderValue::from_str(&format!("{existing_str}, {value}")) {
                headers.insert(VARY, new);
                return;
            }
        }
    }
    if let Ok(hv) = HeaderValue::from_str(value) {
        headers.insert(VARY, hv);
    }
}

// ---------------------------------------------------------------------
// Integration tests.
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Extension;
    use axum::routing::get;
    use axum::Json;
    use http::{Request as HttpRequest, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::json;
    use starter_spi::preferences::{
        DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
    };
    use starter_spi::units::StaticRegistry;
    use tower::ServiceExt;

    /// Build a `ResolvedPreferences` whose unit picks differ from the
    /// canonical defaults — the test asserts the middleware actually
    /// honours the prefs (not just falls back to canonical).
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

    /// Stub resolver that hands the same `ResolvedPreferences` to
    /// every request — kept on the trait so the same shape works
    /// once the real `starter-prefs` adapter ships.
    struct StubResolver(ResolvedPreferences);

    #[async_trait]
    impl PrefsResolverFor for StubResolver {
        async fn resolve_for(&self, _parts: &Parts) -> Result<ResolvedPreferences, Response> {
            Ok(self.0.clone())
        }
    }

    /// Test handler that opts into conversion: reads `UnitsCtx`,
    /// converts a temperature originally in Celsius, and emits the
    /// `(value, unit)` pair as JSON.
    async fn handler(Extension(ctx): Extension<UnitsCtx>) -> Json<serde_json::Value> {
        // 100 °C as the source value — happens to be canonical, so
        // the convert path exercises the canonical → target branch.
        let (value, unit) = ctx
            .convert(Quantity::Temperature, 100.0, Unit::Celsius)
            .expect("convert");
        Json(json!({
            "value": value,
            "unit": format!("{unit:?}"),
            "mode": format!("{:?}", ctx.mode()),
        }))
    }

    fn app(resolver: Arc<dyn PrefsResolverFor>) -> Router {
        let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
        Router::new()
            .route("/t", get(handler))
            .layer(accept_units_layer(registry, resolver))
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn missing_header_defaults_to_preferred_and_sets_vary() {
        let resolver = Arc::new(StubResolver(imperial_prefs()));
        let app = app(resolver);
        let resp = app
            .oneshot(HttpRequest::builder().uri("/t").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Vary header present and names Accept-Units.
        let vary = resp.headers().get(VARY).expect("Vary header").to_str().unwrap();
        assert!(
            vary.split(',').any(|p| p.trim().eq_ignore_ascii_case("Accept-Units")),
            "Vary did not list Accept-Units: {vary}"
        );
        let body = body_json(resp).await;
        assert_eq!(body["mode"], "Preferred");
        // Preferred mode + Fahrenheit prefs → 100 °C reported as 212 °F.
        assert_eq!(body["unit"], "Fahrenheit");
        let v = body["value"].as_f64().unwrap();
        assert!((v - 212.0).abs() < 1e-6, "expected 212 °F, got {v}");
    }

    #[tokio::test]
    async fn accept_units_canonical_bypasses_conversion() {
        let resolver = Arc::new(StubResolver(imperial_prefs()));
        let app = app(resolver);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/t")
                    .header(ACCEPT_UNITS_HEADER, "canonical")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["mode"], "Canonical");
        // Canonical mode → temperature in Celsius (registry canonical),
        // value unchanged from the 100 °C input.
        assert_eq!(body["unit"], "Celsius");
        let v = body["value"].as_f64().unwrap();
        assert!((v - 100.0).abs() < 1e-9, "expected 100 °C, got {v}");
    }

    #[tokio::test]
    async fn accept_units_preferred_explicit_matches_default() {
        let resolver = Arc::new(StubResolver(imperial_prefs()));
        let app = app(resolver);
        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/t")
                    .header(ACCEPT_UNITS_HEADER, "preferred")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["mode"], "Preferred");
        assert_eq!(body["unit"], "Fahrenheit");
    }

    #[tokio::test]
    async fn units_ctx_inserted_into_extensions() {
        // A handler that asserts the extension is present is the
        // only observation we have — if the extension were missing
        // axum would 500 on `Extension<UnitsCtx>` extraction.
        // We assert here by serving the test handler successfully.
        let resolver = Arc::new(StubResolver(imperial_prefs()));
        let app = app(resolver);
        let resp = app
            .oneshot(HttpRequest::builder().uri("/t").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn convert_round_trip_is_lossless_for_canonical_source() {
        // Direct unit test for UnitsCtx::convert covering each
        // quantity — protects the affine-inverse math against future
        // edits to normalize_for_storage.
        let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
        let ctx = UnitsCtx::new(
            UnitsMode::Preferred,
            Arc::new(imperial_prefs()),
            registry,
        );
        let cases = [
            (Quantity::Temperature, 0.0, Unit::Celsius, Unit::Fahrenheit, 32.0),
            (Quantity::Temperature, 100.0, Unit::Celsius, Unit::Fahrenheit, 212.0),
            (Quantity::Length, 1.0, Unit::Meter, Unit::Foot, 3.280_839_895),
        ];
        for (q, v, src, expect_unit, expect_val) in cases {
            let (out, unit) = ctx.convert(q, v, src).unwrap();
            assert_eq!(unit, expect_unit, "wrong target unit for {q:?}");
            assert!(
                (out - expect_val).abs() < 1e-6,
                "convert {q:?} {v} {src:?} -> {out} (expected ~{expect_val})"
            );
        }
    }

    #[tokio::test]
    async fn vary_header_appends_when_handler_sets_its_own() {
        // Handler that emits its own Vary header — middleware must
        // append Accept-Units without clobbering.
        async fn h() -> Response {
            let mut r = Response::new(Body::from("ok"));
            r.headers_mut()
                .insert(VARY, HeaderValue::from_static("Accept-Language"));
            r
        }
        let resolver: Arc<dyn PrefsResolverFor> = Arc::new(StubResolver(imperial_prefs()));
        let registry: Arc<dyn UnitRegistry + Send + Sync> = Arc::new(StaticRegistry::new());
        let app = Router::new()
            .route("/h", get(h))
            .layer(accept_units_layer(registry, resolver));
        let resp = app
            .oneshot(HttpRequest::builder().uri("/h").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let vary = resp.headers().get(VARY).unwrap().to_str().unwrap();
        let parts: Vec<_> = vary.split(',').map(|p| p.trim().to_ascii_lowercase()).collect();
        assert!(parts.contains(&"accept-language".to_string()), "vary={vary}");
        assert!(parts.contains(&"accept-units".to_string()), "vary={vary}");
    }
}
