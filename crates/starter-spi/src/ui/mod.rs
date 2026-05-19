//! Org-level UI surface contracts.
//!
//! Wire types and traits that any starter consumer rendering a
//! starter-built admin surface needs to speak. Unlike
//! [`crate::preferences`] (per-user choices), this module describes
//! the **org-shared** appearance the admin editor configures once.
//!
//! Current contents:
//!
//! - [`theme`] — theme tokens, shell config, and the [`theme::ThemeStore`]
//!   persistence trait.

pub mod theme;
