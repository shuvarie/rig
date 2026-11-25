#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        clippy::unreachable
    )
)]
//! Rig: provider-agnostic model, message, conversation, tool, and vector-store
//! contracts; the agent runtime over them; and the bundled providers.
//!
//! A provider configuration builds an endpoint [`wire`], and a [`Model`] binds
//! a wire to a [`driver::Transport`]. An [`AgentBuilder`] runs a model against
//! tools, hooks and a serializable [`run::AgentRun`]. Every integration a host
//! could own is a trait: transports ([`http_client::HttpClientExt`],
//! [`ws_client::WebSocketClientExt`]), vector stores
//! ([`vector_store::VectorStoreIndex`]), and conversation stores
//! ([`memory::Store`]).
//!
//! ```no_run
//! use rig::prelude::*;
//! use rig::providers::openai::{self, OpenAI};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let agent = AgentBuilder::new(OpenAI::from_env()?.completion(openai::GPT_4O)).build();
//! println!("{}", agent.prompt("Who are you?").await?.output());
//! # Ok(())
//! # }
//! ```
//!
//! The crate does not force serde_json's `preserve_order` or `float_roundtrip`.
//! Without them, schema properties and JSON text in prompts, embeddings and
//! tool results use sorted object keys, and parsed floats can differ in their
//! low bits. Restore the previous configuration in your application's
//! dependencies with
//! `serde_json = { version = "1", features = ["preserve_order", "float_roundtrip"] }`.

extern crate self as rig;

#[cfg(feature = "audio")]
#[cfg_attr(docsrs, doc(cfg(feature = "audio")))]
pub mod audio_generation;
pub mod catalog;
pub mod checkpoint;
pub mod client;
pub mod completion;
pub mod context;
pub mod conversation;
pub mod driver;
pub mod effect;
pub mod embeddings;
pub mod error;
pub mod http_client;
pub mod id;
#[cfg(feature = "image")]
#[cfg_attr(docsrs, doc(cfg(feature = "image")))]
pub mod image_generation;
/// Internal JSON helpers shared with sibling modules (e.g. `agent`).
#[doc(hidden)]
pub mod json_utils;
pub mod loaders;
pub mod markers;
pub mod model;
pub mod observe;
pub mod operation;
pub mod prelude;
pub(crate) mod provider_response;
pub mod providers;
pub mod redact;
pub mod rerank;
#[cfg(all(feature = "reqwest", not(target_family = "wasm")))]
#[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
pub mod runtime;
pub mod serve;

pub mod streaming;
pub mod structured_output;
#[cfg(any(test, feature = "test-utils"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-utils")))]
pub mod test_utils;
pub mod tool;
pub mod transcription;
pub mod vector_store;
pub mod wasm_compat;
pub mod wire;
pub mod ws_client;

/// The agent runtime: builders, hooks, tools, and the serializable run state.
pub mod agent;
/// Conversations over time: windows, compaction, and saving and resuming.
pub mod memory;

pub use agent::{Agent, AgentBuilder, AgentRunner, TypedPromptResponse};
pub use completion::message;
pub use driver::{DynModel, Model};
pub use embeddings::Embed;
pub use error::{ErrorKind, ErrorReport, ProviderError};
pub use provider_response::ProviderResponseError;
// `schemars`, `serde`, and `serde_json` are re-exported so downstream crates
// can resolve them through Rig instead of requiring a direct dependency.
pub use schemars;
pub use serde;
pub use serde_json;

/// The sans-IO run layer: `AgentRun` and its step, spec, patch and response
/// types, the streamed-turn assembler, and the loop-side transcript helpers.
pub mod run {
    pub use crate::agent::run::*;
}

/// Typed extraction.
pub mod extractor {
    pub use crate::agent::extractor::*;
}

/// The handler table and its typed views.
pub mod handlers {
    pub use crate::agent::handlers::*;
}

/// Ready-made integrations: the CLI chatbot.
pub mod integrations {
    pub use crate::agent::integrations::*;
}

pub mod telemetry;

/// Milliseconds since the Unix epoch at `at`, also on the web; 0 before it.
pub fn millis(at: web_time::SystemTime) -> u64 {
    at.duration_since(web_time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// Milliseconds since the Unix epoch, also on the web.
pub fn now_ms() -> u64 {
    millis(web_time::SystemTime::now())
}

// Native runtime values must retain their thread-safety bounds.
#[cfg(not(target_family = "wasm"))]
const _: fn() = || {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    fn assert_send_static<T: Send + 'static>() {}
    assert_send_sync_static::<tool::ManagedToolToken>();
    assert_send_sync_static::<streaming::StreamEvent>();
    // The serializable identity a typed view is resolved from.
    assert_send_sync_static::<completion::ModelRef>();
    assert_send_sync_static::<tool::DynamicTool>();
    // One erased transport, shared by every provider a host binds.
    assert_send_sync_static::<http_client::DynHttpClient>();
    // A live stream is owned by one poller: `Send` so it can move to a worker,
    // not `Sync`.
    assert_send_static::<streaming::CompletionStream>();
    assert_send_sync_static::<Agent>();
    assert_send_sync_static::<AgentRunner>();
    assert_send_sync_static::<tool::server::ToolServerHandle>();
    assert_send_sync_static::<tool::ToolSet>();
    assert_send_sync_static::<tool::ToolCatalog>();
};
