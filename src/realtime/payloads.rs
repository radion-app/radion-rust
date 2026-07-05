//! Typed event payloads for every Radion realtime channel.
//!
//! Each channel emits a `data` object discriminated by a snake_case `type`
//! field. The structs below type the fields documented for each channel's
//! payload. The channel docs now enumerate every event's full field set, and
//! each struct is the **union** of the fields carried by all of its channel's
//! events — Rust drops any field not declared on the struct, so each struct
//! lists every field its events can carry.
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

use super::channels::{Channel, ClobChannel};

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
    /// Maker asset id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maker_asset_id: Option<Hex>,
    /// Opaque order metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Hex>,
    /// Order hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_hash: Option<Hex>,
    /// Address pausing trading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pauser: Option<Hex>,
    /// `0` = buy, `1` = sell. v2 fills and matches only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<i64>,
    /// Taker address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker: Option<Hex>,
    /// Taker amount filled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_amount_filled: Option<Hex>,
    /// Taker asset id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_asset_id: Option<Hex>,
    /// Hash of the taker order (matches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_order_hash: Option<Hex>,
    /// Maker of the taker order (matches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_order_maker: Option<Hex>,
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
    pub amount: Option<Hex>,
    /// Address receiving the fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<Hex>,
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
    /// Ancillary data for the question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ancillary_data: Option<Hex>,
    /// Question creator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<Hex>,
    /// Whether early resolution occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_resolution: Option<bool>,
    /// Whether early resolution is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub early_resolution_enabled: Option<bool>,
    /// Whether the report was an emergency report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency_report: Option<bool>,
    /// Price identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Hex>,
    /// Question owner address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<Hex>,
    /// Resolution payouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payouts: Option<Vec<Hex>>,
    /// Proposal bond amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal_bond: Option<Hex>,
    /// Request timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_timestamp: Option<Hex>,
    /// Requestor address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requestor: Option<Hex>,
    /// Resolution reward amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward: Option<Hex>,
    /// Reward token address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward_token: Option<Hex>,
    /// Resolution time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_time: Option<Hex>,
    /// `int256` price as a signed decimal string (e.g. `"-1"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settled_price: Option<String>,
    /// Ancillary data update payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<Hex>,
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
    /// Event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Hex>,
    /// Market id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<Hex>,
    /// Oracle address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle: Option<Hex>,
    /// Reported outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<bool>,
    /// Outcome slot count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_slot_count: Option<Hex>,
    /// Resolution payout numerators.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout_numerators: Option<Vec<Hex>>,
    /// Question id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Hex>,
    /// Reported result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Hex>>,
    /// Resolver address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver: Option<Hex>,
    /// Event timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Hex>,
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
    /// Number of conditions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_count: Option<Hex>,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Opaque event data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Hex>,
    /// Event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<Hex>,
    /// Fee in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_bips: Option<Hex>,
    /// Position / slot index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<Hex>,
    /// Legacy condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_condition_id: Option<Hex>,
    /// Legacy event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_event_id: Option<Hex>,
    /// Combinatorial legs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legs: Option<Vec<Hex>>,
    /// Market id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<Hex>,
    /// Oracle address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle: Option<Hex>,
    /// Outcome slot count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_slot_count: Option<Hex>,
    /// Question id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question_id: Option<Hex>,
    /// First outcome token id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token0: Option<Hex>,
    /// Second outcome token id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token1: Option<Hex>,
    /// v2 condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_condition_id: Option<Hex>,
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
    /// Amount involved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Hex>,
    /// Amounts involved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amounts: Option<Vec<Hex>>,
    /// Collateral token address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_token: Option<Hex>,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Index sets redeemed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_sets: Option<Vec<Hex>>,
    /// Initiating address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<Hex>,
    /// Parent collection id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_collection_id: Option<Hex>,
    /// Partition of index sets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<Vec<Hex>>,
    /// Payout amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout: Option<Hex>,
    /// Redeeming address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redeemer: Option<Hex>,
    /// Stakeholder address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholder: Option<Hex>,
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
    /// Collateral amount out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_out: Option<Hex>,
    /// Amounts involved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amounts: Option<Vec<Hex>>,
    /// Child NO condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_no_condition_id: Option<Hex>,
    /// Child YES condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_yes_condition_id: Option<Hex>,
    /// Collateral amount out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_out: Option<Hex>,
    /// Combinatorial position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combinatorial_position_id: Option<Hex>,
    /// Condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_id: Option<Hex>,
    /// Condition index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_index: Option<Hex>,
    /// Event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<Hex>,
    /// Sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Hex>,
    /// Full condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_condition_id: Option<Hex>,
    /// Index set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_set: Option<Hex>,
    /// Initiating address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<Hex>,
    /// Legacy condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_condition_id: Option<Hex>,
    /// Legacy payout for outcome 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_payout0: Option<Hex>,
    /// Legacy payout for outcome 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_payout1: Option<Hex>,
    /// Legacy token address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_token: Option<Hex>,
    /// Market id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<Hex>,
    /// New position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_position_id: Option<Hex>,
    /// Old position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_position_id: Option<Hex>,
    /// Outcome index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_index: Option<Hex>,
    /// Parent condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_condition_id: Option<Hex>,
    /// Payout amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payout: Option<Hex>,
    /// Position amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_amount: Option<Hex>,
    /// Position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_id: Option<Hex>,
    /// Position ids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_ids: Option<Vec<Hex>>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient: Option<Hex>,
    /// First recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient0: Option<Hex>,
    /// Second recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient1: Option<Hex>,
    /// Reduced condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduced_condition_id: Option<Hex>,
    /// Residual condition id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_condition_id: Option<Hex>,
    /// Result for outcome 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result0: Option<Hex>,
    /// Result for outcome 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result1: Option<Hex>,
    /// Stakeholder address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stakeholder: Option<Hex>,
    /// Recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Hex>,
    /// Underlying position id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlying_position_id: Option<Hex>,
    /// User address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<Hex>,
    /// Vault address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<Hex>,
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
    pub amount: Option<Hex>,
    /// Token ids (`TransferBatch`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<Hex>>,
    /// Amounts moved (`TransferBatch`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amounts: Option<Vec<Hex>>,
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
    /// Account / proxy id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Hex>,
    /// Proxy implementation contract address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation: Option<Hex>,
    /// Created proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<Hex>,
}

