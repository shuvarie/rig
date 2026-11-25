//! A run saved where it stopped, and the request that continues it.
//!
//! [`Checkpoint`] is a serializable point in a run: the [`AgentRun`] state
//! machine, why it stopped, and the part of the reply that had arrived when it
//! did. [`InterruptedRun`] is that checkpoint read back, with its history
//! repaired so no call is left open and a [`ResumePlan`] naming the one request
//! that continues it: answer the calls the run is waiting on, continue the
//! partial reply, or send the prompt again.
//!
//! ```
//! use rig::agent::AgentRun;
//! use rig::checkpoint::ResumePlan;
//!
//! let checkpoint = AgentRun::new("hello").interrupt("the host stopped").checkpoint().clone();
//! let resumed = checkpoint.interrupted();
//! assert!(matches!(resumed.plan(), ResumePlan::Replay { .. }));
//! assert_eq!(resumed.history().len(), 1);
//! ```

mod interrupt;
mod saved;
mod store;

pub use interrupt::{InterruptedRun, ResumePlan};
pub use saved::{CHECKPOINT_FORMAT, Checkpoint, PartialReply};
pub use store::{CheckpointError, CheckpointStore, MemoryCheckpoints};

#[cfg(test)]
mod tests;
