//! Common imports for Rig's classic runtime.
//!
//! ```
//! use crate::agent::prelude::*;
//! use crate::operation::Completion;
//!
//! fn assistant(model: impl Into<DynModel<Completion>>) -> Agent {
//!     AgentBuilder::new(model).build()
//! }
//! ```

pub use crate::agent::agent::{
    Agent, AgentBuilder, AgentHook, HookContext, ModelHandle, ModelRef, ModelSelection,
    ModelSelectionAction, MultiTurnStreamItem, RunEvents, StreamingResult,
};
pub use crate::agent::completion::{Message, PromptError, StructuredOutputError};
pub use crate::agent::tool::{Tool, ToolSet};
pub use crate::driver::{DynModel, Model};
pub use crate::error::ProviderError;

pub use crate::Embed;
pub use crate::embeddings::EmbeddingsBuilder;
pub use crate::vector_store::VectorStoreIndex;
pub use crate::vector_store::in_memory_store::InMemoryVectorStore;
pub use crate::vector_store::request::VectorSearchRequest;
