//! Wire protocol: subscriptions, filters, and inbound/outbound frames.

use serde::{Deserialize, Serialize};

use super::channels::{Channel, ClobChannel, FilterKey, SubscribableChannel, join_filter_keys};
use super::payloads::Payload;
use crate::error::{RadionError, Result};

/// Server-side filters narrowing the events delivered on a channel.
///
/// Some channels require a filter (for example `wallets` needs `wallets`,
/// `markets` needs `market_ids` or `token_ids`); see the channel docs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChannelFilters {
    /// Wallet addresses to match (required by `wallets`, optional on `trading`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallets: Option<Vec<String>>,
    /// Condition / market ids to match (required by `markets`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_ids: Option<Vec<String>>,
    /// ERC-1155 token ids to match (required by `markets` and every `clob.*`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_ids: Option<Vec<String>>,
    /// Minimum trade notional in USD (optional on `trading`).
    ///
    /// On the confirmed feed this is the actual filled USD; on the pending feed
    /// it is the intended fill notional (`call.notional_usd`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_usd: Option<f64>,
}

impl ChannelFilters {
    /// Whether the given filter key carries a value.
    fn has(&self, key: FilterKey) -> bool {
        match key {
            FilterKey::Wallets => self.wallets.as_ref().is_some_and(|v| !v.is_empty()),
            FilterKey::MarketIds => self.market_ids.as_ref().is_some_and(|v| !v.is_empty()),
            FilterKey::TokenIds => self.token_ids.as_ref().is_some_and(|v| !v.is_empty()),
            FilterKey::MinUsd => self.min_usd.is_some(),
        }
    }
}

/// A single channel subscription.
///
/// `id` is a client-defined string echoed back on acknowledgements and on every
/// event frame, so multiple subscriptions to the same channel can be told
/// apart. `channel` is a topic channel or a CLOB channel.
///
/// `confirmed` picks the feed: `true` (the default) streams confirmed on-chain
/// events; `false` streams pending mempool transactions before block inclusion.
/// It has no effect on CLOB channels, which have no pending feed.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    /// Client-defined id, echoed back on confirmations and event frames.
    pub id: String,
    /// Channel name: a topic channel or a `clob.`-prefixed channel.
    pub channel: SubscribableChannel,
    /// Which feed to stream: `true` = confirmed (default), `false` = pending.
    pub confirmed: bool,
    /// Optional server-side filters.
    pub filters: Option<ChannelFilters>,
}

impl Subscription {
    /// Create a subscription with no filters, on the confirmed feed.
    pub fn new(id: impl Into<String>, channel: impl Into<SubscribableChannel>) -> Self {
        Self {
            id: id.into(),
            channel: channel.into(),
            confirmed: true,
            filters: None,
        }
    }

    /// Attach server-side filters.
    #[must_use]
    pub fn with_filters(mut self, filters: ChannelFilters) -> Self {
        self.filters = Some(filters);
        self
    }

    /// Choose the feed: `true` = confirmed on-chain events, `false` = pending
    /// mempool transactions. Ignored for CLOB channels.
    #[must_use]
    pub fn confirmed(mut self, confirmed: bool) -> Self {
        self.confirmed = confirmed;
        self
    }

    /// Stream the pending (mempool) feed instead of confirmed events. Shorthand
    /// for [`confirmed(false)`](Self::confirmed).
    #[must_use]
    pub fn pending(self) -> Self {
        self.confirmed(false)
    }

    /// Validate that this subscription carries the filters its channel requires.
    ///
    /// # Errors
    ///
    /// Returns [`RadionError::Connection`] describing the first violation. The
    /// requirement is the same for the confirmed and pending feeds; every CLOB
    /// channel requires a `token_ids` filter.
    pub fn validate(&self) -> Result<()> {
        let Some(requirement) = self.channel.filter_requirement() else {
            return Ok(());
        };
        let satisfied = requirement
            .required_any_of
            .iter()
            .any(|key| self.filters.as_ref().is_some_and(|f| f.has(*key)));
        if satisfied {
            return Ok(());
        }
        Err(RadionError::connection(format!(
            "channel \"{}\" requires a {} filter",
            self.channel,
            join_filter_keys(requirement.required_any_of),
        )))
    }
}

