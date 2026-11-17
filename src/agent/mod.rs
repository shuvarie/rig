//! The agent runtime: builders, serializable run state, hooks, tools, and
//! typed extraction.
//!
//! Portable contracts live at the crate root; this module holds what runs over
//! them. Native and browser WASM targets are supported; WASI is not. MCP tool
//! sources are provided by the native-only `providers::rmcp`.
//!
//! ```
//! use crate::agent::agent::AgentBuilder;
//! use crate::test_utils::MockCompletionModel;
//!
//! let agent = AgentBuilder::new(MockCompletionModel::default()).build();
//! # let _ = agent;
//! ```

#[doc(hidden)]
pub use crate as __core;

// The runtime's own module, named as it was when this was a crate of its own.
#[allow(clippy::module_inception)]
pub mod agent;
pub mod completion;
pub mod extractor;
pub mod handlers;
/// Ready-made integrations: the CLI chatbot.
pub mod integrations;

/// Lock `mutex`, taking the guard even if a panicking holder poisoned it.
pub(crate) fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
pub mod prelude;
pub mod run;
pub mod streaming;
#[cfg(any(test, feature = "test-utils"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-utils")))]
pub mod test_utils;
pub mod tool;

/// Everything the runtime's inner module exports, so `crate::agent::Agent`
/// and `crate::agent::HookStack` resolve as they did when this was a crate
/// root.
pub use self::agent::*;

pub use agent::TypedPromptResponse;
pub use agent::{
    Agent, AgentBuilder, AgentHook, AgentRun, AgentRunner, HookContext, ModelHandle, ModelRef,
    ModelSelection, ModelSelectionAction,
};

// Compile-time thread-safety contract: the agent surface must be safe to hold
// in shared host state (worker pools, ECS resources) on native targets.
#[cfg(not(target_family = "wasm"))]
const _: fn() = || {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<Agent>();
    assert_send_sync_static::<AgentRunner>();
    assert_send_sync_static::<ModelHandle>();
    assert_send_sync_static::<agent::MultiTurnStreamItem>();
    assert_send_sync_static::<agent::RunEvents>();
    assert_send_sync_static::<agent::PromptResponse>();
    assert_send_sync_static::<tool::ToolSet>();
    assert_send_sync_static::<tool::ToolCatalog>();
};
