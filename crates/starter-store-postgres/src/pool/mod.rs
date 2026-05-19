//! `Pool` wrapper. Public surface mirrors the sqlite crate's.

mod connect;
mod wrapper;

pub use connect::connect;
pub use wrapper::Pool;
