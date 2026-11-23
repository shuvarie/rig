use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::agent::tool::ToolSet;
use crate::message::ToolName;
use crate::tool::{Tool, ToolContext, ToolErrorKind, ToolOutput};

#[derive(Deserialize)]
struct EchoArgs {
    text: String,
}

/// A typed tool, to prove a source carries registrations that were not
/// defined at runtime.
struct Echo;

impl Tool for Echo {
    const NAME: &'static str = "echo";
    type Args = EchoArgs;
    type Output = String;
    type Error = std::convert::Infallible;

    fn description(&self) -> String {
        "Echoes its input".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {"text": {"type": "string"}}})
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(args.text)
    }
}

fn dynamic_echo() -> DynamicTool {
    DynamicTool::new(
        ToolName::new("echo").expect("a valid name"),
        "Echoes its input",
        json!({"type": "object", "properties": {"text": {"type": "string"}}}),
        |args| {
            Box::pin(async move {
                let text = args
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| crate::tool::ToolExecutionError::invalid_args("text"))?;
                Ok(ToolOutput::text(text))
            })
        },
    )
}

#[test]
fn a_fixed_list_is_a_source_of_its_own_tools() {
    let source = vec![dynamic_echo()];
    let tools = source.tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools.first().map(DynamicTool::name),
        Some(&ToolName::new("echo").expect("valid"))
    );
}

#[test]
fn a_boxed_source_reports_the_same_tools() {
    let source: Arc<dyn ToolSource> = Arc::new(vec![dynamic_echo()]);
    assert_eq!(source.tools().len(), 1);
}

#[test]
fn a_server_takes_every_tool_of_a_source() {
    let registry = crate::tool::server::ToolServer::new()
        .tool_source(vec![dynamic_echo()])
        .run();
    assert!(registry.toolset().contains("echo"));
}

#[tokio::test]
async fn a_tool_set_offers_its_registrations_as_dynamic_tools() {
    let mut set = ToolSet::default();
    set.add_tool(Echo);
    let tools = set.tools();
    let echo = tools.first().expect("the registration is offered");
    assert_eq!(echo.name(), &ToolName::new("echo").expect("valid"));

    let output = echo
        .run(json!({"text": "hello"}))
        .await
        .expect("the registration dispatches");
    assert_eq!(output.as_text(), Some("hello"));
}

#[tokio::test]
async fn a_tool_set_offers_its_dynamic_registrations_unchanged() {
    let mut set = ToolSet::default();
    set.add_dynamic_tool(dynamic_echo());
    let tools = set.tools();
    let echo = tools.first().expect("the registration is offered");

    let output = echo
        .run(json!({"text": "hello"}))
        .await
        .expect("the registration dispatches");
    assert_eq!(output.as_text(), Some("hello"));

    let error = echo
        .run(json!({}))
        .await
        .expect_err("missing arguments fail");
    assert_eq!(error.kind(), ToolErrorKind::InvalidArgs);
}
