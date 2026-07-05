//! Realtime (WebSocket) product surface.
//!
//! Connect a [`RealtimeClient`], [`subscribe`](RealtimeClient::subscribe) to
//! channels, and consume the returned [`Stream`](futures_util::Stream) of typed
//! [`ChannelEvent`]s. The client transparently reconnects, restores
//! subscriptions, and heartbeats the connection.

mod channels;
mod client;
mod payloads;
mod protocol;
mod reconnect;
mod subscription;

pub use channels::{CHANNELS, CLOB_CHANNELS, Channel, ClobChannel, FilterKey, SubscribableChannel};
pub use client::{
    ChannelEventStream, HeartbeatOptions, LifecycleEvent, LifecycleStream, RealtimeClient,
    RealtimeOptions,
};
pub use payloads::{
    AccountsEventType, AccountsPayload, ClobBestBidAskPayload, ClobBookPayload,
    ClobLastTradePayload, ClobMidpointPayload, ClobPricesPayload, ClobTickSizePayload,
    CombosEventType, CombosPayload, FeesEventType, FeesPayload, Hex, Level, LifecycleEventType,
    LifecyclePayload, OracleEventType, OraclePayload, Payload, PositionsEventType,
    PositionsPayload, PriceChange, ResolutionEventType, ResolutionPayload, TradingEventType,
    TradingPayload, TransfersEventType, TransfersPayload,
};
pub use protocol::{ChannelEvent, ChannelFilters, Subscription};
pub use reconnect::ReconnectOptions;
