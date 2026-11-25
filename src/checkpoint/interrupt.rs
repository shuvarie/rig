use std::sync::Arc;

use crate::agent::run::{AgentRun, PendingToolCall};
use crate::completion::Message;

use super::{Checkpoint, PartialReply};

/// The one request that continues an interrupted run.
///
/// [`ResumePlan::of`] derives it from the checkpoint alone: what the run was
/// waiting on when it stopped decides which of the three it is.
#[derive(Debug, Clone)]
pub enum ResumePlan {
    /// The reply had started arriving and its turn was never accepted.
    /// Continuing means sending the same call with the partial reply attached
    /// as a prefill.
    Continue {
        /// What had arrived.
        partial: PartialReply,
    },
    /// The run committed an assistant turn whose calls it was still running.
    /// Continuing means answering these calls; every other call is already
    /// answered and must not run again.
    Answer {
        /// The unanswered calls, in call order.
        calls: Vec<PendingToolCall>,
    },
    /// Nothing of the interrupted attempt survived. Continuing means sending
    /// this prompt again.
    Replay {
        /// The prompt to send.
        prompt: Message,
    },
}

impl ResumePlan {
    /// The plan for `run`.
    pub fn of(run: &InterruptedRun) -> Self {
        Self::derive(run.checkpoint.run(), run.checkpoint.partial())
    }

    fn derive(run: &AgentRun, partial: Option<&PartialReply>) -> Self {
        // A committed tool batch comes first: the run is past the model turn,
        // so answering what is open is the only way forward, and the partial
        // reply has already been committed as that turn.
        let calls = run.pending_tool_calls();
        if !calls.is_empty() {
            return Self::Answer { calls };
        }
        if let Some(partial) = partial.filter(|partial| !partial.is_empty()) {
            return Self::Continue {
                partial: partial.clone(),
            };
        }
        // A run with no committed prompt at all cannot happen: the prompt is
        // committed before the first step. Falling back to an empty prompt
        // keeps the plan total without inventing a message.
        Self::Replay {
            prompt: run
                .next_prompt()
                .cloned()
                .map(Arc::unwrap_or_clone)
                .unwrap_or_else(|| Message::user("")),
        }
    }
}

/// A run read back from a [`Checkpoint`]: what it was, and how to continue it.
///
/// The history is the run's own, repaired as
/// [`AgentRun::canonical_history`] repairs it, so every call the run left open
/// is answered or closed before a provider ever sees it again.
#[derive(Debug, Clone)]
pub struct InterruptedRun {
    checkpoint: Checkpoint,
    history: Vec<Message>,
    plan: ResumePlan,
}

impl InterruptedRun {
    /// Read `checkpoint` back.
    pub fn of(checkpoint: Checkpoint) -> Self {
        let history = checkpoint.run().canonical_history();
        let plan = ResumePlan::derive(checkpoint.run(), checkpoint.partial());
        Self {
            checkpoint,
            history,
            plan,
        }
    }

    /// The checkpoint this was read from.
    pub fn checkpoint(&self) -> &Checkpoint {
        &self.checkpoint
    }

    /// Why the run stopped.
    pub fn reason(&self) -> &str {
        self.checkpoint.reason()
    }

    /// The history at the interruption, every open call closed.
    pub fn history(&self) -> &[Message] {
        &self.history
    }

    /// The request that continues the run.
    pub fn plan(&self) -> &ResumePlan {
        &self.plan
    }

    /// The checkpoint, the repaired history and the plan.
    pub fn into_parts(self) -> (Checkpoint, Vec<Message>, ResumePlan) {
        (self.checkpoint, self.history, self.plan)
    }

    /// The checkpoint back, to resume the run.
    pub fn into_checkpoint(self) -> Checkpoint {
        self.checkpoint
    }

    /// This interruption as the error a driver returns when a run stops early.
    pub fn into_error(self) -> crate::agent::run::response::PromptError {
        let reason = self.checkpoint.reason().to_string();
        crate::agent::run::response::PromptError::Interrupted {
            checkpoint: Box::new(self.checkpoint),
            history: self.history,
            reason,
        }
    }
}
