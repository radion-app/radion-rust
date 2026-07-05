//! The realtime (WebSocket) client and its background connection manager.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderName, HeaderValue};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

use super::auth::{TokenProvider, build_auth_query_url};
use super::protocol::{
    ChannelEvent, InboundFrame, OutboundFrame, Subscription, parse_inbound_frame,
};
use super::reconnect::{ReconnectOptions, ReconnectPolicy};
use super::subscription::SubscriptionManager;
use crate::config::DEFAULT_WS_URL;
use crate::error::{RadionError, Result};

/// Buffered events retained per [`broadcast`] receiver before lagging.
const EVENT_BUFFER: usize = 1024;
/// Buffered lifecycle events retained per receiver before lagging.
const LIFECYCLE_BUFFER: usize = 64;

/// Options controlling heartbeat / stale-connection detection.
#[derive(Debug, Clone, Copy)]
pub struct HeartbeatOptions {
    /// Interval between client pings.
    pub interval: Duration,
    /// How long to wait for any inbound traffic after a ping before declaring
    /// the connection stale.
    pub timeout: Duration,
}

impl Default for HeartbeatOptions {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(15),
            timeout: Duration::from_secs(10),
        }
    }
}

/// Configuration for a [`RealtimeClient`].
#[derive(Debug, Clone)]
pub struct RealtimeOptions {
    /// Radion API key, sent as the `X-API-Key` header.
    pub api_key: String,
    /// WebSocket endpoint. Defaults to [`DEFAULT_WS_URL`].
    pub url: String,
    /// Reconnect policy, or `None` to disable auto-reconnect.
    pub reconnect: Option<ReconnectOptions>,
    /// Heartbeat policy, or `None` to disable heartbeats.
    pub heartbeat: Option<HeartbeatOptions>,
    /// User JWT provider for the public-key (`pk_jwt_`) flow. `None` = secret
    /// key. Resolved on every (re)connect.
    pub token_provider: Option<TokenProvider>,
    /// Send credentials in the URL query instead of headers. Defaults to
    /// `false`; enable for header-stripping proxies or gateways.
    pub auth_in_query: bool,
}

impl RealtimeOptions {
    /// Options for the given API key with default URL, reconnect, and heartbeat.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            url: DEFAULT_WS_URL.to_string(),
            reconnect: Some(ReconnectOptions::default()),
            heartbeat: Some(HeartbeatOptions::default()),
            token_provider: None,
            auth_in_query: false,
        }
    }

    /// Override the WebSocket endpoint.
    #[must_use]
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// Tune the reconnect policy.
    #[must_use]
    pub fn reconnect(mut self, options: ReconnectOptions) -> Self {
        self.reconnect = Some(options);
        self
    }

    /// Disable auto-reconnect.
    #[must_use]
    pub fn disable_reconnect(mut self) -> Self {
        self.reconnect = None;
        self
    }

    /// Tune the heartbeat policy.
    #[must_use]
    pub fn heartbeat(mut self, options: HeartbeatOptions) -> Self {
        self.heartbeat = Some(options);
        self
    }

    /// Disable heartbeats.
    #[must_use]
    pub fn disable_heartbeat(mut self) -> Self {
        self.heartbeat = None;
        self
    }

    /// Set a static user JWT for the public-key (`pk_jwt_`) flow.
    #[must_use]
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token_provider = Some(TokenProvider::from_static(token));
        self
    }

    /// Set a user JWT provider, called on every (re)connect for a fresh token.
    #[must_use]
    pub fn token_provider(mut self, provider: TokenProvider) -> Self {
        self.token_provider = Some(provider);
        self
    }

    /// Send credentials in the URL query string instead of headers.
    #[must_use]
    pub fn auth_in_query(mut self, enabled: bool) -> Self {
        self.auth_in_query = enabled;
        self
    }
}

