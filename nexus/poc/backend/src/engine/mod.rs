//! Embeds the ArkFlow engine: registration, component catalogs, and run paths.

mod collector;
mod register;
mod run;
mod sql_query;
mod validate;

pub mod catalog;

pub use register::register_all;
pub use run::run_config;
pub use sql_query::query as sql_query;
pub use validate::validate;
