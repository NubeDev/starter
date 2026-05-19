//! Map `starter_spi::Error` to the HTTP wire shape ([`Problem`]) and
//! a status code. This is the single place HTTP semantics meet
//! domain errors — see Rule R3 (transport never contains domain
//! logic; this file is transport, not domain).

mod into_response;
mod status;

pub use into_response::IntoResponse;
pub use status::status_for;
