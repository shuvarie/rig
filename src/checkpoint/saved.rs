use serde::{Deserialize, Serialize};

use crate::completion::message::AssistantContent;

use super::{CheckpointError, InterruptedRun};

/// The [`Checkpoint`] envelope format this crate writes and reads.
pub const CHECKPOINT_FORMAT: u32 = 1;

/// A run saved where it stopped.
///
/// The envelope is versioned like [`AgentRun`](crate::agent::AgentRun)'s own:
/// [`CHECKPOINT_FORMAT`] is written on every serialization and checked on every
/// deserialization, and an unknown key is refused, so a checkpoint written by
/// another rig is refused by name rather than loaded with defaults.
///
/// The run carries its usage, its completion calls and the tool calls it is
/// waiting on, so a checkpoint holds the usage anchor and the in-flight calls
/// with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    #[serde(deserialize_with = "checkpoint_format")]
    format: u32,
    run: crate::agent::AgentRun,
    reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    partial: Option<PartialReply>,
}

impl Checkpoint {
    /// A checkpoint of `run`, which stopped because of `reason`.
    pub fn of(run: crate::agent::AgentRun, reason: impl Into<String>) -> Self {
        Self {
            format: CHECKPOINT_FORMAT,
            run,
            reason: reason.into(),
            partial: None,
        }
    }

    /// The same checkpoint, with the reply that had arrived when it stopped.
    #[must_use = "the setting applies to the returned value"]
    pub fn with_partial(mut self, partial: PartialReply) -> Self {
        self.partial = Some(partial);
        self
    }

    /// The run as it stopped.
    pub fn run(&self) -> &crate::agent::AgentRun {
        &self.run
    }

    /// The run as it stopped.
    pub fn into_run(self) -> crate::agent::AgentRun {
        self.run
    }

    /// Why it stopped.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The reply that had arrived when it stopped, when it stopped mid-reply.
    pub fn partial(&self) -> Option<&PartialReply> {
        self.partial.as_ref()
    }

    /// This checkpoint as an interruption: the repaired history and the request
    /// that continues it.
    pub fn interrupted(&self) -> InterruptedRun {
        InterruptedRun::of(self.clone())
    }

    /// The checkpoint as JSON, envelope format included.
    ///
    /// # Errors
    ///
    /// Fails only if the run does not serialize, which a value built by this
    /// crate always does.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// A checkpoint from JSON written by [`Self::to_json`].
    ///
    /// # Errors
    ///
    /// Fails on malformed JSON, on an envelope written by another rig, and on
    /// an unknown key.
    pub fn from_json(text: &str) -> Result<Self, CheckpointError> {
        Ok(serde_json::from_str(text)?)
    }
}

/// What of a reply had arrived when a run stopped mid-turn.
///
/// The content is whatever the provider had streamed: text so far, reasoning so
/// far, and any tool call already complete. A run cut before its turn was
/// accepted never committed this to history, so it is the one part of an
/// interruption a checkpoint must carry beside the run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialReply {
    /// The parts that arrived, in the order they started.
    pub content: Vec<AssistantContent>,
}

impl PartialReply {
    /// The partial reply made of `content`.
    pub fn new(content: Vec<AssistantContent>) -> Self {
        Self { content }
    }

    /// Whether nothing had arrived.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// The reply as one assistant message, or `None` when nothing arrived.
    pub fn message(&self) -> Option<crate::completion::Message> {
        (!self.is_empty()).then(|| {
            crate::completion::Message::Assistant(
                crate::completion::message::AssistantMessage::new(self.content.clone()),
            )
        })
    }
}

/// Deserialize the envelope's `format`, refusing any other than
/// [`CHECKPOINT_FORMAT`] so a checkpoint saved by another rig is never loaded
/// with defaults filled in.
fn checkpoint_format<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let format = u32::deserialize(deserializer)?;
    if format == CHECKPOINT_FORMAT {
        Ok(format)
    } else {
        Err(serde::de::Error::custom(format!(
            "this checkpoint was made by another version of rig ({format}, not {CHECKPOINT_FORMAT})"
        )))
    }
}
