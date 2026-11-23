//! Every plug-in seam of the library implemented from scratch.
//!
//! Nothing below names a vendor or a database. A host supplies files, a log
//! store, a context policy, a tool registry, an embedding, a vector index, an
//! HTTP client, a websocket client and a recorder, and the library runs on
//! those alone. `cargo check -p custom_integrations` is the compile test.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io;
use std::sync::{Arc, Mutex};

use rig::completion::Message;
use rig::effect::{Answered, EffectId, EffectKind, HandlerDescriptor, HandlerKey};
use rig::embeddings::Embedding;
use rig::embeddings::embed::{Embed, EmbedError, TextEmbedder};
use rig::error::ErrorReport;
use rig::http_client::{
    BoxedStream, Bytes, Error as HttpError, HttpClientExt, LazyBody, MultipartForm, NoBody,
    Request, Response, StreamingResponse,
};
use rig::memory::{Files, LogWriteError, Recent, Rewrite, Store};
use rig::message::{EmptyToolName, Origin, ToolName};
use rig::serve::{DispatchContext, Recorder};
use rig::streaming::{Item, StreamEvent};
use rig::tool::{
    ContextValue, DynamicTool, Tool, ToolContext, ToolExecutionError, ToolOutput,
    ToolResultContext, ToolSource,
};
use rig::vector_store::request::Filter;
use rig::vector_store::{
    InsertDocuments, VectorSearchIdResult, VectorSearchRequest, VectorSearchResult,
    VectorStoreError, VectorStoreIndex,
};
use rig::wasm_compat::{WasmBoxedFuture, WasmCompatSend};
use rig::ws_client::{
    BoxedWebSocketConnection, CloseFrame, ConnectOptions, Frame, WebSocketClientExt,
    WebSocketConnection,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Registry::new()?;
    seams_compose(&registry);
    Ok(())
}

/// Nothing here names where the data lives: each seam stands on its own.
fn seams_compose(registry: &Registry) {
    let files = InMemoryFiles::default();
    let store = LogStore {
        files: InMemoryFiles::default(),
    };
    assert!(files.files().is_ok());
    assert!(store.logs().is_ok());
    assert_eq!(registry.tools().len(), 1);
    assert_eq!(LastThree.start(&[]), 0);
}

// ---------------------------------------------------------------------------
// Session storage: files, and the log store built on them.
// ---------------------------------------------------------------------------

/// Files kept in a map, keyed by relative name.
#[derive(Default)]
struct InMemoryFiles {
    entries: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryFiles {
    fn entries(&self) -> io::Result<std::sync::MutexGuard<'_, BTreeMap<String, Vec<u8>>>> {
        self.entries
            .lock()
            .map_err(|_| io::Error::other("the file map was poisoned"))
    }
}

impl Files for InMemoryFiles {
    fn write_file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.entries()?.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }

    fn append_file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.entries()?
            .entry(name.to_string())
            .or_default()
            .extend_from_slice(bytes);
        Ok(())
    }

    fn read_file(&self, name: &str) -> io::Result<Vec<u8>> {
        self.entries()?
            .get(name)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_string()))
    }

    fn remove_file(&self, name: &str) -> io::Result<()> {
        self.entries()?.remove(name);
        Ok(())
    }

    fn files(&self) -> io::Result<Vec<String>> {
        Ok(self.entries()?.keys().cloned().collect())
    }
}

/// The append-only log store, over any [`Files`].
struct LogStore<F> {
    files: F,
}

impl<F: Files> Files for LogStore<F> {
    fn write_file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.files.write_file(name, bytes)
    }

    fn append_file(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.files.append_file(name, bytes)
    }

    fn read_file(&self, name: &str) -> io::Result<Vec<u8>> {
        self.files.read_file(name)
    }

    fn remove_file(&self, name: &str) -> io::Result<()> {
        self.files.remove_file(name)
    }

    fn files(&self) -> io::Result<Vec<String>> {
        self.files.files()
    }
}

impl<F: Files> Store for LogStore<F> {
    fn logs(&self) -> io::Result<Vec<String>> {
        self.files.files()
    }

    fn read_log(&self, name: &str) -> io::Result<Vec<u8>> {
        self.files.read_file(name)
    }

