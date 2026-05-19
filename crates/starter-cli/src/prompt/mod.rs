//! Interactive prompts. Each file is one prompt type.

mod confirm;
mod password;

pub use confirm::confirm;
pub use password::password;
