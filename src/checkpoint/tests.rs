use serde_json::json;
use std::sync::Arc;

use crate::agent::AgentBuilder;
use crate::agent::run::{
    AgentRun, AgentRunStep, ModelTurn, ModelTurnOutcome, PendingToolCall, TurnPolicy,
};
use crate::agent::test_utils::MockCompletionModel;
use crate::checkpoint::{
    CHECKPOINT_FORMAT, Checkpoint, CheckpointStore, MemoryCheckpoints, PartialReply, ResumePlan,
};
use crate::completion::Usage;
use crate::completion::message::{
    AssistantContent, AssistantMessage, Message, Text, ToolCall, ToolFunction, ToolName,
    ToolResultContent, UserContent,
};
use crate::conversation::Conversation;
use crate::tool::{ToolOutput, ToolResult};

fn policy(names: &[&str]) -> TurnPolicy {
    TurnPolicy::new(
        names.iter().map(|name| (*name).to_string()).collect(),
        None,
        None,
    )
    .expect("a policy for the advertised tools")
}

fn raw() -> serde_json::Value {
    json!({"origin": "hand-built test turn"})
}

fn tool_call(id: &str) -> AssistantContent {
    AssistantContent::ToolCall(ToolCall::from_wire(
        id,
        ToolFunction::new(
            ToolName::new("search").expect("a valid tool name"),
            json!({"q": "rig"}),
        ),
    ))
}

fn text_turn(text: &str) -> ModelTurn {
    ModelTurn::new(
        AssistantMessage::default(),
        vec![AssistantContent::text(text)],
        Usage::default(),
        policy(&["search"]),
        raw(),
    )
}

fn call_turn(calls: &[&str]) -> ModelTurn {
    ModelTurn::new(
        AssistantMessage::default(),
        calls.iter().map(|id| tool_call(id)).collect(),
        Usage::default(),
        policy(&["search"]),
        raw(),
    )
}

fn success(output: &str) -> ToolResult {
    ToolResult::success(ToolOutput::text(output))
}

fn answer_open(call: PendingToolCall, output: &str) -> crate::agent::run::ToolAnswer {
    match call {
        PendingToolCall::Execute(call) => call.answer(success(output)),
        PendingToolCall::Malformed(call) => call.answer(None),
    }
}

fn expect_call_model(run: &mut AgentRun) -> (Arc<Message>, usize) {
    match run.next_step().expect("next_step succeeds") {
        AgentRunStep::CallModel { prompt, turn, .. } => (prompt, turn),
        step => panic!("expected CallModel, got {step:?}"),
    }
}

fn expect_call_tools(run: &mut AgentRun) -> Vec<PendingToolCall> {
    match run.next_step().expect("next_step succeeds") {
        AgentRunStep::CallTools { calls } => calls,
        step => panic!("expected CallTools, got {step:?}"),
    }
}

/// Drives `run` to the step after the model turn is accepted.
fn accept(run: &mut AgentRun, turn: ModelTurn) {
    let outcome = run.model_response(turn).expect("the turn is accepted");
    assert!(
        matches!(outcome, ModelTurnOutcome::Continue { .. }),
        "the turn needs no resolution"
    );
}

fn tool_ids(call: &PendingToolCall) -> String {
    match call {
        PendingToolCall::Execute(call) => call.tool_call().id.wire().to_string(),
        PendingToolCall::Malformed(call) => call.tool_call().id.wire().to_string(),
    }
}

#[test]
fn a_checkpoint_round_trips_through_json() {
    let run = AgentRun::new("hello");
    let checkpoint = Checkpoint::of(run, "the host stopped");
    let text = checkpoint.to_json().expect("a checkpoint serializes");

    let restored = Checkpoint::from_json(&text).expect("a checkpoint deserializes");
    assert_eq!(restored.reason(), "the host stopped");
    assert!(restored.partial().is_none());
    assert_eq!(restored.to_json().expect("reserializes"), text);
}

#[test]
fn a_checkpoint_from_another_rig_is_refused_by_name() {
    let run = AgentRun::new("hello");
    let mut value = serde_json::to_value(Checkpoint::of(run, "stopped")).expect("serializes");
    value["format"] = json!(CHECKPOINT_FORMAT + 1);

    let error = Checkpoint::from_json(&value.to_string()).expect_err("the format is refused");
    let message = error.to_string();
    assert!(
        message.contains("another version of rig"),
        "the refusal names the cause: {message}"
    );
}

#[test]
fn an_unknown_key_is_refused() {
    let run = AgentRun::new("hello");
    let mut value = serde_json::to_value(Checkpoint::of(run, "stopped")).expect("serializes");
    value["surprise"] = json!(1);

    Checkpoint::from_json(&value.to_string()).expect_err("an unknown key is refused");
}

