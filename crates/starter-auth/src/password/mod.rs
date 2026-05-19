//! argon2id hashing. One file per direction (hash, verify).

mod hash;
mod verify;

pub use hash::hash;
pub use verify::verify;
