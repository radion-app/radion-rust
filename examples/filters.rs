//! Subscribe with server-side filters.
//!
//! ```sh
//! RADION_API_KEY=rk_... cargo run --example filters
//! ```

use futures_util::StreamExt;
use radion_sdk::Radion;
use radion_sdk::realtime::{Channel, ChannelFilters, Subscription};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("RADION_API_KEY").expect("set RADION_API_KEY");
    let radion = Radion::builder().api_key(api_key).build()?;
    radion.realtime.connect().await?;

    // `wallets` requires a `wallets` filter; validated before sending.
    let subscription =
        Subscription::new("my-wallets", Channel::Wallets).with_filters(ChannelFilters {
            wallets: Some(vec!["0xabc...".to_string()]),
            ..Default::default()
        });

    let mut stream = radion.realtime.subscribe(subscription).await?;
    while let Some(event) = stream.next().await {
        println!("{} {:?}", event.channel, event.data);
    }

    Ok(())
}
