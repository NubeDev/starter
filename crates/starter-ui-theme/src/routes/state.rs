//! Shared state for the `/api/v1/ui/theme` handlers.

use std::sync::Arc;

use starter_spi::ui::theme::ThemeStore;

/// Handler context. Cheap to clone — holds an `Arc<dyn ThemeStore>`.
#[derive(Clone)]
pub struct ThemeState {
    /// Backing store implementation.
    pub store: Arc<dyn ThemeStore>,
}

impl ThemeState {
    /// Build the state from a store.
    pub fn new(store: Arc<dyn ThemeStore>) -> Self {
        Self { store }
    }
}
