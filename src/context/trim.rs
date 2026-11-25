use std::sync::Arc;

use crate::completion::message::{
    CallId, Message, Text, ToolResult, ToolResultContent, UserContent,
};
use crate::memory::{HeuristicTokenCounter, TokenCounter};

use super::Budget;

/// About how many tokens `history` takes, by the same heuristic a
/// [`TrimPlan`] uses.
pub fn estimate(history: &[Arc<Message>]) -> u64 {
    crate::memory::estimate_tokens(history)
}

/// A history made to fit a [`Budget`], and what that took.
///
/// A tool result older than the tail and longer than
/// [`per_result_tokens`](Budget::per_result_tokens) is replaced by one text
/// block naming the call, its tool and its size, so the model knows it can ask
/// again. When that is not enough, the oldest messages go, never into the tail.
/// A dropped assistant turn takes the results answering its calls with it, so
/// no tool result is left referring to a call that is gone.
///
/// ```
/// use rig::completion::Message;
/// use rig::context::{Budget, TrimPlan, estimate};
///
/// let history: Vec<_> = (0..40)
///     .map(|n| std::sync::Arc::new(Message::user(format!("message {n}"))))
///     .collect();
/// let budget = Budget::new(120).reserved(40).tail(0);
/// let plan = TrimPlan::of(&history, &budget);
/// assert!(plan.tokens() <= budget.allowed());
/// assert!(plan.dropped() > 0);
/// ```
#[derive(Debug, Clone)]
pub struct TrimPlan {
    history: Vec<Arc<Message>>,
    stubbed: usize,
    dropped: usize,
    tokens: u64,
}

impl TrimPlan {
    /// The plan for `history` under `budget`.
    pub fn of(history: &[Arc<Message>], budget: &Budget) -> Self {
        let counter = HeuristicTokenCounter::default();
        let tail_start = tail_start(history, budget.tail_tokens(), &counter);
        let tail_len = history.len().saturating_sub(tail_start);

        let mut kept = Vec::with_capacity(history.len());
        let mut stubbed = 0;
        for (at, message) in history.iter().enumerate() {
            match (at < tail_start).then(|| stub(message, budget.per_result_tokens(), &counter)) {
                Some(Some(shorter)) => {
                    stubbed += 1;
                    kept.push(Arc::new(shorter));
                }
                _ => kept.push(Arc::clone(message)),
            }
        }

        let mut dropped = 0;
        while kept.len() > tail_len && tokens(&kept, &counter) > budget.allowed() {
            drop_front(&mut kept);
            dropped += 1;
        }

        Self {
            tokens: tokens(&kept, &counter),
            history: kept,
            stubbed,
            dropped,
        }
    }

    /// The history to send instead.
    pub fn history(&self) -> &[Arc<Message>] {
        &self.history
    }

    /// The history to send instead.
    pub fn into_history(self) -> Vec<Arc<Message>> {
        self.history
    }

    /// How many tool results were replaced by a stub.
    pub fn stubbed(&self) -> usize {
        self.stubbed
    }

    /// How many messages were dropped from the front.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// About how many tokens the history takes.
    pub fn tokens(&self) -> u64 {
        self.tokens
    }

    /// Whether the plan is the history and nothing was trimmed.
    pub fn is_unchanged(&self) -> bool {
        self.stubbed == 0 && self.dropped == 0
    }
}

fn tokens(history: &[Arc<Message>], counter: &HeuristicTokenCounter) -> u64 {
    history
        .iter()
        .map(|message| counter.count(message) as u64)
        .fold(0, u64::saturating_add)
}

/// Removes the oldest message, and the user message answering its tool calls
/// when it asked for any, so no result outlives the call it answers.
fn drop_front(history: &mut Vec<Arc<Message>>) {
    let Some(oldest) = history.first() else {
        return;
    };
    let asked: Vec<CallId> = match oldest.as_ref() {
        Message::Assistant(turn) => turn
            .content
            .iter()
            .filter_map(|block| match block {
                crate::completion::message::AssistantContent::ToolCall(call) => {
                    Some(call.id.clone())
                }
                _ => None,
            })
            .collect(),
        Message::System { .. } | Message::User { .. } => Vec::new(),
    };
    history.remove(0);
    if asked.is_empty() {
        return;
    }
    while let Some(next) = history.first() {
        let Message::User { content } = next.as_ref() else {
            break;
        };
        let results: Vec<&ToolResult> = content
            .iter()
            .filter_map(|item| match item {
                UserContent::ToolResult(result) => Some(result),
                _ => None,
            })
            .collect();
        let answers_only_dropped =
            !results.is_empty() && results.iter().all(|result| asked.contains(&result.call));
        if !answers_only_dropped {
            break;
        }
        history.remove(0);
    }
}

/// The index the tail starts at: the messages from the end that add up to
/// `tail` tokens, which trimming leaves alone.
fn tail_start(history: &[Arc<Message>], tail: u64, counter: &HeuristicTokenCounter) -> usize {
    if tail == 0 {
        return history.len();
    }
    let mut sum = 0u64;
    for (at, message) in history.iter().enumerate().rev() {
        sum = sum.saturating_add(counter.count(message) as u64);
        if sum >= tail {
            return at;
        }
    }
    0
}

/// `message` with every tool result longer than `cap` replaced by a stub, or
/// `None` when it holds no such result.
fn stub(message: &Arc<Message>, cap: u64, counter: &HeuristicTokenCounter) -> Option<Message> {
    let Message::User { content } = message.as_ref() else {
        return None;
    };
    let mut changed = false;
    let content: Vec<UserContent> = content
        .iter()
        .map(|item| match item {
            UserContent::ToolResult(result) => {
                let taken = cost(item, counter);
                if taken <= cap {
                    return item.clone();
                }
                changed = true;
                UserContent::ToolResult(ToolResult {
                    call: result.call.clone(),
                    name: result.name.clone(),
                    content: vec![ToolResultContent::Text(Text::new(stub_text(result, taken)))],
                    is_error: result.is_error,
                })
            }
            other => other.clone(),
        })
        .collect();
    changed.then(|| Message::User { content })
}

/// What one content item of a user message costs, counted as a message of its
/// own, which over-counts by the per-message overhead alone.
fn cost(item: &UserContent, counter: &HeuristicTokenCounter) -> u64 {
    counter.count(&Message::User {
        content: vec![item.clone()],
    }) as u64
}

fn stub_text(result: &ToolResult, tokens: u64) -> String {
    format!(
        "[tool result elided: `{}` returned about {tokens} tokens for call {}. \
         Call it again if you need the output.]",
        result.name, result.call
    )
}
