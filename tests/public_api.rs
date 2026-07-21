//! Integration tests exercising the public API surface (no network).

use radion_sdk::realtime::{Channel, ChannelFilters, ClobChannel, Subscription};
use radion_sdk::{Radion, RadionError};

#[test]
fn builder_requires_api_key() {
    let err = Radion::builder().build().unwrap_err();
    assert!(matches!(err, RadionError::Connection(_)));
}

#[test]
fn builder_applies_overrides() {
    let radion = Radion::builder()
        .api_key("sk_test")
        .ws_url("wss://example.test/ws")
        .build()
        .expect("builds");
    assert_eq!(radion.config.ws_url, "wss://example.test/ws");
    assert!(!radion.realtime.connected());
}

#[test]
fn subscription_filter_validation_is_public() {
    // Missing required filter is rejected before any I/O.
    assert!(Subscription::new("w", Channel::Wallets).validate().is_err());
    assert!(
        Subscription::new("w", Channel::Wallets)
            .with_filters(ChannelFilters {
                wallets: Some(vec!["0x1".into()]),
                ..Default::default()
            })
            .validate()
            .is_ok()
    );
}

#[test]
fn clob_channels_are_first_class_and_require_token_ids() {
    // A clob channel is a first-class subscribable channel requiring token_ids.
    assert!(
        Subscription::new("book", ClobChannel::Book)
            .validate()
            .is_err()
    );
    assert!(
        Subscription::new("book", ClobChannel::Book)
            .with_filters(ChannelFilters {
                token_ids: Some(vec!["1".into()]),
                ..Default::default()
            })
            .validate()
            .is_ok()
    );
}

#[cfg(feature = "compression")]
#[test]
fn compression_is_opt_in_on_both_builders() {
    use radion_sdk::realtime::RealtimeOptions;

    assert!(!RealtimeOptions::new("sk_test").compression);
    assert!(
        RealtimeOptions::new("sk_test")
            .compression(true)
            .compression
    );
    assert!(
        Radion::builder()
            .api_key("sk_test")
            .compression(true)
            .build()
            .is_ok()
    );
}