// -- CLOB channel payloads ---------------------------------------------------
//
// The CLOB family is proxied separately from the topic channels. Each channel
// has ONE fixed `data` shape with NO `type` discriminator, and its fields are
// wire-serialized in `snake_case`. Ids stay strings (`asset_id` is a U256
// decimal string, `market` a `0x…` hex string); `timestamp` is a number; prices
// and sizes are numbers, optional where the wire marks them optional.

/// A single price level in a CLOB order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Level {
    /// Price of this level.
    pub price: f64,
    /// Size resting at this price.
    pub size: f64,
}

/// `clob.book` payload: an order book snapshot for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobBookPayload {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Server timestamp.
    pub timestamp: i64,
    /// Bid levels.
    pub bids: Vec<Level>,
    /// Ask levels.
    pub asks: Vec<Level>,
}

/// `clob.last_trade` payload: the most recent trade print for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobLastTradePayload {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Trade price.
    pub price: f64,
    /// Trade size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    /// Server timestamp.
    pub timestamp: i64,
}

/// A single per-asset price change in a [`ClobPricesPayload`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PriceChange {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// New price.
    pub price: f64,
    /// Size associated with the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    /// Best bid after the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_bid: Option<f64>,
    /// Best ask after the change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_ask: Option<f64>,
}

/// `clob.prices` payload: a batch of per-asset price changes in one market.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobPricesPayload {
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Server timestamp.
    pub timestamp: i64,
    /// The price changes in this batch.
    pub changes: Vec<PriceChange>,
}

/// `clob.midpoint` payload: the order book midpoint for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobMidpointPayload {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Midpoint price.
    pub midpoint: f64,
    /// Server timestamp.
    pub timestamp: i64,
}

/// `clob.tick_size` payload: the minimum price increment for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobTickSizePayload {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Server timestamp.
    pub timestamp: i64,
}

/// `clob.best_bid_ask` payload: the top of book for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClobBestBidAskPayload {
    /// U256 outcome-token id as a decimal string.
    pub asset_id: String,
    /// Market condition id (`0x…` hex string).
    pub market: String,
    /// Best bid price.
    pub best_bid: f64,
    /// Best ask price.
    pub best_ask: f64,
    /// Server timestamp.
    pub timestamp: i64,
}

/// The typed payload carried by a channel event.
///
/// The active variant is determined by the event frame's `channel` field. The
/// `wallets` and `markets` filter channels re-emit confirmed payloads, so they
/// deserialize to whichever confirmed variant matches its `type`. CLOB channels
/// map to their fixed payload shape (no `type` discriminator). Unknown channels,
/// unknown `type` values, or data that does not match any typed payload fall
/// back to [`Payload::Other`].
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
    /// `clob.book` payload.
    ClobBook(ClobBookPayload),
    /// `clob.prices` payload.
    ClobPrices(ClobPricesPayload),
    /// `clob.last_trade` payload.
    ClobLastTrade(ClobLastTradePayload),
    /// `clob.midpoint` payload.
    ClobMidpoint(ClobMidpointPayload),
    /// `clob.tick_size` payload.
    ClobTickSize(ClobTickSizePayload),
    /// `clob.best_bid_ask` payload.
    ClobBestBidAsk(ClobBestBidAskPayload),
    /// Any structurally valid payload the SDK does not type.
    Other(serde_json::Value),
}

