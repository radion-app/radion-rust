# Radion Rust SDK

[![crates.io](https://img.shields.io/crates/v/radion-sdk.svg)](https://crates.io/crates/radion-sdk)
[![docs.rs](https://img.shields.io/docsrs/radion-sdk)](https://docs.rs/radion-sdk)
[![license](https://img.shields.io/crates/l/radion-sdk.svg)](./LICENSE)

Official, async-first, fully-typed Rust SDK for [Radion](https://radion.app).
One client, one API key.

```rust,no_run
use futures_util::StreamExt;
use radion_sdk::realtime::{Channel, Payload, Subscription};
use radion_sdk::Radion;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let radion = Radion::builder().api_key(std::env::var("RADION_API_KEY")?).build()?;
    radion.realtime.connect().await?;

    let mut trades = radion
        .realtime
        .subscribe(Subscription::new("trading", Channel::Trading))
        .await?;

    while let Some(event) = trades.next().await {
        if let Payload::Trading(trade) = event.data {
            println!("{} {:?}", event.id, trade.kind);
        }
    }
    Ok(())
}
```

## Features

- **Unified client** — one `Radion`, one API key; product surfaces attach as fields.
- **Auto-reconnect** — exponential backoff with jitter after unexpected drops.
- **Subscription restore** — subscriptions are replayed automatically on reconnect.
- **Heartbeats** — periodic pings with stale-connection detection.
- **Typed end-to-end** — every frame, channel, filter, and payload is statically
  typed; events stream as a typed `Payload` enum you can `match` exhaustively.
- **Async-first** — built on `tokio`; events are a [`Stream`](https://docs.rs/futures).

## Requirements

Rust 1.85 or later.

## Install

```sh
cargo add radion-sdk
```

Cargo features:

| Feature | Default | Description |
| --- | --- | --- |
| `realtime` | ✅ | The WebSocket product surface. |
| `rustls` | ✅ | rustls TLS backend (no system OpenSSL). |
| `native-tls` | | Use the platform native TLS backend instead. |
| `tracing` | | Emit [`tracing`](https://docs.rs/tracing) spans/events. |

To drop the realtime transport (e.g. for a future REST-only build):

```sh
cargo add radion-sdk --no-default-features
```

## Usage

### Configuration

Build a client with [`Radion::builder`]:

| Method | Description |
| --- | --- |
| `.api_key(key)` | Radion API key (required), sent as `X-API-Key`. |
| `.base_url(url)` | Override the REST base URL. Defaults to `https://api.radion.app`. |
| `.ws_url(url)` | Override the realtime endpoint. Defaults to `wss://api.radion.app/ws`. |

```rust,no_run
let radion = radion_sdk::Radion::builder().api_key("sk_...").build()?;
# Ok::<(), radion_sdk::RadionError>(())
```

### Realtime client

Reached as `radion.realtime`, or constructed standalone with
`RealtimeClient::new(RealtimeOptions::new(api_key))`.

| Method | Description |
| --- | --- |
| `connect()` | Open the connection; resolves once established. |
| `subscribe(sub)` | Subscribe and return a `Stream` of that subscription's events. |
| `unsubscribe(id)` | Drop a subscription by id. |
| `events()` | Firehose `Stream` across all subscriptions. |
| `lifecycle()` | `Stream` of connection lifecycle events. |
| `close(code, reason)` | Gracefully close; stops reconnecting. |
| `connected()` | Whether the socket is currently open. |

### Subscriptions & filters

`Subscription::new(id, channel)` takes a client-defined id (echoed back on every
event) and a channel. Use `mempool.`-prefixed channels for speculative pending
transactions: `"mempool.trading".parse::<SubscribableChannel>()?`.

Attach server-side filters with `.with_filters(...)`. Some channels require a
filter — `wallets` needs `wallets`, `markets` needs `market_ids` or `token_ids`.
Requirements are validated before any frame is sent.

```rust,no_run
use radion_sdk::realtime::{Channel, ChannelFilters, Subscription};

let sub = Subscription::new("whales", Channel::Wallets).with_filters(ChannelFilters {
    wallets: Some(vec!["0xabc...".into()]),
    ..Default::default()
});
```

### Channels

`Channel` enumerates every channel. Nine topic channels each carry a typed
payload — `Trading`, `Fees`, `Oracle`, `Resolution`, `Lifecycle`, `Positions`,
`Combos`, `Transfers`, `Accounts` — plus two cross-cutting filter channels,
`Wallets` and `Markets`, that re-emit the matching topic payload. `CHANNELS` is
the full array. Each channel's event `data` is the typed `Payload` enum — `match`
on it for compile-time exhaustiveness. Unknown channels or event types are
preserved as `Payload::Other(serde_json::Value)`.

Filter high-volume order flow by size with `min_usd` on `Trading` (there is no
separate large-trades channel). Every topic channel also has a `mempool.`-prefixed
companion for speculative pending transactions.

### CLOB channels

The CLOB (central limit order book) family is a first-class, separate set of
subscribable channels. `ClobChannel` enumerates the six — `Book`, `Prices`,
`LastTrade`, `Midpoint`, `TickSize`, `BestBidAsk` (wire names `clob.book`,
`clob.prices`, `clob.last_trade`, `clob.midpoint`, `clob.tick_size`,
`clob.best_bid_ask`) — and `CLOB_CHANNELS` is the full array. Unlike topic
channels, each CLOB channel **requires** a `token_ids` filter and has **no**
`mempool.` companion. Each carries one fixed payload (`Payload::ClobBook`,
`ClobPrices`, `ClobLastTrade`, `ClobMidpoint`, `ClobTickSize`, `ClobBestBidAsk`)
with no event `type` discriminator.

```rust,no_run
use radion_sdk::realtime::{ChannelFilters, ClobChannel, Subscription};

let book = Subscription::new("book", ClobChannel::Book).with_filters(ChannelFilters {
    token_ids: Some(vec!["71321...".into()]),
    ..Default::default()
});
```

### Lifecycle events

```rust,no_run
use futures_util::StreamExt;
use radion_sdk::realtime::LifecycleEvent;
# async fn run(radion: radion_sdk::Radion) {
let mut lifecycle = radion.realtime.lifecycle();
while let Some(event) = lifecycle.next().await {
    match event {
        LifecycleEvent::Open => {}
        LifecycleEvent::Close { code, reason } => {}
        LifecycleEvent::Reconnect { attempt, delay } => {}
        LifecycleEvent::Error(err) => eprintln!("{err}"),
        _ => {}
    }
}
# }
```

### Reconnect & subscription restore

By default the client reconnects with exponential backoff (500ms initial, ×2,
30s max, 0.2 jitter) and replays every active subscription on reconnect. Tune via
`RealtimeOptions::reconnect(...)` or disable with `.disable_reconnect()`.

### Heartbeats

A ping is sent every 15s; if no inbound traffic arrives within 10s the connection
is treated as stale, torn down, and reconnected. Tune via
`RealtimeOptions::heartbeat(...)` or disable with `.disable_heartbeat()`.

### Error handling

`RadionError` covers connection-lifecycle misuse (`Connection`), server-reported
`error` frames (`Server`), and transport failures (`Transport`). Server and
transport errors during a live connection are surfaced on the `lifecycle()`
stream as `LifecycleEvent::Error`.

## License

MIT — see [LICENSE](./LICENSE).