/// A frame sent from the client to the Radion server.
#[cfg(feature = "realtime")]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum OutboundFrame {
    Subscribe {
        id: String,
        channel: SubscribableChannel,
        #[serde(skip_serializing_if = "Option::is_none")]
        confirmed: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        filters: Option<ChannelFilters>,
    },
    Unsubscribe {
        id: String,
    },
    Ping,
}

#[cfg(feature = "realtime")]
impl OutboundFrame {
    pub(crate) fn subscribe(subscription: &Subscription) -> Self {
        let confirmed = subscription.channel.topic().map(|_| subscription.confirmed);
        Self::Subscribe {
            id: subscription.id.clone(),
            channel: subscription.channel,
            confirmed,
            filters: subscription.filters.clone(),
        }
    }
}

/// A data event delivered on a subscribed channel.
///
/// `id` identifies the subscription it belongs to; `channel` is the bare channel
/// name (same for both feeds). `confirmed` says which feed it came from: `true` =
/// confirmed on-chain event, `false` = pending mempool transaction. Route by
/// `id`, not by the channel name. `seq` and `sent_at_ms` describe the envelope:
/// gap detection and server send time. `data` is the typed payload — match on it
/// to handle a specific event shape.
#[derive(Debug, Clone)]
pub struct ChannelEvent {
    /// Subscription id this event belongs to.
    pub id: String,
    /// Bare channel name.
    pub channel: String,
    /// Which feed this event came from: `true` = confirmed, `false` = pending.
    pub confirmed: bool,
    /// Per-connection monotonic counter, starting at 0 and incremented by one
    /// for every event frame across all subscriptions. A jump means frames
    /// were dropped.
    pub seq: u64,
    /// Unix milliseconds when the server sent the frame. Server→client latency
    /// is your receive time (ms) minus `sent_at_ms`.
    pub sent_at_ms: u64,
    /// Typed payload.
    pub data: Payload,
}

/// A frame received from the server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum InboundFrame {
    Event {
        id: String,
        channel: String,
        #[serde(default = "confirmed_default")]
        confirmed: bool,
        seq: u64,
        sent_at_ms: u64,
        data: serde_json::Value,
    },
    Subscribed {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        channel: Option<String>,
        #[allow(dead_code)]
        confirmed: Option<bool>,
    },
    Unsubscribed {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        channel: Option<String>,
    },
    #[cfg_attr(not(feature = "realtime"), allow(dead_code))]
    Warning {
        code: String,
        id: Option<String>,
        message: String,
    },
    Pong,
    #[cfg_attr(not(feature = "realtime"), allow(dead_code))]
    Error {
        message: String,
        code: Option<String>,
        id: Option<String>,
        channel: Option<String>,
        #[allow(dead_code)]
        skipped: Option<u64>,
    },
}

/// Default for a missing `confirmed` envelope field: confirmed feed.
fn confirmed_default() -> bool {
    true
}

/// Parse and validate a raw text frame into a typed [`InboundFrame`].
///
/// Returns `None` when the payload is not valid JSON or does not match a known
/// frame envelope, so callers can drop malformed frames without erroring.
pub(crate) fn parse_inbound_frame(raw: &str) -> Option<InboundFrame> {
    serde_json::from_str(raw).ok()
}

