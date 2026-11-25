use std::sync::Arc;

use crate::completion::message::{
    AssistantContent, AssistantMessage, Message, Text, ToolCall, ToolFunction, ToolName,
    ToolResultContent, UserContent,
};
use crate::context::{Budget, ContextHook, TrimPlan, estimate};
use crate::conversation::Conversation;

fn user(text: impl Into<String>) -> Arc<Message> {
    Arc::new(Message::user(text))
}

fn call(id: &str) -> ToolCall {
    ToolCall::from_wire(
        id,
        ToolFunction::new(
            ToolName::new("search").expect("a valid tool name"),
            serde_json::json!({}),
        ),
    )
}

/// An assistant turn asking for `search` under `id`.
fn tool_call(id: &str) -> Arc<Message> {
    Arc::new(Message::Assistant(AssistantMessage::new(vec![
        AssistantContent::ToolCall(call(id)),
    ])))
}

/// The user message answering that call, with `padding` characters of output.
fn tool_result(id: &str, padding: usize) -> Arc<Message> {
    Arc::new(Message::User {
        content: vec![UserContent::ToolResult(call(id).result(vec![
            ToolResultContent::Text(Text::new("x".repeat(padding))),
        ]))],
    })
}

#[test]
fn a_budget_reserves_the_answer_and_the_preamble() {
    let budget = Budget::new(10_000).reserved(2_000).preamble(1_000);
    assert_eq!(budget.allowed(), 7_000);
    assert!(budget.fits(7_000));
    assert!(budget.over(7_001));
}

#[test]
fn reserved_room_beyond_the_window_leaves_nothing_to_spend() {
    let budget = Budget::new(1_000).reserved(2_000);
    assert_eq!(budget.allowed(), 0);
    assert!(budget.over(1));
}

#[test]
fn a_history_that_fits_is_untouched() {
    let history: Vec<_> = (0..3).map(|n| user(format!("message {n}"))).collect();
    let budget = Budget::new(200_000);
    let plan = TrimPlan::of(&history, &budget);

    assert!(plan.is_unchanged());
    assert_eq!(plan.stubbed(), 0);
    assert_eq!(plan.dropped(), 0);
    assert_eq!(plan.history().len(), 3);
    assert_eq!(plan.tokens(), estimate(&history));
}

#[test]
fn an_old_long_tool_result_becomes_a_stub_that_names_the_call() {
    let history = vec![tool_call("a"), tool_result("a", 40_000), user("and now?")];
    // A one-token tail reaches only the last message, so the result is old
    // enough to trim.
    let budget = Budget::new(200_000).tail(1).per_result(100);
    let plan = TrimPlan::of(&history, &budget);

    assert_eq!(plan.stubbed(), 1);
    assert_eq!(plan.dropped(), 0);
    assert_eq!(plan.history().len(), 3, "a stub replaces, it does not drop");

    let Some(Message::User { content }) = plan.history().get(1).map(|m| m.as_ref()) else {
        panic!("the result is still a user message");
    };
    let Some(UserContent::ToolResult(result)) = content.first() else {
        panic!("the result is still a tool result");
    };
    let text = result
        .content
        .first()
        .and_then(ToolResultContent::as_text)
        .expect("a stub is text");
    assert!(text.contains("search"), "{text}");
    assert!(text.contains("elided"), "{text}");
    assert!(estimate(plan.history()) < estimate(&history));
}

#[test]
fn a_short_tool_result_is_left_alone() {
    let history = vec![tool_call("a"), tool_result("a", 40), user("next")];
    let budget = Budget::new(200_000).tail(1).per_result(1_000);
    let plan = TrimPlan::of(&history, &budget);
    assert_eq!(plan.stubbed(), 0);
    assert!(plan.is_unchanged());
}

#[test]
fn the_tail_is_never_trimmed() {
    // One large result at the very end, inside a tail that holds everything.
    let history = vec![tool_call("a"), tool_result("a", 40_000)];
    let budget = Budget::new(50).reserved(10).tail(200_000).per_result(10);
    let plan = TrimPlan::of(&history, &budget);

    assert_eq!(plan.stubbed(), 0, "the tail is not stubbed");
    assert_eq!(plan.dropped(), 0, "the tail is not dropped");
    assert_eq!(plan.history().len(), 2);
}

#[test]
fn a_history_over_budget_drops_its_oldest_messages_only() {
    let history: Vec<_> = (0..40).map(|n| user(format!("message {n}"))).collect();
    let budget = Budget::new(120).reserved(40).tail(0);
    let plan = TrimPlan::of(&history, &budget);

    assert!(plan.dropped() > 0);
    assert!(plan.tokens() <= budget.allowed(), "{}", plan.tokens());
    assert!(plan.history().len() < history.len());
    // What survives is the end of the conversation, in order.
    let last = plan.history().last().expect("something remains");
    assert_eq!(
        last.as_ref(),
        history.last().expect("the input ends").as_ref()
    );
}

#[test]
fn dropping_a_call_takes_the_result_that_answers_it() {
    let history = vec![
        user("first question"),
        tool_call("a"),
        tool_result("a", 40_000),
        user("second question"),
        tool_call("b"),
        tool_result("b", 40),
        user("third question"),
    ];
    // Room for the stubs and the newest turns only.
    let budget = Budget::new(110).reserved(50).tail(0).per_result(50);
    let plan = TrimPlan::of(&history, &budget);

    assert!(plan.dropped() > 0, "dropped {}", plan.dropped());
    let conversation = Conversation::from_messages(plan.history().to_vec());
    conversation
        .validate()
        .expect("a trimmed history is a canonical conversation");
    assert!(
        plan.dropped() > 1,
        "dropping the oldest turn took its result with it, in one step"
    );
    let newest = plan.history().last().expect("the newest turn remains");
    assert_eq!(
        newest.as_ref(),
        history.last().expect("the input ends").as_ref()
    );
}

#[test]
fn a_turn_that_fits_continues_without_a_patch() {
    let hook = ContextHook::new(Budget::new(200_000));
    let history = vec![user("hello")];
    let plan = hook.plan(&history);
    assert!(plan.is_unchanged());
    assert_eq!(hook.budget().window(), 200_000);
}

#[test]
fn a_turn_over_budget_plans_a_shorter_history() {
    let history: Vec<_> = (0..40).map(|n| user(format!("message {n}"))).collect();
    let hook = ContextHook::new(Budget::new(120).reserved(40).tail(0));
    let plan = hook.plan(&history);
    assert!(plan.dropped() > 0);
    assert!(plan.tokens() <= hook.budget().allowed());
}
