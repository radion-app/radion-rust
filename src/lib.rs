//! Official, async-first, fully-typed Rust SDK for the [Radion](https://radion.app) platform.
//!
//! One [`Radion`] client, one API key. Today the SDK exposes the realtime
//! (WebSocket) product surface; further surfaces attach to the same client as
//! they ship, so adopting them is purely additive.
//!
//! # Example
//!
//! ```no_run
//! use futures_util::StreamExt;
//! use radion_sdk::{Radion, realtime::{Channel, Payload, Subscription}};
//!
//! # async fn run() -> anyhow::Result<()> {
//! let radion = Radion::builder().api_key("rk_...").build()?;
//! radion.realtime.connect().await?;
//!
//! let mut trades = radion.realtime.subscribe(Subscription::new("trades", Channel::Trades)).await?;
//! while let Some(event) = trades.next().await {
//!     if let Payload::Trades(trade) = event.data {
//!         println!("{} {:?}", event.id, trade);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - `realtime` *(default)* — the WebSocket product surface.
//! - `rustls` *(default)* — rustls TLS backend (no system OpenSSL dependency).
//! - `native-tls` — use the platform native TLS backend instead.
//! - `tracing` — emit [`tracing`](https://docs.rs/tracing) spans/events.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod client;
mod config;
mod error;

pub use client::{Radion, RadionBuilder};
pub use config::{DEFAULT_BASE_URL, DEFAULT_WS_URL, RadionConfig};
pub use error::{RadionError, Result};

#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub mod realtime;
