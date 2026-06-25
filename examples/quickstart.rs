//! Connect, subscribe to trades, and print typed events.
//!
//! ```sh
//! RADION_API_KEY=rk_... cargo run --example quickstart
//! ```

use futures_util::StreamExt;
use radion_sdk::Radion;
use radion_sdk::realtime::{Channel, Payload, Subscription};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("RADION_API_KEY").expect("set RADION_API_KEY");
    let radion = Radion::builder().api_key(api_key).build()?;

    radion.realtime.connect().await?;

    let mut trades = radion
        .realtime
        .subscribe(Subscription::new("trades", Channel::Trades))
        .await?;

    while let Some(event) = trades.next().await {
        match event.data {
            Payload::Trades(trade) => println!("{} {:?}", event.id, trade.kind),
            other => println!("{} {other:?}", event.id),
        }
    }

    Ok(())
}
