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
/// apart. `channel` is a confirmed topic channel, its `mempool.` companion, or a
/// CLOB channel.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    /// Client-defined id, echoed back on confirmations and event frames.
    pub id: String,
    /// Channel name: topic, `mempool.`-prefixed, or `clob.`-prefixed.
    pub channel: SubscribableChannel,
    /// Optional server-side filters.
    pub filters: Option<ChannelFilters>,
}

impl Subscription {
    /// Create a subscription with no filters.
    pub fn new(id: impl Into<String>, channel: impl Into<SubscribableChannel>) -> Self {
        Self {
            id: id.into(),
            channel: channel.into(),
            filters: None,
        }
    }

    /// Attach server-side filters.
    #[must_use]
    pub fn with_filters(mut self, filters: ChannelFilters) -> Self {
        self.filters = Some(filters);
        self
    }

    /// Validate that this subscription carries the filters its channel requires.
    ///
    /// # Errors
    ///
    /// Returns [`RadionError::Connection`] describing the first violation.
    /// Mempool companions share their confirmed channel's requirements; every
    /// CLOB channel requires a `token_ids` filter.
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
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum OutboundFrame {
    Subscribe {
        id: String,
        channel: SubscribableChannel,
        #[serde(skip_serializing_if = "Option::is_none")]
        filters: Option<ChannelFilters>,
    },
    Unsubscribe {
        id: String,
    },
    Ping,
}

impl OutboundFrame {
    pub(crate) fn subscribe(subscription: &Subscription) -> Self {
        Self::Subscribe {
            id: subscription.id.clone(),
            channel: subscription.channel,
            filters: subscription.filters.clone(),
        }
    }
}

/// A data event delivered on a subscribed channel.
///
/// `id` identifies the subscription it belongs to; `channel` is the resolved
/// channel name (possibly `mempool.`-prefixed). `data` is the typed payload —
/// match on it to handle a specific event shape.
#[derive(Debug, Clone)]
pub struct ChannelEvent {
    /// Subscription id this event belongs to.
    pub id: String,
    /// Resolved channel name.
    pub channel: String,
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
        data: serde_json::Value,
    },
    Subscribed {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        channel: Option<String>,
    },
    Unsubscribed {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        channel: Option<String>,
    },
    Pong,
    Error {
        message: String,
        code: Option<String>,
        id: Option<String>,
        channel: Option<String>,
        #[allow(dead_code)]
        skipped: Option<u64>,
    },
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
    /// against the resolved channel. Returns `None` for non-event frames.
    pub(crate) fn into_channel_event(self) -> Option<ChannelEvent> {
        let Self::Event { id, channel, data } = self else {
            return None;
        };
        let payload = if let Ok(clob) = channel.parse::<ClobChannel>() {
            Payload::from_clob_channel(clob, data)
        } else {
            match channel
                .strip_prefix("mempool.")
                .unwrap_or(&channel)
                .parse::<Channel>()
            {
                Ok(topic) => Payload::from_channel(topic, data),
                Err(_) => Payload::Other(data),
            }
        };
        Some(ChannelEvent {
            id,
            channel,
            data: payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::channels::ClobChannel;
    use crate::realtime::payloads::{Payload, TradingEventType};

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
        let raw = r#"{"type":"event","id":"mid","channel":"clob.midpoint","data":{"asset_id":"7","market":"0xm","midpoint":0.5,"timestamp":1}}"#;
        let event = parse_inbound_frame(raw)
            .expect("valid frame")
            .into_channel_event()
            .expect("event");
        assert_eq!(event.channel, "clob.midpoint");
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
        // No filters key when none are set.
        assert!(json.get("filters").is_none());
    }

    #[test]
    fn parses_and_types_event_frames() {
        let raw = r#"{"type":"event","id":"t","channel":"trading","data":{"type":"order_filled_v2","side":1,"tokenId":"0xabc"}}"#;
        let frame = parse_inbound_frame(raw).expect("valid frame");
        let event = frame.into_channel_event().expect("event");
        assert_eq!(event.id, "t");
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
    fn unknown_channel_falls_back_to_other() {
        let raw = r#"{"type":"event","id":"m","channel":"mempool.unknownz","data":{"foo":1}}"#;
        let event = parse_inbound_frame(raw)
            .unwrap()
            .into_channel_event()
            .unwrap();
        assert!(matches!(event.data, Payload::Other(_)));
    }

    #[test]
    fn drops_malformed_frames() {
        assert!(parse_inbound_frame("not json").is_none());
        assert!(parse_inbound_frame(r#"{"type":"mystery"}"#).is_none());
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
