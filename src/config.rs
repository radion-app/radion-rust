//! Shared configuration for every Radion product surface.

/// Default base URL for the Radion REST API.
pub const DEFAULT_BASE_URL: &str = "https://api.radion.app";

/// Default endpoint for the Radion realtime (WebSocket) API.
pub const DEFAULT_WS_URL: &str = "wss://api.radion.app/ws";

/// Shared configuration for every Radion product surface.
///
/// `base_url` is reserved for forthcoming product surfaces; the realtime client
/// uses `ws_url`.
#[derive(Debug, Clone)]
pub struct RadionConfig {
    /// Radion API key, sent as the `X-API-Key` header on every request.
    pub api_key: String,
    /// Base URL for the REST API. Defaults to [`DEFAULT_BASE_URL`].
    pub base_url: String,
    /// Realtime endpoint. Defaults to [`DEFAULT_WS_URL`].
    pub ws_url: String,
}

impl RadionConfig {
    /// Build a config from an API key, using the default URLs.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            ws_url: DEFAULT_WS_URL.to_string(),
        }
    }
}
