//! Typed event payloads for every Radion realtime channel.
//!
//! Each channel emits a `data` object discriminated by a snake_case `type`
//! field. The structs below type the fields documented for each channel's
//! payload.
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
    /// Discriminator for [`TradingPayload`].
    TradingEventType {
        OrderFilledV1,
        OrderFilledV2,
        OrdersMatchedV1,
        OrdersMatchedV2,
        OrderCancelled,
        OrderPreapproved,
        OrderPreapprovalInvalidated,
        TradingPaused,
        TradingUnpaused,
    }
);

/// Order flow on the exchange (fills, matches, cancels, preapprovals, pauses).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TradingPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: TradingEventType,
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
    /// Discriminator for [`FeesPayload`].
    FeesEventType {
        FeeChargedV1,
        FeeChargedV2,
    }
);

/// Exchange fee charged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FeesPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: FeesEventType,
    /// Fee amount charged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<Hex>,
    /// Address charged the fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer: Option<Hex>,
    /// Address receiving the fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<Hex>,
    /// Order hash the fee is attached to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_hash: Option<Hex>,
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

/// UMA question mechanism payload.
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
    /// Discriminator for [`ResolutionPayload`].
    ResolutionEventType {
        ConditionResolution,
        ConditionResolved,
        OutcomeReported,
        ResultReported,
        ResolutionPaused,
        ResolutionUnpaused,
        ResolverPaused,
        ResolverUnpaused,
    }
);

/// Settlement outcome payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ResolutionPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: ResolutionEventType,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Question id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Hex>,
    /// Resolution payouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payouts: Option<Vec<Hex>>,
}

event_type_enum!(
    /// Discriminator for [`LifecyclePayload`].
    LifecycleEventType {
        MarketPrepared,
        EventPrepared,
        ConditionPreparation,
        TokenRegistered,
        NegRiskQuestionPrepared,
        CombinatorialConditionPrepared,
        MigrationConditionRegistered,
    }
);

/// Market creation and prep payload.
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
    /// Discriminator for [`PositionsPayload`].
    PositionsEventType {
        CtfPositionSplit,
        CtfPositionsMerge,
        CtfPayoutRedemption,
        CollateralPositionSplit,
        CollateralPositionsMerged,
        PositionsRedeemed,
    }
);

/// Plain CTF base-layer split / merge / redemption payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PositionsPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: PositionsEventType,
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
    /// ERC-1155 token id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_id: Option<Hex>,
}

event_type_enum!(
    /// Discriminator for [`CombosPayload`].
    CombosEventType {
        Redemption,
        BinaryRedemption,
        NegRiskRedemption,
        CollateralPositionsConverted,
        NegRiskPositionsConverted,
        PositionConverted,
        PositionRedeemed,
        ModulePositionsMerged,
        ModulePositionsSplit,
        HorizontalMerge,
        HorizontalSplit,
        SplitOnCondition,
        MergedOnCondition,
        ConvertedToYesBasket,
        MergedFromYesBasket,
        Extracted,
        Injected,
        Compressed,
        CombinatorialWrapped,
        CombinatorialUnwrapped,
        PositionMigrated,
        MigrationResolved,
        BridgePositionMinted,
        BridgePositionsBurned,
        LegacyCollateralSettled,
    }
);

/// Module / redeemer / neg-risk / combinatorial system payload.
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
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
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

event_type_enum!(
    /// Discriminator for [`TransfersPayload`].
    TransfersEventType {
        TransferSingle,
        TransferBatch,
    }
);

/// ERC-1155 outcome-token move payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TransfersPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: TransfersEventType,
    /// Operator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Hex>,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Hex>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Hex>,
    /// Token id (`TransferSingle`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Hex>,
    /// Amount moved (`TransferSingle`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Hex>,
    /// Token ids (`TransferBatch`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<Hex>>,
    /// Amounts moved (`TransferBatch`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Hex>>,
}

event_type_enum!(
    /// Discriminator for [`AccountsPayload`].
    AccountsEventType {
        WalletDeployed,
        ProxyCreation,
    }
);

/// Proxy wallet creation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AccountsPayload {
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: AccountsEventType,
    /// The deployed / created proxy wallet address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<Hex>,
    /// The owner controlling the proxy wallet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Hex>,
    /// Proxy implementation address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Hex>,
}

/// The typed payload carried by a channel event.
///
/// The active variant is determined by the event frame's `channel` field. The
/// `wallets` and `markets` filter channels re-emit confirmed payloads, so they
/// deserialize to whichever confirmed variant matches its `type`. Unknown
/// channels, unknown `type` values, or data that does not match any typed
/// payload fall back to [`Payload::Other`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Payload {
    /// `trading` payload.
    Trading(TradingPayload),
    /// `fees` payload.
    Fees(FeesPayload),
    /// `oracle` payload.
    Oracle(OraclePayload),
    /// `resolution` payload.
    Resolution(ResolutionPayload),
    /// `lifecycle` payload.
    Lifecycle(LifecyclePayload),
    /// `positions` payload.
    Positions(PositionsPayload),
    /// `combos` payload.
    Combos(CombosPayload),
    /// `transfers` payload.
    Transfers(TransfersPayload),
    /// `accounts` payload.
    Accounts(AccountsPayload),
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
            Channel::Trading => typed(data, Payload::Trading),
            Channel::Fees => typed(data, Payload::Fees),
            Channel::Oracle => typed(data, Payload::Oracle),
            Channel::Resolution => typed(data, Payload::Resolution),
            Channel::Lifecycle => typed(data, Payload::Lifecycle),
            Channel::Positions => typed(data, Payload::Positions),
            Channel::Combos => typed(data, Payload::Combos),
            Channel::Transfers => typed(data, Payload::Transfers),
            Channel::Accounts => typed(data, Payload::Accounts),
            // Filtered views re-emit confirmed payloads; the untagged enum picks
            // the variant whose `type` matches, preserving unknowns.
            Channel::Wallets | Channel::Markets => {
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
    fn trading_types_a_fill() {
        let data = json!({"type":"order_filled_v2","side":1,"tokenId":"0xabc"});
        match Payload::from_channel(Channel::Trading, data) {
            Payload::Trading(t) => {
                assert_eq!(t.kind, TradingEventType::OrderFilledV2);
                assert_eq!(t.side, Some(1));
                assert_eq!(t.token_id.as_deref(), Some("0xabc"));
            }
            other => panic!("expected trading, got {other:?}"),
        }
    }

    #[test]
    fn wallets_view_discriminates_by_type() {
        // A lifecycle event arriving on the `wallets` filter channel types correctly.
        let data = json!({"type":"market_prepared","conditionId":"0xc"});
        assert!(matches!(
            Payload::from_channel(Channel::Wallets, data),
            Payload::Lifecycle(_)
        ));
    }

    #[test]
    fn unknown_event_type_falls_back_to_other() {
        let data = json!({"type":"brand_new_event","foo":1});
        assert!(matches!(
            Payload::from_channel(Channel::Trading, data),
            Payload::Other(_)
        ));
    }
}
