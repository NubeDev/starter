//! [`SduiState`] — the axum state the three SDUI routes share.
//!
//! Built via [`SduiStateBuilder`]; the consumer wires four pieces:
//!
//! - a [`PageProvider`] for `/resolve`,
//! - an [`EntityGraph`] for the binding engine,
//! - a [`HandlerRegistry`] for `/action`,
//! - a [`QueryEngine`] for `/table`.
//!
//! The state is cheap-to-clone (`Arc` everywhere) so axum's
//! per-request `State<SduiState>` extraction stays zero-copy.

use std::sync::Arc;

use starter_ui_bindings::EntityGraph;

use crate::handler::HandlerRegistry;
use crate::page::PageProvider;
use crate::query::QueryEngine;

/// Shared state for the SDUI routes. Cloning is cheap — every
/// inner is wrapped in an `Arc`.
#[derive(Clone)]
pub struct SduiState {
    pub(crate) pages: Arc<dyn PageProvider>,
    pub(crate) graph: Arc<dyn EntityGraph + Send + Sync>,
    pub(crate) handlers: Arc<HandlerRegistry>,
    pub(crate) query: Arc<dyn QueryEngine>,
}

impl SduiState {
    /// Start a new builder. Every field is required; `build()`
    /// fails with a descriptive error if anything is missing.
    pub fn builder() -> SduiStateBuilder {
        SduiStateBuilder::default()
    }

    /// Borrow the registered [`HandlerRegistry`] — useful for
    /// tests that want to assert a handler is present.
    pub fn handlers(&self) -> &HandlerRegistry {
        &self.handlers
    }
}

impl std::fmt::Debug for SduiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SduiState")
            .field("handlers", &self.handlers)
            .finish()
    }
}

/// Builder for [`SduiState`].
#[derive(Default)]
pub struct SduiStateBuilder {
    pages: Option<Arc<dyn PageProvider>>,
    graph: Option<Arc<dyn EntityGraph + Send + Sync>>,
    handlers: Option<Arc<HandlerRegistry>>,
    query: Option<Arc<dyn QueryEngine>>,
}

impl SduiStateBuilder {
    /// Wire the [`PageProvider`] used by `/resolve`.
    pub fn with_pages<P: PageProvider>(mut self, pages: P) -> Self {
        self.pages = Some(Arc::new(pages));
        self
    }

    /// Wire the [`EntityGraph`] the binding engine walks.
    pub fn with_entity_graph<G>(mut self, graph: G) -> Self
    where
        G: EntityGraph + Send + Sync + 'static,
    {
        self.graph = Some(Arc::new(graph));
        self
    }

    /// Wire the [`HandlerRegistry`] for `/action`.
    pub fn with_handler_registry(mut self, registry: HandlerRegistry) -> Self {
        self.handlers = Some(Arc::new(registry));
        self
    }

    /// Wire the [`QueryEngine`] for `/table`.
    pub fn with_query_engine<Q: QueryEngine>(mut self, engine: Q) -> Self {
        self.query = Some(Arc::new(engine));
        self
    }

    /// Finalise. Returns a descriptive `&'static str` error when a
    /// required piece was not wired.
    pub fn build(self) -> Result<SduiState, &'static str> {
        Ok(SduiState {
            pages: self.pages.ok_or("SduiState requires a PageProvider")?,
            graph: self.graph.ok_or("SduiState requires an EntityGraph")?,
            handlers: self
                .handlers
                .ok_or("SduiState requires a HandlerRegistry")?,
            query: self.query.ok_or("SduiState requires a QueryEngine")?,
        })
    }
}