/// A connection lifecycle event, delivered on [`RealtimeClient::lifecycle`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum LifecycleEvent {
    /// The connection opened (initial connect or successful reconnect).
    Open,
    /// The connection closed.
    Close {
        /// WebSocket close code.
        code: u16,
        /// Close reason, if any.
        reason: String,
    },
    /// A reconnect attempt was scheduled.
    Reconnect {
        /// Number of retries since the last successful connection.
        attempt: u32,
        /// Delay before the attempt.
        delay: Duration,
    },
    /// An error occurred: a server `error` frame, a transport failure, or a
    /// stale connection.
    Error(RadionError),
}

/// Commands sent from a [`RealtimeClient`] handle to its manager task.
enum Command {
    Subscribe(Subscription),
    Unsubscribe(String),
    Close { code: u16, reason: String },
}

/// Async WebSocket client for the Radion realtime API.
///
/// Owns the connection lifecycle: it transparently reconnects with exponential
/// backoff after unexpected drops, restores subscriptions on reconnect, and
/// fans inbound channel frames out to [`events`](Self::events) and
/// per-subscription streams.
///
/// Usually reached as [`Radion::realtime`](crate::Radion::realtime), but can be
/// constructed standalone with [`RealtimeClient::new`].
#[derive(Debug)]
pub struct RealtimeClient {
    options: RealtimeOptions,
    cmd_tx: mpsc::UnboundedSender<Command>,
    cmd_rx: Mutex<Option<mpsc::UnboundedReceiver<Command>>>,
    events_tx: broadcast::Sender<ChannelEvent>,
    lifecycle_tx: broadcast::Sender<LifecycleEvent>,
    connected: Arc<AtomicBool>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl RealtimeClient {
    /// Construct a standalone realtime client.
    pub fn new(options: RealtimeOptions) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (events_tx, _) = broadcast::channel(EVENT_BUFFER);
        let (lifecycle_tx, _) = broadcast::channel(LIFECYCLE_BUFFER);
        Self {
            options,
            cmd_tx,
            cmd_rx: Mutex::new(Some(cmd_rx)),
            events_tx,
            lifecycle_tx,
            connected: Arc::new(AtomicBool::new(false)),
            task: Mutex::new(None),
        }
    }

    /// Whether the underlying socket is currently open.
    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Open the connection.
    ///
    /// Resolves once the socket is established. Calling it again after a
    /// successful connect is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the first connection attempt fails.
    pub async fn connect(&self) -> Result<()> {
        if self.connected() {
            return Ok(());
        }
        let Some(cmd_rx) = self.cmd_rx.lock().expect("cmd_rx mutex poisoned").take() else {
            // Manager already started by a previous connect() call.
            return Ok(());
        };

        let (ready_tx, ready_rx) = oneshot::channel();
        let task = tokio::spawn(run(
            self.options.clone(),
            cmd_rx,
            self.events_tx.clone(),
            self.lifecycle_tx.clone(),
            Arc::clone(&self.connected),
            ready_tx,
        ));
        *self.task.lock().expect("task mutex poisoned") = Some(task);

        match ready_rx.await {
            Ok(result) => result,
            Err(_) => Err(RadionError::connection(
                "connection task ended before connecting",
            )),
        }
    }

    /// Subscribe to a channel, returning a stream of its events.
    ///
    /// The subscription is resent automatically after a reconnect. The returned
    /// stream yields only events for this subscription's `id`; use
    /// [`events`](Self::events) for the firehose across all subscriptions.
    ///
    /// # Errors
    ///
    /// Returns [`RadionError::Connection`] if the subscription is missing a
    /// filter its channel requires, or if the client has been closed.
    pub async fn subscribe(&self, subscription: Subscription) -> Result<ChannelEventStream> {
        subscription.validate()?;
        let id = subscription.id.clone();
        // Subscribe to the broadcast before sending the command so no event
        // delivered between the two is missed.
        let rx = self.events_tx.subscribe();
        self.cmd_tx
            .send(Command::Subscribe(subscription))
            .map_err(|_| RadionError::connection("client has been closed"))?;
        Ok(ChannelEventStream {
            inner: BroadcastStream::new(rx),
            filter_id: Some(id),
        })
    }

    /// Unsubscribe by subscription id.
    ///
    /// # Errors
    ///
    /// Returns [`RadionError::Connection`] if the client has been closed.
    pub async fn unsubscribe(&self, id: impl Into<String>) -> Result<()> {
        self.cmd_tx
            .send(Command::Unsubscribe(id.into()))
            .map_err(|_| RadionError::connection("client has been closed"))
    }