/// Deserialize `data` into `T`, falling back to [`Payload::Other`] on mismatch.
fn typed_payload<T, F>(data: serde_json::Value, wrap: F) -> Payload
where
    T: for<'de> Deserialize<'de>,
    F: FnOnce(T) -> Payload,
{
    match serde_json::from_value::<T>(data.clone()) {
        Ok(value) => wrap(value),
        Err(_) => Payload::Other(data),
    }
}

impl Payload {
    /// Decode raw event `data` into the typed payload for a topic `channel`.
    ///
    /// Never fails: data that does not match the channel's typed shape is
    /// preserved as [`Payload::Other`].
    pub(crate) fn from_channel(channel: Channel, data: serde_json::Value) -> Self {
        match channel {
            Channel::Trading => typed_payload(data, Payload::Trading),
            Channel::Fees => typed_payload(data, Payload::Fees),
            Channel::Oracle => typed_payload(data, Payload::Oracle),
            Channel::Resolution => typed_payload(data, Payload::Resolution),
            Channel::Lifecycle => typed_payload(data, Payload::Lifecycle),
            Channel::Positions => typed_payload(data, Payload::Positions),
            Channel::Combos => typed_payload(data, Payload::Combos),
            Channel::Transfers => typed_payload(data, Payload::Transfers),
            Channel::Accounts => typed_payload(data, Payload::Accounts),
            // Filtered views re-emit confirmed payloads; the untagged enum picks
            // the variant whose `type` matches, preserving unknowns.
            Channel::Wallets | Channel::Markets => {
                serde_json::from_value(data.clone()).unwrap_or(Payload::Other(data))
            }
        }
    }

    /// Decode raw event `data` into the typed payload for a CLOB `channel`.
    ///
    /// Never fails: data that does not match the channel's fixed shape is
    /// preserved as [`Payload::Other`].
    pub(crate) fn from_clob_channel(channel: ClobChannel, data: serde_json::Value) -> Self {
        match channel {
            ClobChannel::Book => typed_payload(data, Payload::ClobBook),
            ClobChannel::Prices => typed_payload(data, Payload::ClobPrices),
            ClobChannel::LastTrade => typed_payload(data, Payload::ClobLastTrade),
            ClobChannel::Midpoint => typed_payload(data, Payload::ClobMidpoint),
            ClobChannel::TickSize => typed_payload(data, Payload::ClobTickSize),
            ClobChannel::BestBidAsk => typed_payload(data, Payload::ClobBestBidAsk),
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

    #[test]
    fn clob_book_types_a_snapshot() {
        let data = json!({
            "asset_id": "123",
            "market": "0xabc",
            "timestamp": 1_700_000_000_i64,
            "bids": [{"price": 0.4, "size": 100.0}],
            "asks": [{"price": 0.6, "size": 50.0}],
        });
        match Payload::from_clob_channel(ClobChannel::Book, data) {
            Payload::ClobBook(book) => {
                assert_eq!(book.asset_id, "123");
                assert_eq!(book.market, "0xabc");
                assert_eq!(book.bids.len(), 1);
                assert_eq!(book.bids[0].price, 0.4);
                assert_eq!(book.asks[0].size, 50.0);
            }
            other => panic!("expected clob book, got {other:?}"),
        }
    }

    #[test]
    fn clob_last_trade_size_is_optional() {
        let data = json!({"asset_id":"9","market":"0xm","price":0.55,"timestamp":42});
        match Payload::from_clob_channel(ClobChannel::LastTrade, data) {
            Payload::ClobLastTrade(trade) => {
                assert_eq!(trade.price, 0.55);
                assert!(trade.size.is_none());
                assert_eq!(trade.timestamp, 42);
            }
            other => panic!("expected clob last_trade, got {other:?}"),
        }
    }

    #[test]
    fn clob_payload_mismatch_falls_back_to_other() {
        // A book payload is missing its required `bids`/`asks`.
        let data = json!({"asset_id":"1","market":"0xm","timestamp":1});
        assert!(matches!(
            Payload::from_clob_channel(ClobChannel::Book, data),
            Payload::Other(_)
        ));
    }
}
