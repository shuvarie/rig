//! Conversations over time, over the crate's [`Conversation`](crate::conversation::Conversation):
//! - [`recent`]: which newest messages a request sends.
//! - [`CompactionPolicy`], [`apply_summary`], [`undo_summary`]: compacting a long conversation.
//! - [`load`], [`save`], [`Session`], [`Writer`]: saving and resuming, in the one format
//!   [`FORMAT`]. Reading is strict: a session of another format, or that does not read whole,
//!   is refused and nothing is written.
//!   [`load_async`] and [`save_async`] keep a conversation
//!   in any [`AsyncStore`], such as a database or IndexedDB.
//! - [`history`], [`as_of`]: how a conversation got where it is.
//! - [`rewrite`] (what it promises is on it): rewriting a saved session.
//! - [`SessionMeta`], [`read_conversation`]: what a session list shows, read without folding
//!   every log.
//! - [`Store`], [`MemoryStore`]: where a session is kept, plus [`Files`] and
//!   [`PluginFiles`] for named files on their own. Every [`Store`] is an
//!   [`AsyncStore`], its calls done when made. A log write appends only if the log is as long as
//!   the writer read it, the store checking and writing in one step, so of two saves that read
//!   one log the second is refused ([`SessionError::Changed`]) and writes nothing to it.
//!   `conformance` (feature `test-utils`) checks a store against the contract.
//!
//! ```
//! use crate::message::Message;
//! use crate::memory::recent::{LastMessages, view};
//! use crate::memory::{MemoryStore, load, save};
//!
//! let store = MemoryStore::default();
//! let mut conversation = load(&store, "user-123")?;
//! conversation.extend([Message::user("My name is Alice."), Message::assistant("Hi Alice!")]);
//! save(&store, "user-123", &mut conversation)?;
//!
//! let resumed = load(&store, "user-123")?;
//! assert_eq!(resumed, conversation);
//! assert_eq!(view(resumed.messages(), &LastMessages(1)).len(), 1);
//! # Ok::<(), crate::memory::SessionError>(())
//! ```
//!
//! [`Conversation`]: crate::conversation::Conversation

mod blobs;
mod compaction;
#[cfg(any(test, feature = "test-utils"))]
pub mod conformance;
mod fork;
mod format;
mod listing;
mod meta;
pub mod recent;
mod rewrite;
mod saved;
mod session;
mod store;
mod tokens;
mod writer;

pub use blobs::BLOBS;
pub use compaction::{
    AFTER_OVERFLOW, COMPACTED, ClearPlan, ClearToolOutputs, Cleared, CompactReason, Compacted,
    CompactionPolicy, NearWindow, Summarized, Summarizer, SummaryCut, SummaryError, SummaryLimits,
    SummaryPlan, SummaryPrompts, SummaryState, TrackArgument, TrackedSet, Uncompacted, UndoError,
    apply_summary, estimate_tokens, is_output_notes, known_limits, summarized_in_log,
    summarized_prefix, summary_author, summary_in, summary_state, undo_summary,
};
pub use format::{FORMAT, LogRewrite, Repair, SESSION, SessionError, Stamp};
pub use listing::read_conversation;
pub use meta::{META, SessionMeta, excerpt, title_of};
pub use rewrite::{
    Messages, Place, Rewrite, Rewritten, RewrittenLog, Strings, map_strings, rewrite, rewrite_file,
};
pub use saved::{CURRENT, MODEL, TITLE};
pub use session::{
    LogRecord, SavedAgent, Session, Unreadable, as_of, history, load, load_async, save, save_async,
};
pub use store::{
    AsyncStore, Files, LogWriteError, MemoryStore, PLUGINS, PluginFiles, Store, check_log_name,
    check_name,
};
pub use tokens::{
    ATTACHMENT_TOKENS, Anchor, HeuristicTokenCounter, OUTPUT_MARGIN, Prepared, RequestStart,
    TokenCounter, next_request_tokens,
};
pub use writer::Writer;
