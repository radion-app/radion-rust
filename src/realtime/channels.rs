//! Channel names and per-channel filter requirements.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::RadionError;

/// A confirmed channel the SDK can subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Channel {
    /// Order flow on the exchange (fills and order matches).
    Trading,
    /// Exchange fees charged.
    Fees,
    /// UMA question mechanism.
    Oracle,
    /// Settlement outcomes.
    Resolution,
    /// Market creation and prep.
    Lifecycle,
    /// Plain CTF base layer splits, merges, and redemptions.
    Positions,
    /// Module / redeemer / neg-risk / combinatorial system.
    Combos,
    /// ERC-1155 outcome-token moves.
    Transfers,
    /// Proxy wallet creation.
    Accounts,
    /// Events filtered to specific wallets (cross-cutting).
    Wallets,
    /// Events filtered to specific markets (cross-cutting).
    Markets,
}

/// Every confirmed channel, in declaration order.
pub const CHANNELS: [Channel; 11] = [
    Channel::Trading,
    Channel::Fees,
    Channel::Oracle,
    Channel::Resolution,
    Channel::Lifecycle,
    Channel::Positions,
    Channel::Combos,
    Channel::Transfers,
    Channel::Accounts,
    Channel::Wallets,
    Channel::Markets,
];

impl Channel {
    /// The wire name of this channel (e.g. `"trading"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trading => "trading",
            Self::Fees => "fees",
            Self::Oracle => "oracle",
            Self::Resolution => "resolution",
            Self::Lifecycle => "lifecycle",
            Self::Positions => "positions",
            Self::Combos => "combos",
            Self::Transfers => "transfers",
            Self::Accounts => "accounts",
            Self::Wallets => "wallets",
            Self::Markets => "markets",
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Channel {
    type Err = RadionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        CHANNELS
            .into_iter()
            .find(|channel| channel.as_str() == value)
            .ok_or_else(|| RadionError::connection(format!("unknown channel \"{value}\"")))
    }
}

/// A CLOB (central limit order book) channel the SDK can subscribe to.
///
/// The CLOB family is separate from the topic [`Channel`] set: names are
/// `clob.`-prefixed, each subscription requires at least one `token_id`, and no
/// channel has a `mempool.` companion. Each CLOB channel carries one fixed
/// payload shape with no event `type` discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClobChannel {
    /// Full order book snapshots and updates.
    Book,
    /// Per-asset price changes.
    Prices,
    /// Last trade prints.
    LastTrade,
    /// Order book midpoint.
    Midpoint,
    /// Minimum price increment.
    TickSize,
    /// Best bid and ask.
    BestBidAsk,
}

/// Every CLOB channel, in declaration order.
pub const CLOB_CHANNELS: [ClobChannel; 6] = [
    ClobChannel::Book,
    ClobChannel::Prices,
    ClobChannel::LastTrade,
    ClobChannel::Midpoint,
    ClobChannel::TickSize,
    ClobChannel::BestBidAsk,
];

impl ClobChannel {
    /// The wire name of this channel (e.g. `"clob.book"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Book => "clob.book",
            Self::Prices => "clob.prices",
            Self::LastTrade => "clob.last_trade",
            Self::Midpoint => "clob.midpoint",
            Self::TickSize => "clob.tick_size",
            Self::BestBidAsk => "clob.best_bid_ask",
        }
    }
}

impl fmt::Display for ClobChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ClobChannel {
    type Err = RadionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        CLOB_CHANNELS
            .into_iter()
            .find(|channel| channel.as_str() == value)
            .ok_or_else(|| RadionError::connection(format!("unknown clob channel \"{value}\"")))
    }
}

const MEMPOOL_PREFIX: &str = "mempool.";
const CLOB_PREFIX: &str = "clob.";

/// A channel name accepted by [`subscribe`](super::RealtimeClient::subscribe) —
/// a confirmed topic channel, its `mempool.` companion emitting speculative
/// pending transactions before block inclusion, or a [`ClobChannel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubscribableChannel {
    /// A confirmed topic channel.
    Confirmed(Channel),
    /// The `mempool.`-prefixed companion of a confirmed topic channel.
    Mempool(Channel),
    /// A CLOB channel (no `mempool.` companion).
    Clob(ClobChannel),
}

impl SubscribableChannel {
    /// The confirmed topic channel underlying this subscription (mempool or
    /// not), or `None` for a CLOB channel.
    pub fn confirmed(&self) -> Option<Channel> {
        match self {
            Self::Confirmed(channel) | Self::Mempool(channel) => Some(*channel),
            Self::Clob(_) => None,
        }
    }

    /// Whether this is a `mempool.` companion channel.
    pub fn is_mempool(&self) -> bool {
        matches!(self, Self::Mempool(_))
    }

    /// Whether this is a CLOB channel.
    pub fn is_clob(&self) -> bool {
        matches!(self, Self::Clob(_))
    }

    /// The filter requirement for this channel, if any. CLOB channels require a
    /// `token_ids` filter; mempool companions share their confirmed channel's
    /// requirements.
    pub(crate) fn filter_requirement(&self) -> Option<FilterRequirement> {
        match self {
            Self::Confirmed(channel) | Self::Mempool(channel) => filter_requirement(*channel),
            Self::Clob(_) => Some(FilterRequirement {
                required_any_of: &[FilterKey::TokenIds],
            }),
        }
    }
}

