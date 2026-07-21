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
async fn main() -> eyre::Result<()> {
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
- **Webhook helpers** — verify delivery signatures and parse bodies into the same typed
  `Payload` events (`webhooks` feature, no async transport).

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
| `compression` | | Inflate zlib-compressed realtime frames. Opt in per client. |
| `tracing` | | Emit [`tracing`](https://docs.rs/tracing) spans/events. |
| `webhooks` | | Webhook helpers: signature verification + typed body parsing. No async transport. |

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
| `.token(jwt)` / `.token_provider(p)` | User JWT for the `pk_jwt_` flow (see below). |
| `.auth_in_query(true)` | Send credentials in the WS URL query string. |
| `.compression(true)` | Ask for zlib-compressed realtime frames (`compression` feature). |

```rust,no_run
let radion = radion_sdk::Radion::builder().api_key("sk_...").build()?;
# Ok::<(), radion_sdk::RadionError>(())
```

### Authentication

Two credential schemes, both keyed on the API key (sent as `X-API-Key`):

- **Secret key** (`sk_` / `rk_`): `.api_key("sk_...")`.
- **Public JWT** (`pk_jwt_`): pair the public key with a per-user JWT. Use
  `.token_provider(...)` so a fresh JWT is fetched on every (re)connect:

```rust,no_run
use radion_sdk::realtime::TokenProvider;

let radion = radion_sdk::Radion::builder()
    .api_key("pk_jwt_...")
    .token_provider(TokenProvider::new(|| async { Ok(fetch_user_jwt().await?) }))
    .build()?;
# Ok::<(), radion_sdk::RadionError>(())
# async fn fetch_user_jwt() -> radion_sdk::error::Result<String> { Ok(String::new()) }
```

Use `.auth_in_query(true)` to move credentials into the WS URL query string —
for a proxy or gateway that strips the `X-API-Key` / `Authorization` headers.

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
event) and a channel. Subscriptions default to the confirmed on-chain feed; call
`.pending()` (or `.confirmed(false)`) for the speculative pending (mempool) feed:
`Subscription::new("pending", Channel::Trading).pending()`. Pending events arrive
on the same bare channel name and are told apart by `ChannelEvent.confirmed`;
their `data` is a decoded transaction (`Payload::Mempool`).

Every event also carries two envelope fields: `seq`, a per-connection monotonic
counter (+1 per event across all subscriptions — a jump means frames were
dropped), and `sent_at_ms`, the Unix-millisecond server send time, so
server→client latency is your receive time minus `sent_at_ms`. Pending events
keep `data.seen_at_ms` (block-detection time) for block→client latency.

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
separate large-trades channel). On the confirmed feed `min_usd` is the actual
filled USD; on the pending feed it is the intended fill notional
(`call.notional_usd`). Every topic channel also has a pending (mempool) feed,
selected with `.pending()` on the subscription.

### CLOB channels

The CLOB (central limit order book) family is a first-class, separate set of
subscribable channels. `ClobChannel` enumerates the six — `Book`, `Prices`,
`LastTrade`, `Midpoint`, `TickSize`, `BestBidAsk` (wire names `clob.book`,
`clob.prices`, `clob.last_trade`, `clob.midpoint`, `clob.tick_size`,
`clob.best_bid_ask`) — and `CLOB_CHANNELS` is the full array. Unlike topic
channels, each CLOB channel **requires** a `token_ids` filter and has **no**
pending feed. Each carries one fixed payload (`Payload::ClobBook`,
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
        LifecycleEvent::Warning { code, message, .. } => eprintln!("warning {code}: {message}"),
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

### Compression

Enable the `compression` feature and call `.compression(true)` to cut bandwidth
on high-volume channels:

```sh
cargo add radion-sdk --features compression
```

```rust,no_run
let radion = radion_sdk::Radion::builder()
    .api_key("sk_...")
    .compression(true)
    .build()?;
# Ok::<(), radion_sdk::RadionError>(())
```

This adds `compress=zlib` to the connect URL. The server then sends event frames
as binary zlib (RFC 1950), which the client inflates before parsing. Text frames
still work, so a server may mix both on one connection. Pings and pongs are
unaffected. A frame that fails to inflate is reported on the `lifecycle()` stream
as `LifecycleEvent::Error(RadionError::Decompression(_))` — it is never dropped
in silence.

Compression is off by default: it trades CPU for bandwidth, which only pays off
on busy subscriptions.

### Webhooks

Radion webhooks POST the same event frames as the WebSocket, signed with your
endpoint's secret. Enable the `webhooks` feature for standalone helpers — no
client and no async transport, so they fit any HTTP server:

```sh
cargo add radion-sdk --features webhooks
```

```rust,no_run
use radion_sdk::webhooks::{WebhookDelivery, parse_webhook_event};

fn handle(raw_body: &[u8], signature: &str, timestamp: &str, secret: &str) {
    let delivery = WebhookDelivery {
        payload: raw_body,
        signature,
        timestamp,
    };
    if !delivery.verify(&[secret]) {
        return;
    }
    let raw = std::str::from_utf8(raw_body).unwrap_or_default();
    if let Some(event) = parse_webhook_event(raw) {
        println!("{} seq={} {:?}", event.channel, event.seq, event.data);
    }
}
```

`payload` is the raw request body exactly as received, `signature` the
`X-Radion-Signature` header, and `timestamp` the `X-Radion-Timestamp` header.
`verify` runs a constant-time HMAC-SHA256 check and rejects deliveries older
than five minutes — tune the replay window with `verify_with_tolerance`, and
pass both secrets during a rotation window. `parse_webhook_event` returns the
same typed `ChannelEvent` the realtime client streams. Deliveries are retried
and unordered, so deduplicate on `(id, seq)` or on fields of `data`.

### Error handling

`RadionError` covers connection-lifecycle misuse (`Connection`), server-reported
`error` frames (`Server`), transport failures (`Transport`), and frames that
fail to inflate (`Decompression`). Errors during a live connection are surfaced
on the `lifecycle()` stream as `LifecycleEvent::Error`.

## License

MIT — see [LICENSE](./LICENSE).
