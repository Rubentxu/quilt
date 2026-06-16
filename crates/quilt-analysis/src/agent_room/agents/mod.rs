//! Built-in agent executors. V1 ships ONE: `decay-annotator`.
//!
//! Adding a new type is a matter of:
//! 1. Implementing `AgentExecutor` for a new struct here.
//! 2. Registering it in `AgentRegistry::with_defaults()`.

pub mod decay_annotator;
