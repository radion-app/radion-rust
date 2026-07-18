//! The unified [`Radion`] client.

use crate::config::{DEFAULT_BASE_URL, DEFAULT_WS_URL, RadionConfig};
use crate::error::{RadionError, Result};

/// Unified entry point for the Radion platform SDK.
///
/// Holds shared configuration and exposes each product surface as a field.
/// Today that is [`realtime`](Radion::realtime); further surfaces attach here
/// as they ship — the builder shape stays stable so adding them is additive.
///
/// Build one with [`Radion::builder`].
#[derive(Debug)]
#[non_exhaustive]
pub struct Radion {
    /// Shared configuration (API key, base/realtime URLs).
    pub config: RadionConfig,

    /// Realtime (WebSocket) product surface.
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    pub realtime: crate::realtime::RealtimeClient,
}

impl Radion {
    /// Start building a client. Equivalent to [`RadionBuilder::default`].
    pub fn builder() -> RadionBuilder {
        RadionBuilder::default()
    }
}

/// Builder for [`Radion`].
///
/// ```no_run
/// # fn main() -> eyre::Result<()> {
/// let radion = radion_sdk::Radion::builder()
///     .api_key("sk_...")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct RadionBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    ws_url: Option<String>,
    #[cfg(feature = "realtime")]
    token_provider: Option<crate::realtime::TokenProvider>,
    #[cfg(feature = "realtime")]
    auth_in_query: bool,
}

impl RadionBuilder {
    /// Set the Radion API key (required). Sent as the `X-API-Key` header.
    #[must_use]
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Override the REST base URL. Defaults to [`DEFAULT_BASE_URL`].
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Override the realtime endpoint. Defaults to [`DEFAULT_WS_URL`].
    #[must_use]
    pub fn ws_url(mut self, ws_url: impl Into<String>) -> Self {
        self.ws_url = Some(ws_url.into());
        self
    }

    /// Set a static user JWT for the public-key (`pk_jwt_`) flow.
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token_provider = Some(crate::realtime::TokenProvider::from_static(token));
        self
    }

    /// Set a user JWT provider, called on every (re)connect for a fresh token.
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    #[must_use]
    pub fn token_provider(mut self, provider: crate::realtime::TokenProvider) -> Self {
        self.token_provider = Some(provider);
        self
    }

    /// Send credentials in the WS URL query string instead of headers.
    #[cfg(feature = "realtime")]
    #[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
    #[must_use]
    pub fn auth_in_query(mut self, enabled: bool) -> Self {
        self.auth_in_query = enabled;
        self
    }

    /// Build the [`Radion`] client.
    ///
    /// # Errors
    ///
    /// Returns [`RadionError::Connection`] if the API key is missing or empty.
    pub fn build(self) -> Result<Radion> {
        let api_key = self.api_key.unwrap_or_default();
        if api_key.is_empty() {
            return Err(RadionError::connection("api_key is required"));
        }
        let config = RadionConfig {
            api_key: api_key.clone(),
            base_url: self
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            ws_url: self.ws_url.unwrap_or_else(|| DEFAULT_WS_URL.to_string()),
        };

        #[cfg(feature = "realtime")]
        let realtime = {
            let mut options = crate::realtime::RealtimeOptions::new(api_key)
                .url(config.ws_url.clone())
                .auth_in_query(self.auth_in_query);
            if let Some(provider) = self.token_provider {
                options = options.token_provider(provider);
            }
            crate::realtime::RealtimeClient::new(options)
        };

        Ok(Radion {
            config,
            #[cfg(feature = "realtime")]
            realtime,
        })
    }
}

#[cfg(all(test, feature = "realtime"))]
mod builder_tests {
    use super::*;

    #[test]
    fn builder_accepts_token_and_query_flag() {
        let radion = Radion::builder()
            .api_key("pk_jwt_x")
            .token("jwt")
            .auth_in_query(true)
            .build()
            .expect("builds");
        let _ = radion;
    }
}
