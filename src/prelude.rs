//! Common model-construction, completion, embedding, tool, agent and
//! vector-store imports.
//!
//! ```
//! use rig::prelude::*;
//!
//! let message = Message::user("Hello");
//! ```

pub use crate::agent::agent::{
    Agent, AgentBuilder, AgentHook, HookContext, ModelHandle, ModelRef, ModelSelection,
    ModelSelectionAction, MultiTurnStreamItem, RunEvents, StreamingResult,
};
pub use crate::agent::completion::{PromptError, StructuredOutputError};
pub use crate::agent::tool::ToolSet;
pub use crate::completion::Message;
pub use crate::driver::{DynModel, Model};
pub use crate::error::ProviderError;

pub use crate::Embed;
pub use crate::embeddings::EmbeddingsBuilder;

pub use crate::tool::{Tool, ToolContext};

pub use crate::vector_store::VectorStoreIndex;
pub use crate::vector_store::in_memory_store::InMemoryVectorStore;
pub use crate::vector_store::request::VectorSearchRequest;