#[test]
fn a_run_interrupted_before_its_first_call_replays_the_prompt() {
    let interrupted = AgentRun::new("hello").interrupt("the host stopped");

    assert_eq!(interrupted.reason(), "the host stopped");
    let ResumePlan::Replay { prompt } = interrupted.plan() else {
        panic!("nothing of the attempt survived, so the prompt is sent again");
    };
    assert_eq!(prompt, &Message::user("hello"));
    assert_eq!(interrupted.history(), [Message::user("hello")]);
}

#[test]
fn a_run_interrupted_mid_model_call_resumes_by_replaying_that_call() {
    let mut run = AgentRun::new("hello").max_turns(2);
    let (prompt, turn) = expect_call_model(&mut run);

    let interrupted = run.interrupt("the process died");
    let ResumePlan::Replay { prompt: replay } = interrupted.plan() else {
        panic!("the attempt was not accepted, so it is re-issued");
    };
    assert_eq!(Message::clone(replay), Message::clone(prompt.as_ref()));

    // Resuming from the checkpoint alone re-issues the same call, consuming no
    // further turn.
    let mut resumed = interrupted.checkpoint().clone().into_run();
    let (again, resumed_turn) = expect_call_model(&mut resumed);
    assert_eq!(again, prompt);
    assert_eq!(resumed_turn, turn, "the interrupted call is not a turn");
}

#[test]
fn a_run_interrupted_mid_stream_keeps_the_partial_reply() {
    let mut run = AgentRun::new("hello").max_turns(2);
    let (prompt, _) = expect_call_model(&mut run);

    let partial = PartialReply::new(vec![AssistantContent::text("The answer is")]);
    let interrupted = run.interrupt_with("the stream was cut", partial.clone());

    let ResumePlan::Continue { partial: kept } = interrupted.plan() else {
        panic!("a reply that started arriving continues as a prefill");
    };
    assert_eq!(kept, &partial);
    assert_eq!(
        kept.message(),
        Some(Message::Assistant(AssistantMessage::new(vec![
            AssistantContent::Text(Text::new("The answer is"))
        ]))),
        "the partial reply is one assistant message"
    );

    // The history is untouched: the partial turn was never accepted.
    assert_eq!(interrupted.history(), [Message::user("hello")]);
    let mut resumed = interrupted.checkpoint().clone().into_run();
    let (again, _) = expect_call_model(&mut resumed);
    assert_eq!(again, prompt, "continue sends the same call");
}

