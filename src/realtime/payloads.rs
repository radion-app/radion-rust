//! Typed event payloads for every Radion realtime channel.
//!
//! Each channel emits a `data` object discriminated by a snake_case `type`
//! field (the `prices` channel is the exception — a flat tick with no `type`).
//! The structs below type the fields documented for each channel's payload.
//!
//! Provenance mirrors the published channel docs (`/websockets/channels/*`).
//! An event whose `type` this SDK version does not enumerate — or whose shape
//! does not match the channel's typed payload — is preserved as
//! [`Payload::Other`] rather than dropped, mirroring the TS/Python SDKs' loose
//! validation for forward compatibility.
//!
//! On-chain amounts stay **strings** to remain bigint-safe; do not assume they
//! fit a numeric type.

use serde::{Deserialize, Serialize};

use super::channels::Channel;

/// Hex-encoded string (`0x…`) or other opaque on-chain string value.
pub type Hex = String;

macro_rules! event_type_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[non_exhaustive]
        // Variants map 1:1 to documented wire event types; names are the doc,
        // and shared prefixes (e.g. `Uma`) come straight from the protocol.
        #[allow(missing_docs, clippy::enum_variant_names)]
        pub enum $name {
            $($(#[$vmeta])* $variant,)*
        }
    };
}

event_type_enum!(
    /// Discriminator for [`TradesPayload`].
    TradeEventType {
        OrderFilledV1,
        OrderFilledV2,
        OrdersMatchedV1,
        OrdersMatchedV2,
    }
);

/// Confirmed fill / order-match payload from the exchange contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TradesPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: TradeEventType,
    /// `0` = buy, `1` = sell. v2 fills and matches only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<i64>,
    /// Builder address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder: Option<Hex>,
    /// Fee paid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<Hex>,
    /// Maker address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maker: Option<Hex>,
    /// Maker amount filled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maker_amount_filled: Option<Hex>,
    /// Opaque order metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Hex>,
    /// Order hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_hash: Option<Hex>,
    /// Taker address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker: Option<Hex>,
    /// Taker amount filled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_amount_filled: Option<Hex>,
    /// ERC-1155 token id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<Hex>,
}

event_type_enum!(
    /// Discriminator for [`OraclePayload`].
    OracleEventType {
        UmaAdapterQuestionInitialized,
        UmaAdapterQuestionResolved,
        UmaAdapterQuestionEmergencyResolved,
        UmaAdapterQuestionFlagged,
        UmaAdapterQuestionPaused,
        UmaAdapterQuestionUnpaused,
        UmaAdapterQuestionReset,
        UmaAdapterAncillaryDataUpdated,
        UmaOptimisticQuestionInitialized,
        UmaOptimisticQuestionResolved,
        UmaOptimisticQuestionPaused,
        UmaOptimisticQuestionUnpaused,
        UmaOptimisticQuestionSettled,
        UmaOptimisticResolutionDataRequested,
        UmaOptimisticQuestionUpdated,
        UmaOptimisticQuestionFlaggedForAdminResolution,
    }
);

/// UMA oracle lifecycle payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct OraclePayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: OracleEventType,
    /// UMA question id.
    #[serde(rename = "questionID", skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Hex>,
    /// Resolution payouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payouts: Option<Vec<Hex>>,
    /// `int256` price as a signed decimal string (e.g. `"-1"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settled_price: Option<String>,
}

event_type_enum!(
    /// Discriminator for [`LifecyclePayload`].
    LifecycleEventType {
        MarketPrepared,
        NegRiskQuestionPrepared,
        OutcomeReported,
        EventPrepared,
        ConditionResolved,
        ConditionPreparation,
        ConditionResolution,
        TokenRegistered,
    }
);

/// Market / condition lifecycle payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LifecyclePayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: LifecycleEventType,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Oracle address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle: Option<Hex>,
    /// Outcome slot count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_slot_count: Option<Hex>,
    /// Question id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Hex>,
}

event_type_enum!(
    /// Discriminator for [`ActivityPayload`].
    ActivityEventType {
        Redemption,
        BinaryRedemption,
        NegRiskRedemption,
        PositionsRedeemed,
        CollateralPositionSplit,
        CollateralPositionsMerged,
        CollateralPositionsConverted,
        NegRiskPositionsConverted,
        CtfPositionSplit,
        CtfPositionsMerge,
        CtfPayoutRedemption,
    }
);

/// Redemption / split / merge / conversion payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ActivityPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: ActivityEventType,
    /// Amounts involved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amounts: Option<Vec<Hex>>,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Initiating address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<Hex>,
    /// Payout amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout: Option<Hex>,
}

event_type_enum!(
    /// Discriminator for [`CollateralPayload`].
    CollateralEventType {
        Transfer,
        Approval,
        Wrapped,
        Unwrapped,
    }
);

/// ERC-20 collateral payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CollateralPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: CollateralEventType,
    /// Amount transferred / approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Hex>,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Hex>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Hex>,
}