    /// Stream of every channel event across all subscriptions (the firehose).
    pub fn events(&self) -> ChannelEventStream {
        ChannelEventStream {
            inner: BroadcastStream::new(self.events_tx.subscribe()),
            filter_id: None,
        }
    }

    /// Stream of connection lifecycle events.
    pub fn lifecycle(&self) -> LifecycleStream {
        LifecycleStream {
            inner: BroadcastStream::new(self.lifecycle_tx.subscribe()),
        }
    }

    /// Gracefully close the connection and stop reconnect attempts.
    ///
    /// Waits for the manager task to finish. Subsequent calls are no-ops.
    pub async fn close(&self, code: u16, reason: impl Into<String>) {
        let _ = self.cmd_tx.send(Command::Close {
            code,
            reason: reason.into(),
        });
        let handle = self.task.lock().expect("task mutex poisoned").take();
        if let Some(handle) = handle {
            let _ = handle.await;
        }
    }
}

/// A stream of [`ChannelEvent`]s. Lagged events (slow consumer) are skipped.
pub struct ChannelEventStream {
    inner: BroadcastStream<ChannelEvent>,
    filter_id: Option<String>,
}

impl Stream for ChannelEventStream {
    type Item = ChannelEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(event))) => {
                    if self.filter_id.as_ref().is_none_or(|id| *id == event.id) {
                        return Poll::Ready(Some(event));
                    }
                }
                Poll::Ready(Some(Err(BroadcastStreamRecvError::Lagged(_)))) => {}
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// A stream of [`LifecycleEvent`]s. Lagged events are skipped.
pub struct LifecycleStream {
    inner: BroadcastStream<LifecycleEvent>,
}

impl Stream for LifecycleStream {
    type Item = LifecycleEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(event))) => return Poll::Ready(Some(event)),
                Poll::Ready(Some(Err(BroadcastStreamRecvError::Lagged(_)))) => {}
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Outcome of a single connected session.
enum SessionOutcome {
    /// The consumer requested a graceful shutdown.
    Shutdown { code: u16, reason: String },
    /// The connection dropped unexpectedly.
    Disconnected { code: u16, reason: String },
}

/// The background manager task: connect, run a session, reconnect on drop.
async fn run(
    options: RealtimeOptions,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    events_tx: broadcast::Sender<ChannelEvent>,
    lifecycle_tx: broadcast::Sender<LifecycleEvent>,
    connected: Arc<AtomicBool>,
    ready_tx: oneshot::Sender<Result<()>>,
) {
    let mut ready = Some(ready_tx);
    let mut policy = ReconnectPolicy::new(options.reconnect.unwrap_or_default());
    let mut subscriptions = SubscriptionManager::default();

    loop {
        match connect_ws(&options).await {
            Ok(ws) => {
                connected.store(true, Ordering::SeqCst);
                policy.reset();
                if let Some(tx) = ready.take() {
                    let _ = tx.send(Ok(()));
                }
                let _ = lifecycle_tx.send(LifecycleEvent::Open);
                #[cfg(feature = "tracing")]
                tracing::debug!(url = %options.url, "radion realtime connected");

                let outcome = session(
                    ws,
                    &options,
                    &mut cmd_rx,
                    &events_tx,
                    &lifecycle_tx,
                    &mut subscriptions,
                )
                .await;
                connected.store(false, Ordering::SeqCst);

                match outcome {
                    SessionOutcome::Shutdown { code, reason } => {
                        let _ = lifecycle_tx.send(LifecycleEvent::Close { code, reason });
                        return;
                    }
                    SessionOutcome::Disconnected { code, reason } => {
                        let _ = lifecycle_tx.send(LifecycleEvent::Close { code, reason });
                        if options.reconnect.is_none() {
                            return;
                        }
                    }
                }
            }
            Err(error) => {
                if let Some(tx) = ready.take() {
                    // First attempt failed: surface to connect() and stop.
                    let _ = tx.send(Err(error));
                    return;
                }
                let _ = lifecycle_tx.send(LifecycleEvent::Error(error));
                if options.reconnect.is_none() {
                    return;
                }
            }
        }

        // Back off before the next attempt; a Close command stops reconnecting.
        let delay = policy.next_delay();
        let _ = lifecycle_tx.send(LifecycleEvent::Reconnect {
            attempt: policy.attempts(),
            delay,
        });
        #[cfg(feature = "tracing")]
        tracing::debug!(
            ?delay,
            attempt = policy.attempts(),
            "radion realtime reconnecting"
        );

        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            cmd = cmd_rx.recv() => match cmd {
                Some(Command::Subscribe(subscription)) => {
                    subscriptions.add(subscription);
                }
                Some(Command::Unsubscribe(id)) => {
                    subscriptions.remove(&id);
                }
                Some(Command::Close { .. }) | None => return,
            },
        }
    }
}

