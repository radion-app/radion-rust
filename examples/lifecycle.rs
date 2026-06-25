//! Observe connection lifecycle events alongside channel data.
//!
//! ```sh
//! RADION_API_KEY=rk_... cargo run --example lifecycle
//! ```

use futures_util::StreamExt;
use radion_sdk::Radion;
use radion_sdk::realtime::{Channel, LifecycleEvent, Subscription};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("RADION_API_KEY").expect("set RADION_API_KEY");
    let radion = Radion::builder().api_key(api_key).build()?;

    // Watch lifecycle events on a background task.
    let mut lifecycle = radion.realtime.lifecycle();
    tokio::spawn(async move {
        while let Some(event) = lifecycle.next().await {
            match event {
                LifecycleEvent::Open => println!("open"),
                LifecycleEvent::Close { code, reason } => println!("close {code} {reason}"),
                LifecycleEvent::Reconnect { attempt, delay } => {
                    println!("reconnect #{attempt} in {delay:?}");
                }
                LifecycleEvent::Error(error) => eprintln!("error: {error}"),
                _ => {}
            }
        }
    });

    radion.realtime.connect().await?;
    let mut events = radion
        .realtime
        .subscribe(Subscription::new("global", Channel::Global))
        .await?;
    while let Some(event) = events.next().await {
        println!("{}: {:?}", event.channel, event.data);
    }

    Ok(())
}
