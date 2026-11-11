//! Shared fixtures, tiny tools, and durable assertions for the live provider suites.
//!
//! Its consumers are feature-gated, so one build sees a subset: the Bedrock
//! suites behind `bedrock`, the image and audio ones behind `image` and
//! `audio`. That is why unused items and imports are allowed here.

#![allow(dead_code, unused_imports)]

use futures::StreamExt;

use rig_agent::agent::MultiTurnStreamItem;

use rig_agent::completion::PromptError;

use rig_agent::agent::StreamingResult;

use rig_agent::completion::AssistantContent;

use rig_core::embeddings::Embedding;

use rig_core::streaming::{Item, StreamEvent};

use rig_core::streaming::CompletionStream;

use rig_core::tool::Tool;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// System instruction for the basic completion smoke scenario.
pub const BASIC_PREAMBLE: &str = "You are a concise assistant. Answer directly.";
/// Rust and memory-safety question used by basic completion smoke tests.
pub const BASIC_PROMPT: &str = "In one or two sentences, explain what Rust programming language is and why memory safety matters.";

/// Fixed context documents defining the fictional glarb-glarb.
pub const CONTEXT_DOCS: [&str; 3] = [
    "Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets.",
    "Definition of a *glarb-glarb*: A glarb-glarb is an ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.",
    "Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.",
];
/// Question answered from the fixed context documents.
pub const CONTEXT_PROMPT: &str = "What does \"glarb-glarb\" mean?";

/// System instruction requiring arithmetic tool use.
pub const TOOLS_PREAMBLE: &str = "You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.";
/// Subtraction question used by tool-call smoke tests.
pub const TOOLS_PROMPT: &str = "Calculate 2 - 5.";