    /// A log is the file of the same name. `len` is the length the caller
    /// read, so a mismatch is [`LogWriteError::Changed`] and nothing is
    /// written.
    fn append_log(&self, name: &str, len: u64, bytes: &[u8]) -> Result<(), LogWriteError> {
        let current = self.files.read_file(name).unwrap_or_default();
        if current.len() as u64 != len {
            return Err(LogWriteError::Changed);
        }
        self.files.append_file(name, bytes)?;
        Ok(())
    }

    fn replace_log(&self, name: &str, len: u64, bytes: &[u8]) -> Result<(), LogWriteError> {
        let current = self.files.read_file(name).unwrap_or_default();
        if current.len() as u64 != len {
            return Err(LogWriteError::Changed);
        }
        self.files.write_file(name, bytes)?;
        Ok(())
    }

    fn destroy(&self) -> io::Result<()> {
        for name in self.files.files()? {
            self.files.remove_file(&name)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Context policy: a recent tail, and a rewrite that changes nothing.
// ---------------------------------------------------------------------------

/// Keeps the last three messages as the tail that is never compacted.
struct LastThree;

impl Recent for LastThree {
    fn start(&self, messages: &[Arc<Message>]) -> usize {
        messages.len().saturating_sub(3)
    }
}

/// Every hook keeps its default, which changes nothing.
struct Unchanged;

impl Rewrite for Unchanged {}

// ---------------------------------------------------------------------------
// Tools: one typed tool, one runtime-defined tool, and a registry.
// ---------------------------------------------------------------------------

struct Upper;

impl Tool for Upper {
    const NAME: &'static str = "upper";
    type Args = String;
    type Output = String;
    type Error = std::convert::Infallible;

    fn description(&self) -> String {
        "Uppercases its input".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({"type": "string"})
    }

    async fn call(
        &self,
        _context: &mut ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(args.to_uppercase())
    }
}

/// A registry that offers the same tool on every call.
struct Registry {
    name: ToolName,
}

impl Registry {
    fn new() -> Result<Self, EmptyToolName> {
        Ok(Self {
            name: ToolName::new("upper")?,
        })
    }
}

impl ToolSource for Registry {
    fn tools(&self) -> Vec<DynamicTool> {
        vec![DynamicTool::new(
            self.name.clone(),
            "Uppercases its input",
            serde_json::json!({"type": "string"}),
            |args| {
                Box::pin(async move {
                    let text = args
                        .as_str()
                        .ok_or_else(|| ToolExecutionError::invalid_args("the input is not text"))?;
                    Ok(ToolOutput::text(text.to_uppercase()))
                })
            },
        )]
    }
}

/// A value the host publishes into a call's context, and reads back from it.
#[derive(serde::Serialize, serde::Deserialize)]
struct Tenant(String);

impl ContextValue for Tenant {
    const KEY: &'static str = "tenant";
}

// ---------------------------------------------------------------------------
// Embeddings: one document, embedded from its text.
// ---------------------------------------------------------------------------

struct Note(String);

impl Embed for Note {
    fn embed(&self, embedder: &mut TextEmbedder) -> Result<(), EmbedError> {
        embedder.embed(self.0.clone());
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Vector index and insertion, over a map in memory.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Index {
    documents: Mutex<Vec<(String, Vec<f64>)>>,
}

impl Index {
    fn documents(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Vec<(String, Vec<f64>)>>, VectorStoreError> {
        self.documents
            .lock()
            .map_err(|_| VectorStoreError::DatastoreError("the index was poisoned".into()))
    }
}

impl VectorStoreIndex for Index {
    type Filter = Filter<serde_json::Value>;

    async fn top_n<T: serde::de::DeserializeOwned>(
        &self,
        request: VectorSearchRequest,
    ) -> Result<Vec<VectorSearchResult<T>>, VectorStoreError> {
        let documents = self.documents()?;
        documents
            .iter()
            .take(request.samples() as usize)
            .map(|(id, _)| {
                Ok(VectorSearchResult {
                    id: id.clone(),
                    score: 1.0,
                    document: serde_json::from_value(serde_json::json!(id))?,
                })
            })
            .collect()
    }

    async fn top_n_ids(
        &self,
        request: VectorSearchRequest,
    ) -> Result<Vec<VectorSearchIdResult>, VectorStoreError> {
        let documents = self.documents()?;
        Ok(documents
            .iter()
            .take(request.samples() as usize)
            .map(|(id, _)| VectorSearchIdResult {
                id: id.clone(),
                score: 1.0,
            })
            .collect())
    }
}

impl InsertDocuments for Index {
    async fn insert_documents<Doc: serde::Serialize + Embed + WasmCompatSend>(
        &self,
        documents: Vec<(Doc, Vec<Embedding>)>,
    ) -> Result<(), VectorStoreError> {
        let mut stored = self.documents()?;
        for (id, (_document, embeddings)) in documents.into_iter().enumerate() {
            let vector = embeddings
                .first()
                .map(|embedding| embedding.vec.clone())
                .unwrap_or_default();
            stored.push((id.to_string(), vector));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Transports: an HTTP client that answers from a canned body, and a websocket
// client that opens nothing.
// ---------------------------------------------------------------------------

struct Canned;

impl HttpClientExt for Canned {
    fn send<T, U>(
        &self,
        _request: Request<T>,
    ) -> impl std::future::Future<Output = Result<Response<LazyBody<U>>, HttpError>>
    + WasmCompatSend
    + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let body: LazyBody<U> = Box::pin(async { Ok(U::from(Bytes::from_static(b"{}"))) });
        let response = Response::builder().status(200).body(body);
        std::future::ready(response.map_err(HttpError::from))
    }

    fn send_multipart<U>(
        &self,
        _request: Request<MultipartForm>,
    ) -> impl std::future::Future<Output = Result<Response<LazyBody<U>>, HttpError>>
    + WasmCompatSend
    + 'static
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        let body: LazyBody<U> = Box::pin(async { Ok(U::from(Bytes::new())) });
        let response = Response::builder().status(200).body(body);
        std::future::ready(response.map_err(HttpError::from))
    }

    fn send_streaming<T>(
        &self,
        _request: Request<T>,
    ) -> impl std::future::Future<Output = Result<StreamingResponse, HttpError>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        let body: BoxedStream = Box::pin(futures::stream::empty());
        let response = Response::builder().status(200).body(body);
        std::future::ready(response.map_err(HttpError::from))
    }
}

#[derive(Clone)]
struct Refusing;

impl WebSocketClientExt for Refusing {
    async fn connect(
        &self,
        _request: Request<NoBody>,
        _options: ConnectOptions,
    ) -> Result<BoxedWebSocketConnection, HttpError> {
        Err(HttpError::Permanent(Box::new(io::Error::other(
            "this host opens no sockets",
        ))))
    }
}

/// An open session that never yields a frame.
struct Silent;

impl WebSocketConnection for Silent {
    fn send(&mut self, _frame: Frame) -> WasmBoxedFuture<'_, Result<(), HttpError>> {
        Box::pin(async { Ok(()) })
    }

    fn recv(&mut self) -> WasmBoxedFuture<'_, Result<Option<Frame>, HttpError>> {
        Box::pin(async { Ok(None) })
    }

    fn close(&mut self, _frame: Option<CloseFrame>) -> WasmBoxedFuture<'_, Result<(), HttpError>> {
        Box::pin(async { Ok(()) })
    }
}

// ---------------------------------------------------------------------------
// Effect recording: a recorder that keeps nothing.
// ---------------------------------------------------------------------------

struct Discarding;

impl Recorder for Discarding {
    fn handlers(&self, _handlers: Vec<HandlerDescriptor>) {}

    fn begin(&self, _id: EffectId, _key: HandlerKey, _kind: EffectKind, _origin: DispatchContext) {}

    fn discard(&self, _id: EffectId) {}

    fn patch(&self, _id: EffectId, _kind: EffectKind) {}

    fn keep_events(&self) -> bool {
        false
    }

    fn event(&self, _id: EffectId, _item: &Item<StreamEvent>) {}

    fn origin(&self, _id: EffectId, _origin: &Origin) {}

    fn tool_output(&self, _id: EffectId, _output: ToolResultContext) {}

    fn resolve(&self, _id: EffectId, _outcome: Result<Answered, ErrorReport>) {}
}
