//! What the sign-in use cases take and give back. Inputs that change something are named after
//! what they do; inputs that only read end in `Query` (ARCH-EVENTS).

mod answers;
mod commands;
mod queries;

pub use answers::*;
pub use commands::*;
pub use queries::*;
