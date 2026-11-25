use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::wasm_compat::{WasmCompatSend, WasmCompatSync};

use super::Checkpoint;

/// Why a checkpoint store refused or failed.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// The store failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The saved checkpoint is not one this crate can read.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The store's own failure, opaque to the caller.
    #[error("{0}")]
    Store(String),
}

/// Where a host keeps its checkpoints.
///
/// A key is opaque to the library: it is whatever the host names a run, and the
/// library never parses it. A save replaces what the key held, so the newest
/// checkpoint of a run is the one a load returns. Implementations are shared
/// (`Clone` over an `Arc` where they need to be) and their futures are `Send`
/// natively, and on browser wasm neither they nor the store need be.
///
/// ```
/// use rig::agent::AgentRun;
/// use rig::checkpoint::{Checkpoint, CheckpointStore, MemoryCheckpoints};
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let store = MemoryCheckpoints::new();
/// let checkpoint = Checkpoint::of(AgentRun::new("hi"), "the host stopped");
/// store.save("session-1", &checkpoint).await?;
/// assert_eq!(store.load("session-1").await?.as_ref().map(Checkpoint::reason), Some("the host stopped"));
/// # Ok(())
/// # }
/// ```
pub trait CheckpointStore: WasmCompatSend + WasmCompatSync {
    /// Replaces the checkpoint under `key`.
    fn save(
        &self,
        key: &str,
        checkpoint: &Checkpoint,
    ) -> impl Future<Output = Result<(), CheckpointError>> + WasmCompatSend;

    /// The checkpoint under `key`, or `None` when it holds none.
    fn load(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<Checkpoint>, CheckpointError>> + WasmCompatSend;

    /// Removes the checkpoint under `key`; a key holding none is not an error.
    fn remove(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<(), CheckpointError>> + WasmCompatSend;
}

/// Checkpoints held in memory, by key.
///
/// What a host uses before it has picked durable storage, and what tests need.
#[derive(Debug, Default)]
pub struct MemoryCheckpoints(Mutex<HashMap<String, String>>);

impl MemoryCheckpoints {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn checkpoints(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl CheckpointStore for MemoryCheckpoints {
    async fn save(&self, key: &str, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        let text = checkpoint.to_json()?;
        self.checkpoints().insert(key.to_string(), text);
        Ok(())
    }

    async fn load(&self, key: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        let Some(text) = self.checkpoints().get(key).cloned() else {
            return Ok(None);
        };
        Checkpoint::from_json(&text).map(Some)
    }

    async fn remove(&self, key: &str) -> Result<(), CheckpointError> {
        self.checkpoints().remove(key);
        Ok(())
    }
}

impl<T: CheckpointStore + ?Sized> CheckpointStore for std::sync::Arc<T> {
    async fn save(&self, key: &str, checkpoint: &Checkpoint) -> Result<(), CheckpointError> {
        (**self).save(key, checkpoint).await
    }

    async fn load(&self, key: &str) -> Result<Option<Checkpoint>, CheckpointError> {
        (**self).load(key).await
    }

    async fn remove(&self, key: &str) -> Result<(), CheckpointError> {
        (**self).remove(key).await
    }
}
