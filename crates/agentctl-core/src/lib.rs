//! `agentctl-core` — the application/use-case layer. `App` implements every
//! current CLI command (list, install, uninstall, agent event dispatch,
//! clear, info) purely against trait objects (`AgentRepository`, `Harness`).
//! No `clap` dependency, no concrete infrastructure dependency.

mod app;
mod event;
mod list;

pub use app::App;
pub use event::EventKind;
pub use list::{DEFAULT_COLUMNS, DEFAULT_COLUMNS_STR, Field};
