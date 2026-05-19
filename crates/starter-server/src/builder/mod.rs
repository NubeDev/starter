//! `ServerBuilder` and `bind` are two responsibilities; one file each.

mod bind;
mod server_builder;

pub use bind::bind;
pub use server_builder::ServerBuilder;
