//! Auth helpers for the realtime client: token providers and query-string URLs.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Result;

type BoxFuture = Pin<Box<dyn Future<Output = Result<String>> + Send>>;
type ProviderFn = Arc<dyn Fn() -> BoxFuture + Send + Sync>;

/// Resolves the current user JWT for the public-key (`pk_jwt_`) flow. Cloneable
/// and shared across reconnect attempts; invoked once per connection attempt.
#[derive(Clone)]
pub struct TokenProvider(ProviderFn);

impl TokenProvider {
    /// A provider that always returns the same token.
    pub fn from_static(token: impl Into<String>) -> Self {
        let token = token.into();
        Self(Arc::new(move || {
            let token = token.clone();
            Box::pin(async move { Ok(token) })
        }))
    }

    /// A provider backed by an async closure, called on every (re)connect.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        Self(Arc::new(move || Box::pin(f())))
    }

    /// Resolve the current token.
    pub fn fetch(&self) -> BoxFuture {
        (self.0)()
    }
}

impl fmt::Debug for TokenProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenProvider(..)")
    }
}

/// Percent-encode a query value (encode everything that is not unreserved).
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Append `api-key` (and optional `token`) to a WS URL's query string,
/// preserving any existing query and percent-encoding the values.
pub fn build_auth_query_url(url: &str, api_key: &str, token: Option<&str>) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    let mut query = format!("api-key={}", encode(api_key));
    if let Some(token) = token {
        query.push_str(&format!("&token={}", encode(token)));
    }
    format!("{url}{separator}{query}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_appends_api_key() {
        assert_eq!(
            build_auth_query_url("wss://api.radion.app/ws", "k1", None),
            "wss://api.radion.app/ws?api-key=k1"
        );
    }

    #[test]
    fn build_url_appends_token() {
        assert_eq!(
            build_auth_query_url("wss://api.radion.app/ws", "k1", Some("jwt-2")),
            "wss://api.radion.app/ws?api-key=k1&token=jwt-2"
        );
    }

    #[test]
    fn build_url_preserves_existing_query() {
        assert_eq!(
            build_auth_query_url("wss://api.radion.app/ws?v=1", "k1", None),
            "wss://api.radion.app/ws?v=1&api-key=k1"
        );
    }

    #[test]
    fn build_url_encodes_values() {
        assert_eq!(
            build_auth_query_url("wss://api.radion.app/ws", "a b", Some("x/y=")),
            "wss://api.radion.app/ws?api-key=a%20b&token=x%2Fy%3D"
        );
    }

    #[tokio::test]
    async fn static_provider_returns_token() {
        let provider = TokenProvider::from_static("jwt");
        assert_eq!(provider.fetch().await.unwrap(), "jwt");
    }

    #[tokio::test]
    async fn closure_provider_returns_token() {
        let provider = TokenProvider::new(|| async { Ok("fresh".to_string()) });
        assert_eq!(provider.fetch().await.unwrap(), "fresh");
    }
}