/// Manifest-anchored glob for loader fixture source files.
pub const LOADERS_GLOB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/loaders/*.rs");
/// Question that identifies the agent-building loader fixture.
pub const LOADERS_PROMPT: &str = "Which fixture file builds an agent from the loaders test fixtures? Answer with just the file name.";

/// Plain-text system instruction used by streaming smoke tests.
pub const STREAMING_PREAMBLE: &str = "You are a concise assistant. Answer directly in plain text.";
/// Solar-eclipse question used by streaming smoke tests.
pub const STREAMING_PROMPT: &str = "In one short paragraph, explain what a solar eclipse is.";

/// System instruction for a streamed arithmetic tool roundtrip.
pub const STREAMING_TOOLS_PREAMBLE: &str =
    "You are a calculator. Use the provided tools before answering arithmetic questions.";
/// Subtraction question used by streamed tool tests.
pub const STREAMING_TOOLS_PROMPT: &str = "Calculate 2 - 5.";

/// Deterministic result returned by the alpha signal tool.
#[cfg(feature = "bedrock")]
pub const ALPHA_SIGNAL_OUTPUT: &str = "crimson-harbor";

/// System instruction used to assert tool and text event ordering.
#[cfg(feature = "bedrock")]
pub const ORDERED_TOOL_STREAM_PREAMBLE: &str = "\
You must call the requested tool before writing any normal text. \
After the tool result is available, do not call any more tools and answer in one short sentence that includes the exact tool output.";
/// Prompt used to assert tool and text event ordering.
#[cfg(feature = "bedrock")]
pub const ORDERED_TOOL_STREAM_PROMPT: &str = "\
Call `lookup_harbor_label` exactly once before answering. \
After the tool result is available, answer in one short sentence that includes the exact tool output.";

/// Question about the committed image fixture.
#[cfg(any(feature = "image", feature = "bedrock"))]
pub const IMAGE_PROMPT: &str =
    "A lighthouse on a rocky cliff at sunrise, painted in a clean illustrative style.";

/// Source text for the person-extraction smoke test.
#[cfg(feature = "bedrock")]
pub const EXTRACTOR_TEXT: &str = "Hello, my name is Ada Lovelace and I work as a mathematician.";

/// Manifest-anchored path to the committed audio fixture.
pub const AUDIO_FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/en-us-natural-speech.mp3"
);
/// Manifest-anchored path to the committed image fixture.
#[cfg(any(feature = "image", feature = "bedrock"))]
pub const IMAGE_FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/data/camponotus_flavomarginatus_ant.jpg"
);

/// Fixed text inputs used to check embedding count and dimensions.
pub const EMBEDDING_INPUTS: [&str; 3] = [
    "Rust values memory safety and predictable performance.",
    "Streaming responses arrive incrementally instead of all at once.",
    "Embeddings turn text into numeric vectors for similarity search.",
];

/// Person fields used to test required extraction properties.
#[derive(Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(description = "")]
#[cfg(feature = "bedrock")]
pub struct SmokePerson {
    /// Extracted first name.
    #[schemars(required, description = "")]
    pub first_name: Option<String>,
    /// Extracted last name.
    #[schemars(required, description = "")]
    pub last_name: Option<String>,
    /// Extracted occupation.
    #[schemars(required, description = "")]
    pub job: Option<String>,
}

#[derive(Deserialize)]
/// Integer operands shared by arithmetic tools.
pub struct OperationArgs {
    /// First operand, and minuend for subtraction.
    pub x: i32,
    /// Second operand, and subtrahend for subtraction.
    pub y: i32,
}

#[derive(Deserialize)]
/// Empty object accepted by zero-argument tools.
pub struct EmptyArgs {}

#[derive(Debug)]
/// Error type required by the deterministic arithmetic and signal tools.
pub struct MathError;

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Math error")
    }
}

impl std::error::Error for MathError {}

/// Tool that adds its two integer arguments: rig-agent's mock, the same on the wire.
pub use rig_agent::test_utils::MockAddTool as Adder;

#[derive(Deserialize, Serialize)]
/// Tool that subtracts its second integer argument from the first.
pub struct Subtract;

impl Tool for Subtract {
    const NAME: &'static str = "subtract";
    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

    fn description(&self) -> String {
        "Subtract y from x (i.e.: x - y)".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::from_str(
                r#"{"type":"object","properties":{"x":{"type":"number","description":"The number to subtract from"},"y":{"type":"number","description":"The number to subtract"}},"required":["x","y"]}"#,
            )
            .expect("subtract schema should deserialize")
    }

    async fn call(
        &self,
        _context: &mut rig_core::tool::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(args.x - args.y)
    }
}

#[derive(Deserialize, Serialize)]
#[cfg(feature = "bedrock")]
pub struct AlphaSignal;

#[cfg(feature = "bedrock")]
impl Tool for AlphaSignal {
    const NAME: &'static str = "lookup_harbor_label";
    type Error = MathError;
    type Args = EmptyArgs;
    type Output = String;

    fn description(&self) -> String {
        "Return the alpha signal marker.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
            "required": [],
        })
    }

    async fn call(
        &self,
        _context: &mut rig_core::tool::ToolContext,
        _args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(ALPHA_SIGNAL_OUTPUT.to_string())
    }
}

/// Sends [`BASIC_PROMPT`] through an agent on `model` under [`BASIC_PREAMBLE`]
/// and asserts a non-empty answer.
pub async fn assert_completion_smoke(
    model: impl Into<rig_core::DynModel<rig_core::operation::Completion>>,
) {
    let response = rig_agent::AgentBuilder::new(model)
        .preamble(BASIC_PREAMBLE)
        .build()
        .prompt(BASIC_PROMPT)
        .await
        .expect("completion should succeed")
        .output();
    assert_nonempty_response(&response);
}

/// Assert that a response contains non-whitespace text.
pub fn assert_nonempty_response(response: &str) {
    let trimmed = response.trim();

    assert!(
        !trimmed.is_empty(),
        "Response was empty or whitespace-only."
    );
}

/// Assert a nonempty response contains at least one expected substring, ignoring ASCII case.
pub fn assert_contains_any_case_insensitive(response: &str, expected: &[&str]) {
    assert_nonempty_response(response);

    let response_lower = response.to_ascii_lowercase();
    let matched = expected
        .iter()
        .any(|needle| response_lower.contains(&needle.to_ascii_lowercase()));

    assert!(
        matched,
        "Response {response:?} did not contain any of {expected:?}."
    );
}

/// Assert a nonempty response contains every expected substring, ignoring ASCII case.
pub fn assert_contains_all_case_insensitive(response: &str, expected: &[&str]) {
    assert_nonempty_response(response);

    let response_lower = response.to_ascii_lowercase();
    let missing: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|needle| !response_lower.contains(&needle.to_ascii_lowercase()))
        .collect();

    assert!(
        missing.is_empty(),
        "Response {response:?} did not contain all of {expected:?}; missing {missing:?}."
    );
}

/// Assert a response mentions the number, accepting minus/negative wording for negatives.
pub fn assert_mentions_expected_number(response: &str, expected: i32) {
    assert_nonempty_response(response);

    let response_lower = response.to_ascii_lowercase();
    let abs = expected.abs();
    let mut candidates = vec![expected.to_string()];

    if expected < 0 {
        candidates.push(format!("minus {abs}"));
        candidates.push(format!("negative {abs}"));
    }

    let matched = candidates
        .iter()
        .any(|candidate| response_lower.contains(&candidate.to_ascii_lowercase()));

    assert!(
        matched,
        "Response {response:?} did not mention the expected number {expected:?}."
    );
}

/// Assert that a generated byte payload is nonempty.
pub fn assert_nonempty_bytes(bytes: &[u8]) {
    assert!(!bytes.is_empty(), "Expected non-empty bytes.");
}

/// Assert the expected embedding count and nonzero, consistent vector dimensions.
pub fn assert_embeddings_nonempty_and_consistent(embeddings: &[Embedding], expected_count: usize) {
    assert_eq!(
        embeddings.len(),
        expected_count,
        "Expected {expected_count} embeddings but received {}.",
        embeddings.len()
    );

    let mut expected_dims = None;

    for embedding in embeddings {
        assert!(
            !embedding.vec.is_empty(),
            "Embedding for {:?} was empty.",
            embedding.document
        );

        let dims = embedding.vec.len();
        match expected_dims {
            Some(previous_dims) => assert_eq!(
                dims, previous_dims,
                "Expected consistent embedding dimensionality."
            ),
            None => expected_dims = Some(dims),
        }
    }
}

// ---------------------------------------------------------------------------
// Model-turn termination metadata.
//
// One implementation, driven by every provider suite that covers the feature.
// That is the portability claim as code: if a provider needed its own probe,
// the metadata would not be provider-neutral.
// ---------------------------------------------------------------------------

/// Drain the stream, propagating errors and requiring an agent final response.
pub async fn collect_stream_final_response(
    stream: &mut StreamingResult,
) -> Result<String, PromptError> {
    let mut final_response = None;

    while let Some(item) = stream.next().await {
        if let MultiTurnStreamItem::FinalResponse(response) = item? {
            final_response = Some(response.output().to_owned());
        }
    }

    Ok(final_response.expect("stream should yield a final response"))
}

/// Provider tool-call metadata retained for signature and parameter checks.
pub struct ToolCallRecord {
    /// Tool name emitted by the provider.
    pub name: String,
}

/// Observations from a raw provider completion stream before agent tool execution.
pub struct RawStreamObservation {
    /// Text accumulated from provider text deltas.
    pub text: String,
    /// Completed tool calls in stream order.
    pub tool_calls: Vec<rig_core::message::ToolCall>,
    /// Tool-call metadata in stream order.
    pub tool_call_records: Vec<ToolCallRecord>,
    /// Formatted stream errors in observation order.
    pub errors: Vec<String>,
    /// Whether a provider final event was observed.
    pub got_final: bool,
    /// Event-kind labels in stream order for ordering assertions.
    pub events: Vec<&'static str>,
}

impl RawStreamObservation {
    fn new() -> Self {
        Self {
            text: String::new(),
            tool_calls: Vec::new(),
            tool_call_records: Vec::new(),
            errors: Vec::new(),
            got_final: false,
            events: Vec::new(),
        }
    }
}

/// Drain a raw provider stream into text, tool, error, and final-event observations.
pub async fn collect_raw_stream_observation(mut stream: CompletionStream) -> RawStreamObservation
where
{
    let mut observation = RawStreamObservation::new();

    while let Some(item) = stream.next().await {
        match item {
            Ok(Item::Event(StreamEvent::Text { text, .. })) => {
                observation.text.push_str(&text);
                observation.events.push("text");
            }
            Ok(Item::Event(StreamEvent::End {
                content: AssistantContent::ToolCall(tool_call),
                ..
            })) => {
                observation.tool_calls.push(tool_call.clone());
                observation.tool_call_records.push(ToolCallRecord {
                    name: tool_call.function.name.into(),
                });
                observation.events.push("tool_call");
            }
            Ok(Item::Event(StreamEvent::Arguments { .. })) => {
                observation.events.push("tool_call_delta");
            }
            Ok(Item::Event(StreamEvent::End {
                content: AssistantContent::Reasoning(_),
                ..
            })) => {
                observation.events.push("reasoning");
            }
            Ok(Item::Event(StreamEvent::Reasoning { .. })) => {
                observation.events.push("reasoning_delta");
            }
            Ok(Item::Unknown(_)) => {
                observation.events.push("unknown");
            }
            Ok(Item::Event(StreamEvent::Start { .. } | StreamEvent::End { .. })) => {}
            Err(error) => {
                observation.errors.push(error.to_string());
                observation.events.push("error");
            }
        }
    }
    match stream.finish().await {
        Ok(_) => {
            observation.got_final = true;
            observation.events.push("final");
        }
        // The error the stream already yielded is not reported twice.
        Err(error) if observation.errors.is_empty() => {
            observation.errors.push(error.to_string());
            observation.events.push("error");
        }
        Err(_) => {}
    }

    observation
}

fn first_event_index(events: &[&'static str], expected: &'static str) -> Option<usize> {
    events.iter().position(|event| *event == expected)
}

/// Assert a successful raw stream contains the expected tool before any text that is emitted.
pub fn assert_raw_stream_tool_call_precedes_text(
    observation: &RawStreamObservation,
    expected_tool: &str,
) {
    assert!(
        observation.errors.is_empty(),
        "raw stream should not emit errors: {:?}",
        observation.errors
    );
    assert!(
        observation.got_final,
        "raw stream should emit a final response"
    );

    let record = observation
        .tool_call_records
        .iter()
        .find(|record| record.name == expected_tool)
        .unwrap_or_else(|| {
            panic!(
                "expected raw stream tool call for {expected_tool}, saw {:?}",
                observation
                    .tool_call_records
                    .iter()
                    .map(|record| record.name.as_str())
                    .collect::<Vec<_>>()
            )
        });

    assert!(
        first_event_index(&observation.events, "tool_call").is_some(),
        "expected a tool_call event for {expected_tool}, saw {:?}",
        observation.events
    );

    if let Some(first_text) = first_event_index(&observation.events, "text") {
        let first_tool_call = first_event_index(&observation.events, "tool_call")
            .expect("raw stream should emit a tool_call event");
        assert!(
            first_tool_call < first_text,
            "expected the raw stream to emit a tool call before any text, saw events {:?}",
            observation.events
        );
    }

    let _ = record;
}

/// Assert the loader answer identifies the agent-building fixture.
pub fn assert_loader_answer_is_relevant(response: &str) {
    assert_contains_any_case_insensitive(
        response,
        &[
            "agent_with_loaders",
            "agent_with_loaders.rs",
            "agent with loaders",
        ],
    );
}
/// Fixed text used by audio-generation smoke tests.
#[cfg(feature = "audio")]
pub const AUDIO_TEXT: &str = "The quick brown fox jumps over the lazy dog.";
