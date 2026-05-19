//! `Pool` wrapper. Connect and ref-counted clone live in separate
//! files so an AI editing connect logic doesn't load the (much
//! smaller) clone semantics file.

mod connect;
mod wrapper;

pub use connect::connect;
pub use wrapper::Pool;
