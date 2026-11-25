//! Context budgeting: what a request may spend, and what is trimmed to fit.
//!
//! A [`Budget`] states the window, the room the answer needs, the tail never
//! touched, and the most one tool result may keep. [`TrimPlan`] applies it to a
//! history: old tool results are replaced by a stub that names the call, and
//! the oldest messages are dropped when that is not enough. [`ContextHook`]
//! does this per turn.
//!
//! ```
//! use rig::completion::Message;
//! use rig::context::{Budget, TrimPlan};
//!
//! let history = vec![std::sync::Arc::new(Message::user("hello"))];
//! let budget = Budget::new(200_000).tail(0).per_result(1_000);
//! let plan = TrimPlan::of(&history, &budget);
//! assert!(plan.is_unchanged());
//! ```

mod budget;
mod hook;
mod trim;

pub use budget::Budget;
pub use hook::ContextHook;
pub use trim::{TrimPlan, estimate};

#[cfg(test)]
mod tests;
