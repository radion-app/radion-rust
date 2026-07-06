# Changelog

All notable changes to `radion-sdk` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

## [0.6.0] - 2026-07-06

### Changed

- **BREAKING: pending feed is now a flag, not a channel prefix.** The
  `mempool.` channel prefix and `SubscribableChannel::Mempool` are gone.
  Subscribe to the pending feed with a `confirmed` flag on the `Subscription`:
  `Subscription::new(id, Channel::Trading).pending()` (or `.confirmed(false)`).
  Subscriptions default to the confirmed feed (`confirmed = true`). The subscribe
  frame now carries an optional `confirmed` field (default `true`); CLOB channels
  omit it, as they have no pending feed. `SubscribableChannel` is now
  `Topic(Channel)` / `Clob(ClobChannel)`, its `confirmed()` accessor is renamed
  `topic()`, and `is_mempool()` is removed.
- **BREAKING: unified event frame with envelope `confirmed`.** Confirmed and
  pending events share one bare channel name; the feed is told apart by a
  `confirmed` bool on the envelope, now surfaced as `ChannelEvent.confirmed`.
  Route events by subscription `id`, not by a `mempool.`-prefixed channel string.
- **BREAKING: pending payload dropped its inner `confirmed` field** (moved to the
  envelope) and its trade `call.usd` is renamed `call.notional_usd`. Pending
  transactions decode to the new `Payload::Mempool(MempoolPayload)`, whose `call`
  (`MempoolCall`) now also carries an un-collapsed `orders` list
  (`Vec<MempoolOrder>`, each with `maker` / `taker` / `token_id` / `side` /
  `maker_amount` / `taker_amount`; `side` is the new `OrderSide` enum). `orders`
  is empty for non-trade (positions / combos) calls.

### Added

- **`LifecycleEvent::Warning { code, id, message }`** for the new server
  `warning` frame — for example `mempool_unavailable`, sent after a pending
  subscribe when the node has no pending stream. It is non-fatal; delivery
  continues.

## [0.5.0] - 2026-07-05

### Added

- **Public JWT auth flow.** `.token("...")` for a static user JWT or
  `.token_provider(TokenProvider::new(...))` for a refreshable one, alongside a
  `pk_jwt_` API key. The provider is called on every (re)connect, so tokens
  never go stale.
- **`.auth_in_query(true)`** to send credentials in the WS URL query string
  instead of headers (proxies / gateways).

## [0.4.3] - 2026-07-05

### Changed

- **`Payload::Combos` and `Payload::Lifecycle` now box their inner payload**
  (`Box<CombosPayload>` / `Box<LifecyclePayload>`) to shrink the `Payload` enum,
  which the largest variants had inflated to 944 bytes. Callers matching these
  variants must dereference the boxed value.

## [0.4.2] - 2026-07-05

### Fixed

- **Realtime payload structs now decode every field each channel's events
  carry.** Rust drops undeclared fields, so incomplete structs silently lost
  data; the channel docs now enumerate all 77 confirmed events and each struct
  is the union of its events' fields. Corrected wrong fields: `FeesPayload` now
  has `receiver` / `token_id` / `amount` (dropped `fee` / `payer` / `order_hash`);
  `TransfersPayload` uses `amount` / `amounts` (was `value` / `values`);
  `ResolutionPayload` uses `payout_numerators` / `result` (dropped `payouts`);
  dropped the stray `id` / `operator` from `CombosPayload` and `token_id` from
  `PositionsPayload`. Added the many previously-missing fields. All fields are
  `Option`, so this is non-breaking.

## [0.4.1] - 2026-07-05

### Added

- **CLOB channels are now a first-class subscribable family.** `ClobChannel`
  (`Book`, `Prices`, `LastTrade`, `Midpoint`, `TickSize`, `BestBidAsk`; wire
  names `clob.book`, `clob.prices`, `clob.last_trade`, `clob.midpoint`,
  `clob.tick_size`, `clob.best_bid_ask`) and the `CLOB_CHANNELS` array join
  `SubscribableChannel` as a `Clob` variant. Each CLOB channel requires a
  `token_ids` filter and has no `mempool.` companion. Added the typed payloads
  `ClobBookPayload` (with `Level`), `ClobPricesPayload` (with `PriceChange`),
  `ClobLastTradePayload`, `ClobMidpointPayload`, `ClobTickSizePayload`, and
  `ClobBestBidAskPayload`, surfaced as `Payload::Clob*` variants (no event
  `type` discriminator).

## [0.3.0] - 2026-07-04

### Changed

- **BREAKING: realtime channel taxonomy redesign.** The confirmed channel set is
  now `trading`, `fees`, `oracle`, `resolution`, `lifecycle`, `positions`,
  `combos`, `transfers`, `accounts` (nine typed topic channels) plus the
  cross-cutting `wallets` and `markets` filter channels.
  - Renamed `trades` → `trading`; exchange fees moved out of it into the new
    `fees` channel. The `Payload::Trades`/`TradesPayload`/`TradeEventType` types
    are now `Payload::Trading`/`TradingPayload`/`TradingEventType`.
  - Added `fees`, `resolution`, `transfers`, `accounts`, and `positions`
    channels with their own typed payloads and event-`type` unions.

### Removed

- **BREAKING:** the `global` firehose channel (unrepresentable — subscribe to the
  specific channels you need).
- **BREAKING:** the `activity` channel — its events are split across the new
  `positions` and `combos` channels.
- **BREAKING:** the `large_trades` channel — subscribe to `trading` with a
  `min_usd` filter instead.
- **BREAKING:** the derived `prices` (last-trade tick) channel and its
  `PricesPayload`. (The unrelated CLOB `clob.prices` channel is unaffected.)
- **BREAKING:** the `collateral` channel — its role is covered by `positions`.

## [0.2.1] - 2026-06-25

Initial release of the Radion Rust SDK, at feature parity with the TypeScript
(`@radion-app/sdk`) and Python (`radion-sdk`) SDKs at v0.2.x.

### Added

- Unified `Radion` client with a builder and an `X-API-Key` auth scheme.
- `realtime` (WebSocket) product surface behind a default cargo feature:
  - `RealtimeClient` with `connect`, `subscribe`, `unsubscribe`, `events`,
    `lifecycle`, `close`, and `connected`.
  - Typed `Channel`, `SubscribableChannel` (incl. `mempool.` companions),
    `ChannelFilters` with per-channel required-filter validation.
  - Fully typed per-channel `Payload` enum with forward-compatible `Other`
    fallback.
  - Auto-reconnect with exponential backoff + jitter, subscription replay, and
    heartbeat / stale-connection detection.
  - Events delivered as typed `Stream`s.
- Cargo features: `realtime` (default), `rustls` (default), `native-tls`,
  `tracing`.