impl fmt::Display for SubscribableChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Confirmed(channel) => write!(f, "{channel}"),
            Self::Mempool(channel) => write!(f, "{MEMPOOL_PREFIX}{channel}"),
            Self::Clob(channel) => write!(f, "{channel}"),
        }
    }
}

impl FromStr for SubscribableChannel {
    type Err = RadionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = value.strip_prefix(MEMPOOL_PREFIX) {
            return rest.parse().map(Self::Mempool);
        }
        if value.starts_with(CLOB_PREFIX) {
            return value.parse().map(Self::Clob);
        }
        value.parse().map(Self::Confirmed)
    }
}

impl From<Channel> for SubscribableChannel {
    fn from(channel: Channel) -> Self {
        Self::Confirmed(channel)
    }
}

impl From<ClobChannel> for SubscribableChannel {
    fn from(channel: ClobChannel) -> Self {
        Self::Clob(channel)
    }
}

impl Serialize for SubscribableChannel {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// A server-side filter key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKey {
    /// Wallet addresses.
    Wallets,
    /// Condition / market ids.
    MarketIds,
    /// ERC-1155 token ids.
    TokenIds,
    /// Minimum trade notional in USD.
    MinUsd,
}

impl FilterKey {
    fn label(self) -> &'static str {
        match self {
            Self::Wallets => "wallets",
            Self::MarketIds => "market_ids",
            Self::TokenIds => "token_ids",
            Self::MinUsd => "min_usd",
        }
    }
}

/// Per-channel filter requirement.
pub(crate) struct FilterRequirement {
    /// At least one of these filters must be present.
    pub required_any_of: &'static [FilterKey],
}

/// The filter requirement for a confirmed channel, if any. Channels absent here
/// accept no required filters. Mempool companions share their confirmed
/// channel's requirements.
pub(crate) fn filter_requirement(channel: Channel) -> Option<FilterRequirement> {
    match channel {
        Channel::Markets => Some(FilterRequirement {
            required_any_of: &[FilterKey::MarketIds, FilterKey::TokenIds],
        }),
        Channel::Wallets => Some(FilterRequirement {
            required_any_of: &[FilterKey::Wallets],
        }),
        _ => None,
    }
}

pub(crate) fn join_filter_keys(keys: &[FilterKey]) -> String {
    keys.iter()
        .map(|key| key.label())
        .collect::<Vec<_>>()
        .join(" or ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_roundtrips_through_str() {
        for channel in CHANNELS {
            assert_eq!(channel.as_str().parse::<Channel>().unwrap(), channel);
        }
        assert_eq!("trading".parse::<Channel>().unwrap(), Channel::Trading);
        assert!("nope".parse::<Channel>().is_err());
    }

    #[test]
    fn subscribable_channel_handles_mempool_prefix() {
        let confirmed: SubscribableChannel = Channel::Trading.into();
        assert_eq!(confirmed.to_string(), "trading");
        assert!(!confirmed.is_mempool());

        let mempool: SubscribableChannel = "mempool.trading".parse().unwrap();
        assert_eq!(mempool, SubscribableChannel::Mempool(Channel::Trading));
        assert_eq!(mempool.to_string(), "mempool.trading");
        assert_eq!(mempool.confirmed(), Some(Channel::Trading));
        assert!(mempool.is_mempool());
        assert!(!mempool.is_clob());
    }

    #[test]
    fn clob_channel_roundtrips_through_str() {
        for channel in CLOB_CHANNELS {
            assert_eq!(channel.as_str().parse::<ClobChannel>().unwrap(), channel);
        }
        assert_eq!(
            "clob.book".parse::<ClobChannel>().unwrap(),
            ClobChannel::Book
        );
        assert_eq!(
            "clob.best_bid_ask".parse::<ClobChannel>().unwrap(),
            ClobChannel::BestBidAsk
        );
        assert!("clob.nope".parse::<ClobChannel>().is_err());
    }

    #[test]
    fn subscribable_channel_handles_clob_prefix() {
        let clob: SubscribableChannel = ClobChannel::LastTrade.into();
        assert_eq!(clob, SubscribableChannel::Clob(ClobChannel::LastTrade));
        assert_eq!(clob.to_string(), "clob.last_trade");
        assert!(clob.is_clob());
        assert!(!clob.is_mempool());
        assert_eq!(clob.confirmed(), None);

        let parsed: SubscribableChannel = "clob.book".parse().unwrap();
        assert_eq!(parsed, SubscribableChannel::Clob(ClobChannel::Book));
    }

    #[test]
    fn filter_requirements_match_docs() {
        assert!(filter_requirement(Channel::Markets).is_some());
        assert!(filter_requirement(Channel::Wallets).is_some());
        assert!(filter_requirement(Channel::Trading).is_none());

        // Every clob channel requires token_ids.
        for channel in CLOB_CHANNELS {
            let requirement = SubscribableChannel::Clob(channel)
                .filter_requirement()
                .expect("clob requires a filter");
            assert_eq!(requirement.required_any_of, &[FilterKey::TokenIds]);
        }
    }
}
