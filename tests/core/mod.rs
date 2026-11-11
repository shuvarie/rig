//! The root package's own tests: contract suites and guards that scan the
//! source tree, which need the repository root. Provider behaviour lives in
//! `tests/providers`; anything needing crate-private types stays a unit test
//! in its crate.
//!
//! Run them with `cargo nextest run -p rig --test integration -E
//! 'test(/^core::/)'`, and one with `cargo test -p rig --test integration
//! core::prompt_response_messages::standard_prompt_returns_string`.

mod agent_run_stepper;
mod catalog_constants;
mod dependency_graph;
#[cfg(feature = "derive")]
mod embed_macro;
mod history_conformance;
mod history_conformance_registry;
mod loaders;
mod prompt_response_messages;
mod provider_keys;
mod reasoning_stream_stats;
#[cfg(feature = "derive")]
mod rig_tool_facade;
mod streaming_conformance;
mod streaming_conformance_registry;
mod streaming_conformance_suites;
mod user_agent;
