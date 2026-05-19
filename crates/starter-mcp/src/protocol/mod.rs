//! JSON-RPC 2.0 envelopes for MCP. Request, response, and error
//! types — each in its own file because they evolve independently
//! as the MCP spec changes.

mod error;
mod request;
mod response;

pub use error::RpcError;
pub use request::Request;
pub use response::Response;