/// Open a WebSocket connection, presenting the API key (and user JWT, if a token
/// provider is set) either as headers or in the URL query string.
async fn connect_ws(
    options: &RealtimeOptions,
) -> Result<
    impl Stream<Item = std::result::Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
> {
    let token = match &options.token_provider {
        Some(provider) => Some(provider.fetch().await?),
        None => None,
    };

    let (ws, _response) = if options.auth_in_query {
        let url = build_auth_query_url(&options.url, &options.api_key, token.as_deref());
        let request = url.into_client_request().map_err(RadionError::transport)?;
        tokio_tungstenite::connect_async(request)
            .await
            .map_err(RadionError::transport)?
    } else {
        let mut request = options
            .url
            .as_str()
            .into_client_request()
            .map_err(RadionError::transport)?;
        let api_key = HeaderValue::from_str(&options.api_key).map_err(RadionError::transport)?;
        request
            .headers_mut()
            .insert(HeaderName::from_static("x-api-key"), api_key);
        if let Some(token) = &token {
            let bearer = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(RadionError::transport)?;
            request
                .headers_mut()
                .insert(HeaderName::from_static("authorization"), bearer);
        }
        tokio_tungstenite::connect_async(request)
            .await
            .map_err(RadionError::transport)?
    };
    Ok(ws)
}

/// Run one connected session until it shuts down or drops.
async fn session<S>(
    mut ws: S,
    options: &RealtimeOptions,
    cmd_rx: &mut mpsc::UnboundedReceiver<Command>,
    events_tx: &broadcast::Sender<ChannelEvent>,
    lifecycle_tx: &broadcast::Sender<LifecycleEvent>,
    subscriptions: &mut SubscriptionManager,
) -> SessionOutcome
where
    S: Stream<Item = std::result::Result<Message, WsError>>
        + Sink<Message, Error = WsError>
        + Unpin,
{
    // Restore desired subscriptions after a (re)connect.
    let replay: Vec<_> = subscriptions
        .desired()
        .map(OutboundFrame::subscribe)
        .collect();
    for frame in replay {
        send(&mut ws, frame).await;
    }

    let mut ping = options
        .heartbeat
        .map(|hb| tokio::time::interval(hb.interval));
    let mut stale_deadline: Option<tokio::time::Instant> = None;

    loop {
        let stale = async {
            match stale_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await,
            }
        };

        tokio::select! {
            message = ws.next() => match message {
                Some(Ok(message)) => {
                    stale_deadline = None;
                    if let Some(outcome) = handle_message(&message, events_tx, lifecycle_tx) {
                        return outcome;
                    }
                }
                Some(Err(error)) => {
                    let _ = lifecycle_tx.send(LifecycleEvent::Error(RadionError::transport(error)));
                    return SessionOutcome::Disconnected { code: 1006, reason: String::new() };
                }
                None => return SessionOutcome::Disconnected { code: 1006, reason: String::new() },
            },
            command = cmd_rx.recv() => match command {
                Some(Command::Subscribe(subscription)) => {
                    if subscriptions.add(subscription.clone()) {
                        send(&mut ws, OutboundFrame::subscribe(&subscription)).await;
                    }
                }
                Some(Command::Unsubscribe(id)) => {
                    if subscriptions.remove(&id) {
                        send(&mut ws, OutboundFrame::Unsubscribe { id }).await;
                    }
                }
                Some(Command::Close { code, reason }) => {
                    let _ = ws.close().await;
                    return SessionOutcome::Shutdown { code, reason };
                }
                None => {
                    // Handle dropped: client gone, shut down quietly.
                    let _ = ws.close().await;
                    return SessionOutcome::Shutdown { code: 1000, reason: String::from("client dropped") };
                }
            },
            () = next_ping(&mut ping) => {
                send(&mut ws, OutboundFrame::Ping).await;
                if stale_deadline.is_none() {
                    if let Some(hb) = options.heartbeat {
                        stale_deadline = Some(tokio::time::Instant::now() + hb.timeout);
                    }
                }
            }
            () = stale => {
                let _ = lifecycle_tx.send(LifecycleEvent::Error(RadionError::connection("stale connection")));
                return SessionOutcome::Disconnected { code: 1006, reason: String::from("stale connection") };
            }
        }
    }
}

