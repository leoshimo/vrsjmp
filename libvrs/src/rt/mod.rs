mod binding;
mod error;
mod kernel;
pub mod program;
mod registry;
mod runtime;

mod mailbox;
mod proc;
mod proc_io;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use proc::{ProcessExit, ProcessHandle, ProcessId, ProcessResult};
pub use runtime::Runtime;
