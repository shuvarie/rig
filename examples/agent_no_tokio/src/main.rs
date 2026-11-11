//! An agent run driven by a plain thread instead of tokio.
//!
//! rig-agent has no runtime of its own: [`rig::agent::AgentRunner::run_channel`] hands back a
//! plain future plus a bounded [`RunEvents`] feed, so the future can be driven by any
//! executor — here a `std` thread polling it with `futures::executor::block_on` — while a
//! synchronous loop (think: a game frame, an ECS system) drains the events with `try_next`
//! and never blocks on the run. The HTTP transport (`rig-reqwest`) brings its own private
//! tokio runtime for the wire; this crate's manifest depends on neither tokio nor reqwest.
//! The client holds the transport erased ([`rig::http_client::DynHttpClient`]), so no
//! transport type reaches this crate's signatures.
//!
//! Requires `OPENAI_API_KEY`.

use std::{thread, time::Duration};

use anyhow::Result;
use rig::agent::MultiTurnStreamItem;
use rig::prelude::*;
use rig::providers::openai::{self, OpenAI};
use rig::streaming::{Item, StreamEvent};

const PREAMBLE: &str = "You are a comedian here to entertain the user using humour and jokes.";
const PROMPT: &str = "Entertain me!";
/// How long the "frame" loop sleeps between ticks when nothing is ready.
const FRAME: Duration = Duration::from_millis(16);

/// Print what one event shows, as a terminal view would.
fn show(event: MultiTurnStreamItem) {
    match event {
        MultiTurnStreamItem::StreamAssistantItem(Item::Event(StreamEvent::Text {
            text, ..
        })) => {
            print!("{text}");
        }
        MultiTurnStreamItem::ToolExecutionCommitted { tool_call, .. } => {
            println!("\n[tool {}]", tool_call.function.name);
        }
        MultiTurnStreamItem::FinalResponse(_) => println!(),
        _ => {}
    }
}

fn main() -> Result<()> {
    // The client sends through the shared reqwest transport, held erased:
    // the model is `Model<OpenAiWire>`.
    let agent = AgentBuilder::new(OpenAI::from_env()?.completion(openai::GPT_4O))
        .preamble(PREAMBLE)
        .build();

    // Split the run: a future for the driver thread, an event feed for the frame loop.
    let (run, mut events) = agent.prompt(PROMPT).run_channel();
    let driver = thread::spawn(move || futures::executor::block_on(run));

    // The "frame loop": poll the feed without blocking, tick, repeat.
    loop {
        while let Some(event) = events.try_next() {
            show(event);
        }
        if driver.is_finished() {
            break;
        }
        thread::sleep(FRAME);
    }

    // The run has ended and closed the feed: take whatever it buffered last.
    while let Some(event) = events.try_next() {
        show(event);
    }

    let response = driver
        .join()
        .map_err(|_| anyhow::anyhow!("the driver thread panicked"))??;

    println!(
        "done: {} completion call(s), {} total tokens",
        response.completion_calls().len(),
        response.usage().total_tokens.unwrap_or(0)
    );
    Ok(())
}