#[test]
fn a_run_interrupted_mid_tool_call_resumes_by_answering_only_the_open_calls() {
    let mut run = AgentRun::new("hello").max_turns(3);
    expect_call_model(&mut run);
    accept(&mut run, call_turn(&["a", "b", "c"]));
    let calls = expect_call_tools(&mut run);
    let ids: Vec<String> = calls.iter().map(tool_ids).collect();
    assert_eq!(ids, ["a", "b", "c"]);

    // Two of three answers landed before the process died.
    run.answer(answer_open(calls[0].clone(), "first"))
        .expect("answering a is accepted");
    run.answer(answer_open(calls[2].clone(), "third"))
        .expect("answering c is accepted");

    let interrupted = run.interrupt("the process died");
    let ResumePlan::Answer { calls } = interrupted.plan() else {
        panic!("the run is waiting on tools, so it resumes by answering them");
    };
    assert_eq!(
        calls.iter().map(tool_ids).collect::<Vec<_>>(),
        ["b"],
        "only the unanswered call is open"
    );

    // Resuming from the checkpoint alone asks for exactly that call, then
    // finishes the run without repeating an answered one.
    let mut resumed = interrupted.checkpoint().clone().into_run();
    let open = expect_call_tools(&mut resumed);
    assert_eq!(open.iter().map(tool_ids).collect::<Vec<_>>(), ["b"]);
    resumed
        .answer_all(open.into_iter().map(|call| answer_open(call, "second")))
        .expect("the open call is answerable");

    expect_call_model(&mut resumed);
    accept(&mut resumed, text_turn("done"));
    let response = match resumed.next_step().expect("the run completes") {
        AgentRunStep::Done(response) => response,
        step => panic!("expected Done, got {step:?}"),
    };

    // One assistant row per turn, and every call answered exactly once.
    let assistant_rows = response
        .messages
        .iter()
        .filter(|message| matches!(message, Message::Assistant(_)))
        .count();
    assert_eq!(assistant_rows, 2, "the tool turn and the final answer");
    let answers: Vec<String> = response
        .messages
        .iter()
        .flat_map(|message| match message {
            Message::User { content } => content
                .iter()
                .filter_map(|item| match item {
                    UserContent::ToolResult(result) => Some(result.call.wire().to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    let mut sorted = answers.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        ["a", "b", "c"],
        "no duplicated and no orphaned call"
    );
    Conversation::from_messages(response.messages.clone())
        .validate()
        .expect("the finished run is a canonical conversation");
}

#[test]
fn a_run_interrupted_mid_worker_keeps_each_answered_call() {
    // Two workers ran the same batch and answered out of order.
    let mut run = AgentRun::new("hello").max_turns(3);
    expect_call_model(&mut run);
    accept(&mut run, call_turn(&["slow", "fast"]));
    let calls = expect_call_tools(&mut run);
    run.answer(answer_open(calls[1].clone(), "fast finished"))
        .expect("the second call is answerable first");

    let interrupted = run.interrupt("the host stopped mid-batch");
    let ResumePlan::Answer { calls } = interrupted.plan() else {
        panic!("the batch is still open");
    };
    assert_eq!(calls.iter().map(tool_ids).collect::<Vec<_>>(), ["slow"]);

    let mut resumed = interrupted.checkpoint().clone().into_run();
    let open = expect_call_tools(&mut resumed);
    resumed
        .answer_all(
            open.into_iter()
                .map(|call| answer_open(call, "slow finished")),
        )
        .expect("the rest of the batch is answerable");
    expect_call_model(&mut resumed);
    accept(&mut resumed, text_turn("done"));
    let AgentRunStep::Done(response) = resumed.next_step().expect("completes") else {
        panic!("expected Done");
    };

    // The batch committed as one user message with both results, in call order.
    let results: Vec<(String, String)> = response
        .messages
        .iter()
        .flat_map(|message| match message {
            Message::User { content } => content
                .iter()
                .filter_map(|item| match item {
                    UserContent::ToolResult(result) => Some((result.call.wire().to_string(), {
                        result
                            .content
                            .first()
                            .and_then(ToolResultContent::as_text)
                            .unwrap_or_default()
                            .to_string()
                    })),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(
        results,
        [
            ("slow".to_string(), "slow finished".to_string()),
            ("fast".to_string(), "fast finished".to_string())
        ],
        "each call is answered once, in call order"
    );
}

#[test]
fn the_history_of_an_interruption_closes_every_open_call() {
    let mut run = AgentRun::new("hello").max_turns(3);
    expect_call_model(&mut run);
    accept(&mut run, call_turn(&["a", "b"]));

    // Nothing answered: the run holds an assistant turn whose calls are open.
    let interrupted = run.interrupt("the process died");
    let conversation = Conversation::from_messages(interrupted.history().to_vec());
    conversation
        .validate()
        .expect("no call is left open in the reported history");
}

#[test]
fn a_checkpoint_reaches_a_resumed_agent_through_the_store() {
    let store = MemoryCheckpoints::new();
    let checkpoint = Checkpoint::of(AgentRun::new("hello"), "the process died");
    futures::executor::block_on(store.save("session-1", &checkpoint)).expect("the save lands");

    let loaded = futures::executor::block_on(store.load("session-1"))
        .expect("the load succeeds")
        .expect("the checkpoint is there");
    assert_eq!(loaded.reason(), "the process died");
    assert!(matches!(
        loaded.interrupted().plan(),
        ResumePlan::Replay { .. }
    ));

    // The run continues from the checkpoint alone: the model sees the prompt
    // once, and the run finishes.
    let model = MockCompletionModel::from_turns([crate::agent::test_utils::MockTurn::text(
        "resumed answer",
    )]);
    let probe = model.clone();
    let response = futures::executor::block_on(
        AgentBuilder::new(model)
            .build()
            .resume_checkpoint(loaded)
            .run(),
    )
    .expect("the resumed run completes");
    assert_eq!(response.messages.len(), 2, "the prompt and the answer");
    assert_eq!(
        probe.requests().len(),
        1,
        "the prompt is sent once, not repeated for the resume"
    );
}

#[test]
fn removing_a_checkpoint_leaves_no_trace() {
    let store = MemoryCheckpoints::new();
    let checkpoint = Checkpoint::of(AgentRun::new("hello"), "stopped");
    futures::executor::block_on(store.save("session-1", &checkpoint)).expect("the save lands");
    futures::executor::block_on(store.remove("session-1")).expect("the removal lands");
    assert!(
        futures::executor::block_on(store.load("session-1"))
            .expect("the load succeeds")
            .is_none()
    );
    // Removing a key that holds nothing is not an error.
    futures::executor::block_on(store.remove("session-1")).expect("removal is idempotent");
}

#[test]
fn an_interruption_becomes_a_prompt_error_carrying_its_checkpoint() {
    let mut run = AgentRun::new("hello").max_turns(3);
    expect_call_model(&mut run);
    accept(&mut run, call_turn(&["a"]));

    let error = run.interrupt("the host stopped").into_error();
    let crate::agent::run::response::PromptError::Interrupted {
        checkpoint,
        history,
        reason,
    } = error
    else {
        panic!("an interruption is its own error");
    };
    assert_eq!(reason, "the host stopped");
    assert_eq!(history.len(), run.messages().len());
    assert!(matches!(
        checkpoint.interrupted().plan(),
        ResumePlan::Answer { .. }
    ));
}
