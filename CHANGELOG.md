# Changelog

All notable changes to `radion-sdk` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and the project adheres to
[Semantic Versioning](https://semver.org/).

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
  - Fully typed channel `Payload` enum (trades, oracle, lifecycle, activity,
    collateral, combos, prices) with forward-compatible `Other` fallback.
  - Auto-reconnect with exponential backoff + jitter, subscription replay, and
    heartbeat / stale-connection detection.
  - Events delivered as typed `Stream`s.
- Cargo features: `realtime` (default), `rustls` (default), `native-tls`,
  `tracing`.
