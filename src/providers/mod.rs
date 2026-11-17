//! Provider clients, their configurations, and model identifiers.
//!
//! A client, such as [`openai::OpenAI`], holds a provider's configuration on a
//! transport and builds the models it serves. A vendor that speaks another
//! provider's format has its own constructors, such as [`deepseek::from_env`].
//! For a model chosen from data, use [`Catalog::connect`](crate::catalog::Catalog::connect).
//!
//! ```no_run
//! use crate::providers::openai::{self, OpenAI};
//! let model = OpenAI::from_env()?.chat(openai::GPT_5_2);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
pub mod anthropic;
pub mod azure;
pub mod chatgpt;
pub mod cohere;
pub mod copilot;
pub mod deepseek;
pub mod doubleword;
pub mod gemini;
pub mod groq;
pub mod huggingface;
pub mod hyperbolic;
pub mod internal;
pub mod llamacpp;
pub mod minimax;
pub mod mira;
pub mod mistral;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod opencode;
pub mod openrouter;
pub mod perplexity;
pub mod registry;
pub mod together;
pub mod venice;
pub mod voyageai;
pub mod xai;
pub mod xiaomimimo;
pub mod zai;

#[cfg(feature = "bedrock")]
#[cfg_attr(docsrs, doc(cfg(feature = "bedrock")))]
pub mod bedrock;
#[cfg(feature = "candle")]
#[cfg_attr(docsrs, doc(cfg(feature = "candle")))]
pub mod candle;
#[cfg(feature = "fastembed")]
#[cfg_attr(docsrs, doc(cfg(feature = "fastembed")))]
pub mod fastembed;
#[cfg(feature = "gemini-grpc")]
#[cfg_attr(docsrs, doc(cfg(feature = "gemini-grpc")))]
pub mod gemini_grpc;
#[cfg(all(feature = "rmcp", not(target_family = "wasm")))]
#[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
pub mod rmcp;
#[cfg(feature = "typesafeai")]
#[cfg_attr(docsrs, doc(cfg(feature = "typesafeai")))]
pub mod typesafeai;
#[cfg(feature = "vertexai")]
#[cfg_attr(docsrs, doc(cfg(feature = "vertexai")))]
pub mod vertexai;
