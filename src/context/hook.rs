use crate::agent::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext};
use crate::agent::run::patch::RequestPatch;

use super::{Budget, TrimPlan, estimate};

/// Keeps every request inside a [`Budget`] before it is sent.
///
/// The hook estimates the turn's prompt and history on every model call and
/// patches the history with a [`TrimPlan`] when they do not fit. A turn whose
/// tail and preamble alone exceed the window is stopped rather than sent: no
/// trim can make it fit.
///
/// The estimate is the same character-based heuristic the rest of the crate
/// uses, so it is a forecast, not a measurement. A host that wants the
/// provider's own count can pair this hook with its own accounting.
///
/// ```
/// use rig::context::{Budget, ContextHook};
///
/// let hook = ContextHook::new(Budget::new(200_000));
/// assert_eq!(hook.budget().window(), 200_000);
/// ```
#[derive(Debug, Clone)]
pub struct ContextHook {
    budget: Budget,
}

impl ContextHook {
    /// A hook enforcing `budget`.
    pub fn new(budget: Budget) -> Self {
        Self { budget }
    }

    /// The budget it enforces.
    pub fn budget(&self) -> &Budget {
        &self.budget
    }

    /// The plan this hook would apply to `history`.
    pub fn plan(&self, history: &[std::sync::Arc<crate::completion::Message>]) -> TrimPlan {
        TrimPlan::of(history, &self.budget)
    }
}

impl AgentHook for ContextHook {
    fn name(&self) -> Option<String> {
        Some(format!("context-budget/{}", self.budget.window()))
    }

    async fn on_completion_call(
        &self,
        _ctx: &HookContext,
        event: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        let sent = estimate(event.history).saturating_add(estimate(std::slice::from_ref(
            &std::sync::Arc::new(event.prompt.clone()),
        )));
        if self.budget.fits(sent) {
            return CompletionCallAction::Continue;
        }

        let plan = TrimPlan::of(event.history, &self.budget);
        if plan.is_unchanged() {
            return CompletionCallAction::stop(format!(
                "the request takes about {sent} tokens, which this {} token budget cannot hold: \
                 the tail and preamble alone leave no room to trim",
                self.budget.window()
            ));
        }
        tracing::debug!(
            target: "rig",
            stubbed = plan.stubbed(),
            dropped = plan.dropped(),
            tokens = plan.tokens(),
            "trimmed the request to its context budget"
        );
        CompletionCallAction::patch(RequestPatch::new().history(plan.into_history()))
    }
}
