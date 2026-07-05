# Changelog

All notable changes to `radion-sdk` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

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