event_type_enum!(
    /// Discriminator for [`CombosPayload`].
    CombosEventType {
        EventPrepared,
        ResultReported,
        PositionRedeemed,
        ModulePositionsMerged,
        ModulePositionsSplit,
        HorizontalMerge,
        HorizontalSplit,
        PositionConverted,
        ConditionResolved,
        ResolutionPaused,
        ResolutionUnpaused,
        ResolverPaused,
        ResolverUnpaused,
        BridgePositionMinted,
        BridgePositionsBurned,
        LegacyCollateralSettled,
        MigrationConditionRegistered,
        MigrationResolved,
        PositionMigrated,
        CombinatorialConditionPrepared,
        Compressed,
        ConvertedToYesBasket,
        Extracted,
        Injected,
        MergedFromYesBasket,
        MergedOnCondition,
        SplitOnCondition,
        CombinatorialWrapped,
        CombinatorialUnwrapped,
        TransferSingle,
        TransferBatch,
    }
);

/// Module / bridge / combinatorial / ERC-1155 payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CombosPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: CombosEventType,
    /// Amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Hex>,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Hex>,
    /// Token / position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Hex>,
    /// Operator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Hex>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Hex>,
}

/// Last-traded price tick. Flat shape — no `type` discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PricesPayload {
    /// ERC-1155 token id.
    pub token_id: Hex,
    /// Last-traded price, USDC per share.
    pub price: f64,
    /// When the tick was produced (Unix ms).
    pub timestamp_ms: i64,
}

/// The typed payload carried by a channel event.
///
/// The active variant is determined by the event frame's `channel` field. The
/// `global`, `wallets`, and `markets` channels re-emit confirmed payloads, so
/// they deserialize to whichever confirmed variant matches its `type`. Unknown
/// channels, unknown `type` values, or data that does not match any typed
/// payload fall back to [`Payload::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Payload {
    /// `trades` / `large_trades` payload.
    Trades(TradesPayload),
    /// `oracle` payload.
    Oracle(OraclePayload),
    /// `lifecycle` payload.
    Lifecycle(LifecyclePayload),
    /// `activity` payload.
    Activity(ActivityPayload),
    /// `collateral` payload.
    Collateral(CollateralPayload),
    /// `combos` payload.
    Combos(CombosPayload),
    /// `prices` tick.
    Prices(PricesPayload),
    /// Any structurally valid payload the SDK does not type.
    Other(serde_json::Value),
}

impl Payload {
    /// Decode raw event `data` into the typed payload for `channel`.
    ///
    /// Never fails: data that does not match the channel's typed shape is
    /// preserved as [`Payload::Other`].
    pub(crate) fn from_channel(channel: Channel, data: serde_json::Value) -> Self {
        fn typed<T, F>(data: serde_json::Value, wrap: F) -> Payload
        where
            T: for<'de> Deserialize<'de>,
            F: FnOnce(T) -> Payload,
        {
            match serde_json::from_value::<T>(data.clone()) {
                Ok(value) => wrap(value),
                Err(_) => Payload::Other(data),
            }
        }

        match channel {
            Channel::Trades | Channel::LargeTrades => typed(data, Payload::Trades),
            Channel::Oracle => typed(data, Payload::Oracle),
            Channel::Lifecycle => typed(data, Payload::Lifecycle),
            Channel::Activity => typed(data, Payload::Activity),
            Channel::Collateral => typed(data, Payload::Collateral),
            Channel::Combos => typed(data, Payload::Combos),
            Channel::Prices => typed(data, Payload::Prices),
            // Firehose / filtered views re-emit confirmed payloads; the untagged
            // enum picks the variant whose `type` matches, preserving unknowns.
            Channel::Global | Channel::Wallets | Channel::Markets => {
                serde_json::from_value(data.clone()).unwrap_or(Payload::Other(data))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn oracle_renames_question_id() {
        let data = json!({"type":"uma_optimistic_question_resolved","questionID":"0xq","settledPrice":"-1"});
        match Payload::from_channel(Channel::Oracle, data) {
            Payload::Oracle(o) => {
                assert_eq!(o.kind, OracleEventType::UmaOptimisticQuestionResolved);
                assert_eq!(o.question_id.as_deref(), Some("0xq"));
                assert_eq!(o.settled_price.as_deref(), Some("-1"));
            }
            other => panic!("expected oracle, got {other:?}"),
        }
    }

    #[test]
    fn prices_is_a_flat_tick() {
        let data = json!({"token_id":"0x1","price":0.42,"timestamp_ms":1700000000000i64});
        match Payload::from_channel(Channel::Prices, data) {
            Payload::Prices(p) => {
                assert_eq!(p.token_id, "0x1");
                assert!((p.price - 0.42).abs() < f64::EPSILON);
            }
            other => panic!("expected prices, got {other:?}"),
        }
    }

    #[test]
    fn global_firehose_discriminates_by_type() {
        // A lifecycle event arriving on the `global` firehose types correctly.
        let data = json!({"type":"market_prepared","conditionId":"0xc"});
        assert!(matches!(
            Payload::from_channel(Channel::Global, data),
            Payload::Lifecycle(_)
        ));
    }

    #[test]
    fn unknown_event_type_falls_back_to_other() {
        let data = json!({"type":"brand_new_event","foo":1});
        assert!(matches!(
            Payload::from_channel(Channel::Trades, data),
            Payload::Other(_)
        ));
    }
}
