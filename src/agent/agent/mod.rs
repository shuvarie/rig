//! Configurable agents, builders, hooks, and blocking or streaming run interfaces.
//!
//! [`AgentRunner`] owns execution and lifecycle policies. Tools and context can
//! be static or retrieved per turn; hooks can inject context through [`RequestPatch`].
//!
//! ```no_run
//! use rig::agent::prelude::*;
//! use rig::{Model, providers::openai::{self, OpenAI}};
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let model = OpenAI::from_env()?.completion(openai::GPT_5_2);
//! let agent = AgentBuilder::new(model).preamble("Be concise.").build();
//! let response = agent.prompt("Explain ownership.").await?;
//! # Ok(())
//! # }
//! ```
/// The generation-option shortcuts [`AgentBuilder`] and [`AgentRunner`]
/// share: each sets one field of `$whose` generation options (`$whose` is
/// "the agent's" or "this run's") and keeps the others.
macro_rules! generation_option_setters {
    ($whose:literal) => {
        generation_option_setters! { $whose;
            reasoning(reasoning: impl Into<crate::completion::Reasoning>) "the reasoning level or budget";
            cache(cache: crate::completion::CacheRetention) "the cache retention";
            service_tier(tier: crate::completion::ServiceTier) "the service tier";
            verbosity(verbosity: crate::completion::Verbosity) "the answer verbosity";
            parallel_tool_calls(parallel: bool) "whether the model may call several tools in one turn";
            top_p(top_p: f64) "the nucleus sampling probability mass";
            seed(seed: u64) "the sampling seed";
            ///
            /// Each call is checked by the model it goes to, when that model
            /// encodes it, against the catalog spec the model was connected
            /// with: a refused option fails the run under `Error` with nothing
            /// sent, and is dropped with a warning under `Ignore`. A model a run
            /// switches to checks its own calls the same way. To see every
            /// refusal before a run starts, call
            /// [`DynModel::check`](rig::DynModel::check) on the model.
            on_unsupported(policy: crate::completion::OnUnsupported) "what happens to an option the wire or model cannot honour";
        }
    };
    ($whose:literal; $($(#[$extra:meta])* $name:ident($arg:ident: $ty:ty) $what:literal;)*) => {$(
        #[doc = concat!("Set ", $what, " in ", $whose, " generation options, as [`GenerationOptions::",
            stringify!($name), "`](crate::completion::GenerationOptions::", stringify!($name),
            ") does, keeping its other fields. See [`Self::options`] for the order of calls.")]
        $(#[$extra])*
        pub fn $name(mut self, $arg: $ty) -> Self {
            let options = std::mem::take(&mut self.config.spec_mut().options);
            self.config.spec_mut().options = options.$name($arg);
            self
        }
    )*};
}

mod builder;
mod completion;
mod engine;
pub mod hook;
mod memory;
pub mod runner;
mod streaming;
mod telemetry;
mod tool;
mod typed;

/// Fallback display name used in telemetry spans and logs when an agent has no
/// configured name.
pub(crate) const UNKNOWN_AGENT_NAME: &str = "Unnamed Agent";

pub use crate::agent::handlers::ModelHandle;
pub use crate::agent::run::response::{CompletionCall, PromptResponse};
pub use crate::agent::run::spec::RunSpec;
pub use crate::agent::run::{
    AgentRun, AgentRunStep, ModelTurn, ModelTurnOutcome, OutputMode, PendingToolCall, TurnTools,
};
pub use crate::completion::ModelRef;
/// Provider-neutral identity shared by core responses, stream terminals,
/// completion calls, and hook events.
pub use crate::completion::ResponseIdentity;
pub use crate::message::Text;
pub use builder::AgentBuilder;
pub use completion::Agent;
pub(crate) use completion::register_generated;
pub use hook::{
    AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, HookStack,
    InvalidToolCallAction, InvalidToolCallContext, InvalidToolCallReason, ModelSelection,
    ModelSelectionAction, ModelTurnAction, ModelTurnFinished, ObservationAction, ReasoningDelta,
    RequestPatch, RetryRequest, RunId, RunSettled, RunStart, RunStartAction, Scratchpad,
    SettledOutcome, StepEventKind, TextDelta, ToolCallDelta,
};
pub use hook::{DispatchAction, DispatchEvent, OutcomeAction, OutcomeEvent};
pub use runner::AgentRunner;
pub use streaming::{
    MultiTurnStreamItem, RUN_EVENTS_CAPACITY, RunEvents, StreamToStdoutError, StreamingResult,
    stream_to_stdout,
};
pub use typed::{TypedOutput, TypedPromptResponse, TypedRun};
