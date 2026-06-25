//! Error types surfaced by the SDK.

/// Convenience alias for results returned by the SDK.
pub type Result<T, E = RadionError> = std::result::Result<T, E>;

/// Every error surfaced by the Radion SDK.
///
/// Mirrors the TypeScript / Python error hierarchy: connection-lifecycle
/// misuse, server-reported `error` frames, and transport / parse failures.
/// `Clone` so errors can be fanned out over the lifecycle event stream.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum RadionError {
    /// The SDK was used in a way the connection lifecycle forbids — for example
    /// subscribing after [`close`](crate::realtime::RealtimeClient::close), or
    /// building a client without an API key.
    #[error("{0}")]
    Connection(String),

    /// The server reported an `error` frame.
    #[error("{message}")]
    Server {
        /// Human-readable error message.
        message: String,
        /// Machine-readable error code, when present.
        code: Option<String>,
        /// Channel the error relates to, when present.
        channel: Option<String>,
        /// Subscription id the error relates to, when present.
        id: Option<String>,
    },

    /// The underlying WebSocket transport failed.
    #[error("transport error: {0}")]
    Transport(String),
}

impl RadionError {
    /// Construct a [`RadionError::Connection`].
    pub(crate) fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// Construct a [`RadionError::Transport`] from any displayable source.
    #[cfg(feature = "realtime")]
    pub(crate) fn transport(source: impl std::fmt::Display) -> Self {
        Self::Transport(source.to_string())
    }
}