impl InboundFrame {
    /// Convert an `event` frame into a typed [`ChannelEvent`], decoding `data`
    /// against the channel and the envelope's `confirmed` flag. Returns `None`
    /// for non-event frames.
    pub(crate) fn into_channel_event(self) -> Option<ChannelEvent> {
        let Self::Event {
            id,
            channel,
            confirmed,
            seq,
            sent_at_ms,
            data,
        } = self
        else {
            return None;
        };
        let payload = if let Ok(clob) = channel.parse::<ClobChannel>() {
            Payload::from_clob_channel(clob, data)
        } else if confirmed {
            match channel.parse::<Channel>() {
                Ok(topic) => Payload::from_channel(topic, data),
                Err(_) => Payload::Other(data),
            }
        } else {
            Payload::from_pending(data)
        };
        Some(ChannelEvent {
            id,
            channel,
            confirmed,
            seq,
            sent_at_ms,
            data: payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::channels::ClobChannel;
    use crate::realtime::payloads::{OrderSide, Payload, TradingEventType};

    #[test]
    fn validates_required_filters() {
        // `wallets` requires a wallets filter.
        assert!(Subscription::new("w", Channel::Wallets).validate().is_err());
        let ok = Subscription::new("w", Channel::Wallets).with_filters(ChannelFilters {
            wallets: Some(vec!["0x1".into()]),
            ..Default::default()
        });
        assert!(ok.validate().is_ok());

        // `markets` accepts either market_ids or token_ids.
        let markets = Subscription::new("m", Channel::Markets).with_filters(ChannelFilters {
            token_ids: Some(vec!["1".into()]),
            ..Default::default()
        });
        assert!(markets.validate().is_ok());

        // `trading` requires nothing.
        assert!(Subscription::new("t", Channel::Trading).validate().is_ok());

        // Every clob channel requires token_ids.
        assert!(
            Subscription::new("b", ClobChannel::Book)
                .validate()
                .is_err()
        );
        let clob = Subscription::new("b", ClobChannel::Book).with_filters(ChannelFilters {
            token_ids: Some(vec!["1".into()]),
            ..Default::default()
        });
        assert!(clob.validate().is_ok());
    }

    #[test]
    fn parses_and_types_clob_event_frames() {
        let raw = r#"{"type":"event","id":"mid","channel":"clob.midpoint","seq":42,"sent_at_ms":1721818200123,"data":{"asset_id":"7","market":"0xm","midpoint":0.5,"timestamp":1}}"#;
        let event = parse_inbound_frame(raw)
            .expect("valid frame")
            .into_channel_event()
            .expect("event");
        assert_eq!(event.channel, "clob.midpoint");
        assert_eq!(event.seq, 42);
        assert_eq!(event.sent_at_ms, 1_721_818_200_123);
        match event.data {
            Payload::ClobMidpoint(mid) => {
                assert_eq!(mid.asset_id, "7");
                assert_eq!(mid.midpoint, 0.5);
            }
            other => panic!("expected clob midpoint payload, got {other:?}"),
        }
    }

    #[test]
    fn serializes_outbound_frames() {
        let ping = serde_json::to_string(&OutboundFrame::Ping).unwrap();
        assert_eq!(ping, r#"{"action":"ping"}"#);

        let unsub = serde_json::to_string(&OutboundFrame::Unsubscribe { id: "x".into() }).unwrap();
        assert_eq!(unsub, r#"{"action":"unsubscribe","id":"x"}"#);

        let sub = OutboundFrame::subscribe(&Subscription::new("trading", Channel::Trading));
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&sub).unwrap()).unwrap();
        assert_eq!(json["action"], "subscribe");
        assert_eq!(json["channel"], "trading");
        assert_eq!(json["confirmed"], true);
        // No filters key when none are set.
        assert!(json.get("filters").is_none());

        let pending = OutboundFrame::subscribe(&Subscription::new("t", Channel::Trading).pending());
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&pending).unwrap()).unwrap();
        assert_eq!(json["channel"], "trading");
        assert_eq!(json["confirmed"], false);

        let clob = OutboundFrame::subscribe(
            &Subscription::new("b", ClobChannel::Book).with_filters(ChannelFilters {
                token_ids: Some(vec!["1".into()]),
                ..Default::default()
            }),
        );
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&clob).unwrap()).unwrap();
        assert!(json.get("confirmed").is_none());
    }

    #[test]
    fn parses_and_types_confirmed_event_frames() {
        let raw = r#"{"type":"event","id":"t","channel":"trading","confirmed":true,"seq":42,"sent_at_ms":1721818200123,"data":{"type":"order_filled_v2","side":1,"tokenId":"0xabc"}}"#;
        let frame = parse_inbound_frame(raw).expect("valid frame");
        let event = frame.into_channel_event().expect("event");
        assert_eq!(event.id, "t");
        assert!(event.confirmed);
        assert_eq!(event.seq, 42);
        assert_eq!(event.sent_at_ms, 1_721_818_200_123);
        match event.data {
            Payload::Trading(trade) => {
                assert_eq!(trade.kind, TradingEventType::OrderFilledV2);
                assert_eq!(trade.side, Some(1));
                assert_eq!(trade.token_id.as_deref(), Some("0xabc"));
            }
            other => panic!("expected trading payload, got {other:?}"),
        }
    }

    #[test]
    fn parses_and_types_pending_event_frames() {
        let raw = r#"{"type":"event","id":"t","channel":"trading","confirmed":false,"seq":42,"sent_at_ms":1721818200123,"data":{"seen_at_ms":1782027489000,"transaction_hash":"0xhash","from":"0xfrom","to":"0xto","contract_kinds":["exchange"],"method_selector":"0xabcdef12","input":"0xdead","value":"0","call":{"method":"fillOrder","market_ids":["0xm"],"token_ids":["7"],"wallets":["0xw"],"notional_usd":192.5,"orders":[{"maker":"0xa","taker":null,"token_id":"7","side":"buy","maker_amount":"100","taker_amount":"50"}]}}}"#;
        let event = parse_inbound_frame(raw)
            .expect("valid frame")
            .into_channel_event()
            .expect("event");
        assert_eq!(event.channel, "trading");
        assert!(!event.confirmed);
        match event.data {
            Payload::Mempool(tx) => {
                assert_eq!(tx.transaction_hash, "0xhash");
                assert_eq!(tx.from, "0xfrom");
                let call = tx.call.expect("call present");
                assert_eq!(call.method, "fillOrder");
                assert_eq!(call.notional_usd, Some(192.5));
                assert_eq!(call.orders.len(), 1);
                assert_eq!(call.orders[0].side, OrderSide::Buy);
                assert!(call.orders[0].taker.is_none());
            }
            other => panic!("expected mempool payload, got {other:?}"),
        }
    }

    #[test]
    fn missing_confirmed_defaults_to_confirmed_feed() {
        let raw = r#"{"type":"event","id":"t","channel":"trading","seq":42,"sent_at_ms":1721818200123,"data":{"type":"order_cancelled"}}"#;
        let event = parse_inbound_frame(raw)
            .unwrap()
            .into_channel_event()
            .unwrap();
        assert!(event.confirmed);
    }

    #[test]
    fn unknown_channel_falls_back_to_other() {
        let raw = r#"{"type":"event","id":"m","channel":"unknownz","confirmed":true,"seq":42,"sent_at_ms":1721818200123,"data":{"foo":1}}"#;
        let event = parse_inbound_frame(raw)
            .unwrap()
            .into_channel_event()
            .unwrap();
        assert!(matches!(event.data, Payload::Other(_)));
    }

    #[test]
    fn parses_warning_frame() {
        let raw = r#"{"type":"warning","code":"mempool_unavailable","id":"t","message":"no pending stream"}"#;
        assert!(matches!(
            parse_inbound_frame(raw),
            Some(InboundFrame::Warning { .. })
        ));
    }

    #[test]
    fn subscribed_ack_echoes_confirmed() {
        let raw = r#"{"type":"subscribed","id":"t","channel":"trading","confirmed":false}"#;
        assert!(matches!(
            parse_inbound_frame(raw),
            Some(InboundFrame::Subscribed {
                confirmed: Some(false),
                ..
            })
        ));
    }

    #[test]
    fn drops_malformed_frames() {
        assert!(parse_inbound_frame("not json").is_none());
        assert!(parse_inbound_frame(r#"{"type":"mystery"}"#).is_none());
    }

    #[test]
    fn drops_event_frames_missing_seq_or_sent_at_ms() {
        let no_envelope =
            r#"{"type":"event","id":"t","channel":"trading","data":{"type":"order_cancelled"}}"#;
        assert!(parse_inbound_frame(no_envelope).is_none());

        let no_sent_at_ms = r#"{"type":"event","id":"t","channel":"trading","seq":42,"data":{"type":"order_cancelled"}}"#;
        assert!(parse_inbound_frame(no_sent_at_ms).is_none());
    }

    #[test]
    fn parses_error_frame() {
        let raw = r#"{"type":"error","message":"boom","code":"bad","id":"x"}"#;
        assert!(matches!(
            parse_inbound_frame(raw),
            Some(InboundFrame::Error { .. })
        ));
    }
}
