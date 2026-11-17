//! The bundled reqwest HTTP transport.
//!
//! [`ReqwestClient`] is one [`HttpClientExt`](crate::http_client::HttpClientExt)
//! implementation: [`ReqwestClient::default()`] is the process-wide client,
//! built once on first use and shared by every clone, so every provider client
//! made with it shares one connection pool.
//!
//! ```no_run
//! use crate::http_client::reqwest_client::ReqwestClient;
//!
//! let shared = ReqwestClient::default();
//! # let _ = shared;
//! ```

mod client;
pub use client::*;

mod middleware;
pub use middleware::*;
pub use reqwest;

#[cfg(test)]
mod tests;

#[cfg(all(test, not(target_family = "wasm")))]
mod error_body;