/// Await the next heartbeat tick, or never if heartbeats are disabled.
async fn next_ping(ping: &mut Option<tokio::time::Interval>) {
    match ping {
        Some(interval) => {
            interval.tick().await;
        }
        None => std::future::pending().await,
    }
}

/// Route an inbound message. Returns `Some` to end the session on a close frame.
fn handle_message(
    message: &Message,
    events_tx: &broadcast::Sender<ChannelEvent>,
    lifecycle_tx: &broadcast::Sender<LifecycleEvent>,
) -> Option<SessionOutcome> {
    match message {
        Message::Text(_) | Message::Binary(_) => {
            if let Ok(text) = message.to_text() {
                route_text(text, events_tx, lifecycle_tx);
            }
            None
        }
        Message::Close(frame) => {
            let (code, reason) = frame
                .as_ref()
                .map(|frame| (u16::from(frame.code), frame.reason.to_string()))
                .unwrap_or((1005, String::new()));
            Some(SessionOutcome::Disconnected { code, reason })
        }
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}

/// Parse a text frame and route it to the event / lifecycle broadcasts.
fn route_text(
    text: &str,
    events_tx: &broadcast::Sender<ChannelEvent>,
    lifecycle_tx: &broadcast::Sender<LifecycleEvent>,
) {
    let Some(frame) = parse_inbound_frame(text) else {
        return;
    };
    match frame {
        frame @ InboundFrame::Event { .. } => {
            if let Some(event) = frame.into_channel_event() {
                let _ = events_tx.send(event);
            }
        }
        InboundFrame::Error {
            message,
            code,
            id,
            channel,
            ..
        } => {
            let _ = lifecycle_tx.send(LifecycleEvent::Error(RadionError::Server {
                message,
                code,
                channel,
                id,
            }));
        }
        InboundFrame::Pong
        | InboundFrame::Subscribed { .. }
        | InboundFrame::Unsubscribed { .. } => {}
    }
}

/// Serialize and send an outbound frame, dropping it on a transport error.
async fn send<S>(ws: &mut S, frame: OutboundFrame)
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    if let Ok(text) = serde_json::to_string(&frame) {
        let _ = ws.send(Message::text(text)).await;
    }
}

#[cfg(test)]
mod auth_wiring_tests {
    use super::*;

    #[test]
    fn defaults_have_no_token_and_header_mode() {
        let options = RealtimeOptions::new("k");
        assert!(options.token_provider.is_none());
        assert!(!options.auth_in_query);
    }

    #[tokio::test]
    async fn static_token_builder_sets_provider() {
        let options = RealtimeOptions::new("k").token("jwt");
        let provider = options.token_provider.expect("provider set");
        assert_eq!(provider.fetch().await.unwrap(), "jwt");
    }

    #[test]
    fn auth_in_query_builder_flips_flag() {
        assert!(RealtimeOptions::new("k").auth_in_query(true).auth_in_query);
    }

    #[test]
    fn accepts_async_provider() {
        let _ = RealtimeOptions::new("k")
            .token_provider(TokenProvider::new(|| async { Ok("x".into()) }));
    }
}
