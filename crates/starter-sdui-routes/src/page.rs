//! [`PageProvider`] — how the routes crate looks pages up.
//!
//! `POST /api/v1/ui/resolve` carries a `page_ref` that the host
//! translates into a [`starter_ui_ir::ComponentTree`]. Different
//! hosts back this in different ways — a SQL store, a Rubix-style
//! node graph, an in-memory fixture for tests — so the trait is
//! intentionally minimal: one async lookup.
//!
//! The trait is **async** because production hosts touch databases,
//! but synchronous implementations (the in-memory fixtures used in
//! tests and examples) can return an immediately-ready future via
//! `async { ... }`.

use async_trait::async_trait;
use starter_ui_ir::ComponentTree;

/// Opaque reference to a page known to the host. The wire shape is
/// a string — the host decides whether that string is a UUID, a
/// path, a slug, or a typed kind name. The routes crate never
/// interprets it.
pub type PageRef = String;

/// Resolves a [`PageRef`] to a [`ComponentTree`].
#[async_trait]
pub trait PageProvider: Send + Sync + 'static {
    /// Look up the page identified by `page_ref`. Return `None`
    /// when the page is unknown — the route surfaces that as a
    /// `404` with a `diagnostics`-shaped body.
    async fn lookup_page(&self, page_ref: &str) -> Option<ComponentTree>;
}

/// Convenience in-memory provider for examples and tests.
#[derive(Debug, Default, Clone)]
pub struct InMemoryPageProvider {
    pages: std::collections::HashMap<String, ComponentTree>,
}

impl InMemoryPageProvider {
    /// Empty provider.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one page.
    pub fn with(mut self, page_ref: impl Into<String>, tree: ComponentTree) -> Self {
        self.pages.insert(page_ref.into(), tree);
        self
    }
}

#[async_trait]
impl PageProvider for InMemoryPageProvider {
    async fn lookup_page(&self, page_ref: &str) -> Option<ComponentTree> {
        self.pages.get(page_ref).cloned()
    }
}
