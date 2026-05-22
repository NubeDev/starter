//! Named action handlers and the [`HandlerRegistry`].
//!
//! Per **R5** every interaction round-trips through
//! `POST /api/v1/ui/action`. The body's `handler` field names a
//! function registered here; the response is the discriminated
//! [`starter_ui_ir::ActionResponse`] union.
//!
//! Handlers are async closures the host registers at startup:
//!
//! ```
//! use starter_sdui_routes::{HandlerRegistry, HandlerContext};
//! use starter_ui_ir::{ActionResponse, ToastIntent};
//!
//! let registry = HandlerRegistry::new()
//!     .with("device.restart", |_ctx: HandlerContext| async move {
//!         Ok(ActionResponse::Toast {
//!             intent: ToastIntent::Ok,
//!             message: "queued".into(),
//!         })
//!     });
//! ```
//!
//! Auth / RBAC runs **inside** the handler against
//! [`HandlerContext::principal`] — the routes crate does not
//! authorise on the handler's behalf. Per R7's threat model: the
//! capability filter is a *vocabulary* check, not an authorisation
//! check; auth lives here.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use starter_ui_ir::{ActionContext, ActionResponse};

use crate::error::SduiError;

/// Pinned, send-able future the registry stores per handler.
pub type ActionFuture =
    Pin<Box<dyn Future<Output = Result<ActionResponse, SduiError>> + Send + 'static>>;

/// Type-erased handler function.
pub type ActionFn = Arc<dyn Fn(HandlerContext) -> ActionFuture + Send + Sync + 'static>;

/// Principal the action is being dispatched as. Opaque to the
/// routes crate — the host's auth layer constructs this from
/// whatever bearer / cookie / session token the request carried.
#[derive(Debug, Clone, Default)]
pub struct Principal {
    /// Stable subject identifier (user id, service-account name).
    pub subject: String,
    /// Free-form claims the handler can branch on (`{ "role":
    /// "admin", "org": "acme" }` etc).
    pub claims: serde_json::Map<String, JsonValue>,
}

impl Principal {
    /// Empty principal — useful for tests that don't go through
    /// the host's auth layer. Production code constructs a real
    /// principal from the request.
    pub fn anonymous() -> Self {
        Self::default()
    }
}

/// What a handler sees on dispatch.
pub struct HandlerContext {
    /// The principal the request was authenticated as.
    pub principal: Principal,
    /// The handler name that was dispatched — handy for shared
    /// handler closures that branch on `ctx.name`.
    pub name: String,
    /// Handler-specific arguments — opaque JSON forwarded from the
    /// request body.
    pub args: JsonValue,
    /// Page-level [`ActionContext`] (target component / nav stack /
    /// page state / auth subject).
    pub context: ActionContext,
}

/// `404`-style handle that a `dispatch` can return when no handler
/// owns the requested name. The route maps this to
/// [`SduiError::HandlerNotFound`].
#[derive(Debug, Clone)]
pub struct HandlerNotFound {
    /// Handler name that was missing.
    pub handler: String,
}

/// Registry of named action handlers. Inserts are by name; the
/// last `with` call for a given name wins (registries are normally
/// built once at startup, so collisions are accidental).
#[derive(Default, Clone)]
pub struct HandlerRegistry {
    by_name: HashMap<String, ActionFn>,
}

impl HandlerRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a handler. The closure receives [`HandlerContext`]
    /// and returns a future yielding the action response.
    pub fn with<F, Fut>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: Fn(HandlerContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ActionResponse, SduiError>> + Send + 'static,
    {
        let f: ActionFn = Arc::new(move |ctx| Box::pin(f(ctx)));
        self.by_name.insert(name.into(), f);
        self
    }

    /// Insert a pre-boxed handler — useful for dynamic registration
    /// from extensions.
    pub fn insert(&mut self, name: impl Into<String>, handler: ActionFn) {
        self.by_name.insert(name.into(), handler);
    }

    /// `true` when the named handler is registered.
    pub fn has(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// Number of registered handlers.
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// `true` when no handlers are registered.
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Dispatch by name. Returns [`HandlerNotFound`] when nothing
    /// is registered; the route maps that to a structured 404.
    pub async fn dispatch(
        &self,
        ctx: HandlerContext,
    ) -> Result<Result<ActionResponse, SduiError>, HandlerNotFound> {
        let f = match self.by_name.get(&ctx.name) {
            Some(f) => f.clone(),
            None => {
                return Err(HandlerNotFound { handler: ctx.name });
            }
        };

        // Enforce the per-handler timeout from R8. The handler
        // future runs under `tokio::time::timeout`; on expiry the
        // route surfaces a `diagnostics` error tagged
        // `handler_timeout`.
        let fut = f(ctx);
        let result = tokio::time::timeout(crate::limits::MAX_HANDLER_TIMEOUT, fut).await;
        match result {
            Ok(res) => Ok(res),
            Err(_) => Ok(Err(SduiError::PayloadTooLarge {
                what: crate::error::WhatTag::HandlerTimeout,
                detail: format!("handler exceeded {:?}", crate::limits::MAX_HANDLER_TIMEOUT,),
            })),
        }
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("names", &self.by_name.keys().collect::<Vec<_>>())
            .finish()
    }
}
