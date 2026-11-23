//! Registries the runtime pulls tools from.
//!
//! A host that keeps its own tool registry hands over a [`ToolSource`] rather
//! than pushing each tool into an agent, so the runtime needs no dependency on
//! where the tools come from. A fixed `Vec<DynamicTool>` and
//! [`ToolSet`](crate::tool::ToolSet) are sources already.
//!
//! ```
//! use rig::message::ToolName;
//! use rig::tool::{DynamicTool, ToolExecutionError, ToolOutput, ToolSource};
//! use serde_json::json;
//!
//! let echo = DynamicTool::new(ToolName::new("echo")?, "Echoes its input", json!({}), |args| {
//!     Box::pin(async move {
//!         Ok::<_, ToolExecutionError>(ToolOutput::json(args))
//!     })
//! });
//! let source = vec![echo];
//! assert_eq!(source.tools().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::sync::Arc;

use crate::tool::DynamicTool;
use crate::wasm_compat::{WasmCompatSend, WasmCompatSync};

/// A registry the runtime takes tools from.
///
/// [`tools`](ToolSource::tools) reports the current set on every call, so a
/// registry that changes over time supplies a fresh list each time it is
/// asked. Register every tool of a source at once with
/// [`ToolServer::tool_source`](crate::tool::server::ToolServer::tool_source) or
/// [`AgentBuilder::tool_source`](crate::agent::AgentBuilder::tool_source).
///
/// This trait is object safe: a host passes `Arc<dyn ToolSource>` without the
/// runtime knowing the concrete registry.
pub trait ToolSource: WasmCompatSend + WasmCompatSync {
    /// Every tool this source offers now, in a stable order.
    fn tools(&self) -> Vec<DynamicTool>;
}

impl ToolSource for Vec<DynamicTool> {
    fn tools(&self) -> Vec<DynamicTool> {
        self.clone()
    }
}

impl<T> ToolSource for Arc<T>
where
    T: ToolSource + ?Sized,
{
    fn tools(&self) -> Vec<DynamicTool> {
        (**self).tools()
    }
}

#[cfg(test)]
mod tests;
