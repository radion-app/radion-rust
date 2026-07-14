//! Realtime (WebSocket) product surface.
//!
//! Connect a [`RealtimeClient`], [`subscribe`](RealtimeClient::subscribe) to
//! channels, and consume the returned [`Stream`](futures_util::Stream) of typed
//! [`ChannelEvent`]s. The client transparently reconnects, restores
//! subscriptions, and heartbeats the connection.
//!
//! The channel, payload, and frame types here also back the
//! [`webhooks`](crate::webhooks) surface — webhook deliveries carry the same
//! event frame — so they are available with either the `realtime` or the
//! `webhooks` feature.

#[cfg(feature = "realtime")]
mod auth;
mod channels;
#[cfg(feature = "realtime")]
mod client;
mod payloads;
pub(crate) mod protocol;
#[cfg(feature = "realtime")]
mod reconnect;
#[cfg(feature = "realtime")]
mod subscription;

#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub use auth::{TokenProvider, build_auth_query_url};
pub use channels::{CHANNELS, CLOB_CHANNELS, Channel, ClobChannel, FilterKey, SubscribableChannel};
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub use client::{
    ChannelEventStream, HeartbeatOptions, LifecycleEvent, LifecycleStream, RealtimeClient,
    RealtimeOptions,
};
pub use payloads::{
    AccountsEventType, AccountsPayload, ClobBestBidAskPayload, ClobBookPayload,
    ClobLastTradePayload, ClobMidpointPayload, ClobPricesPayload, ClobTickSizePayload,
    CombosEventType, CombosPayload, FeesEventType, FeesPayload, Hex, Level, LifecycleEventType,
    LifecyclePayload, MempoolCall, MempoolOrder, MempoolPayload, OracleEventType, OraclePayload,
    OrderSide, Payload, PositionsEventType, PositionsPayload, PriceChange, ResolutionEventType,
    ResolutionPayload, TradingEventType, TradingPayload, TransfersEventType, TransfersPayload,
};
pub use protocol::{ChannelEvent, ChannelFilters, Subscription};
#[cfg(feature = "realtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "realtime")))]
pub use reconnect::ReconnectOptions;
