//! Consume Radion webhook deliveries.
//!
//! Webhooks POST the same event frame the WebSocket sends, signed with the
//! endpoint's secret. [`WebhookDelivery::verify`] authenticates a delivery;
//! [`parse_webhook_event`] validates the body into a typed [`WebhookEvent`].
//! Verify before you trust the body, then parse.
//!
//! # Example
//!
//! ```no_run
//! use radion_sdk::webhooks::{WebhookDelivery, parse_webhook_event};
//!
//! # fn handle(raw_body: &[u8], signature: &str, timestamp: &str, secret: &str) {
//! let delivery = WebhookDelivery {
//!     payload: raw_body,
//!     signature,
//!     timestamp,
//! };
//! if delivery.verify(&[secret]) {
//!     let raw = std::str::from_utf8(raw_body).unwrap_or_default();
//!     if let Some(event) = parse_webhook_event(raw) {
//!         println!("{} seq={} {:?}", event.channel, event.seq, event.data);
//!     }
//! }
//! # }
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::realtime::ChannelEvent;
use crate::realtime::protocol::parse_inbound_frame;

/// Default replay tolerance for [`WebhookDelivery::verify`]: five minutes.
pub const DEFAULT_WEBHOOK_TOLERANCE_MS: u64 = 5 * 60 * 1000;

const SIGNATURE_PREFIX: &str = "v1=";

/// The body of a webhook delivery.
///
/// Webhooks deliver the same event frame the WebSocket sends, so this is an
/// alias of [`ChannelEvent`]. On a webhook, `id` is the index of the matching
/// subscription in the endpoint's `subscriptions` array (as a string), and
/// `seq` counts deliveries per endpoint — it resets when the endpoint is
/// edited or its secret rotated.
pub type WebhookEvent = ChannelEvent;

/// A webhook delivery to authenticate: the raw request as received.
///
/// `payload` must be the raw body bytes exactly as received, before any JSON
/// parsing — re-serializing a parsed body can change the bytes and break the
/// signature.
#[derive(Debug, Clone, Copy)]
pub struct WebhookDelivery<'a> {
    /// Raw request body, exactly as received.
    pub payload: &'a [u8],
    /// The `X-Radion-Signature` header value (`v1=` + hex HMAC-SHA256 digest).
    pub signature: &'a str,
    /// The `X-Radion-Timestamp` header value (Unix time in milliseconds).
    pub timestamp: &'a str,
}

impl WebhookDelivery<'_> {
    /// Verify the delivery's HMAC-SHA256 signature with the default replay
    /// tolerance ([`DEFAULT_WEBHOOK_TOLERANCE_MS`]).
    ///
    /// Checks that the delivery is fresh (`timestamp` within the tolerance of
    /// now, blocking replays of captured requests) and that `signature`
    /// matches `HMAC-SHA256(secret, "{timestamp}.{body}")` for at least one
    /// of `secrets` — pass both during a secret-rotation window. The
    /// comparison is constant-time. Returns `true` only for a fresh,
    /// correctly signed delivery; then parse the body with
    /// [`parse_webhook_event`].
    #[must_use]
    pub fn verify(&self, secrets: &[&str]) -> bool {
        self.verify_with_tolerance(secrets, DEFAULT_WEBHOOK_TOLERANCE_MS)
    }

    /// [`verify`](Self::verify) with a custom replay tolerance in milliseconds.
    #[must_use]
    pub fn verify_with_tolerance(&self, secrets: &[&str], tolerance_ms: u64) -> bool {
        let Ok(timestamp_ms) = self.timestamp.parse::<u64>() else {
            return false;
        };
        if now_ms().abs_diff(timestamp_ms) > tolerance_ms {
            return false;
        }
        let Some(digest) = decode_signature(self.signature) else {
            return false;
        };
        secrets
            .iter()
            .any(|secret| self.matches_secret(secret, &digest))
    }

    fn matches_secret(&self, secret: &str, digest: &[u8]) -> bool {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("hmac-sha256 accepts keys of any length");
        mac.update(self.timestamp.as_bytes());
        mac.update(b".");
        mac.update(self.payload);
        mac.verify_slice(digest).is_ok()
    }
}

/// Parse and validate a raw webhook request body into a typed [`WebhookEvent`].
///
/// Returns `None` when the body is not valid JSON or does not match the event
/// envelope, so callers can reject malformed requests without erroring.
/// Parsing does not authenticate the request — verify the signature first
/// with [`WebhookDelivery::verify`].
#[must_use]
pub fn parse_webhook_event(raw: &str) -> Option<WebhookEvent> {
    parse_inbound_frame(raw)?.into_channel_event()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

fn decode_signature(signature: &str) -> Option<Vec<u8>> {
    let hex_digest = signature.strip_prefix(SIGNATURE_PREFIX)?;
    if hex_digest.is_empty() || hex_digest.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex_digest.len() / 2);
    for index in (0..hex_digest.len()).step_by(2) {
        let byte = u8::from_str_radix(hex_digest.get(index..index + 2)?, 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::realtime::Payload;

    const SECRET: &str = "whsec_0123456789abcdef0123456789abcdef";
    const BODY: &str = r#"{"channel":"trading","type":"event"}"#;

    fn sign(secret: &str, timestamp_ms: u64, body: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .expect("hmac-sha256 accepts keys of any length");
        mac.update(timestamp_ms.to_string().as_bytes());
        mac.update(b".");
        mac.update(body.as_bytes());
        let digest = mac.finalize().into_bytes();
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        format!("v1={hex}")
    }

    fn delivery<'a>(timestamp: &'a str, signature: &'a str) -> WebhookDelivery<'a> {
        WebhookDelivery {
            payload: BODY.as_bytes(),
            signature,
            timestamp,
        }
    }

    #[test]
    fn accepts_a_fresh_correctly_signed_delivery() {
        let timestamp_ms = now_ms();
        let signature = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        assert!(delivery(&timestamp, &signature).verify(&[SECRET]));
    }

    #[test]
    fn rejects_a_tampered_body() {
        let timestamp_ms = now_ms();
        let signature = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        let tampered = WebhookDelivery {
            payload: b"{}",
            signature: &signature,
            timestamp: &timestamp,
        };
        assert!(!tampered.verify(&[SECRET]));
    }

    #[test]
    fn rejects_a_signature_made_with_another_secret() {
        let timestamp_ms = now_ms();
        let signature = sign("whsec_other", timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        assert!(!delivery(&timestamp, &signature).verify(&[SECRET]));
    }

    #[test]
    fn accepts_when_any_rotation_secret_matches() {
        let timestamp_ms = now_ms();
        let signature = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        assert!(delivery(&timestamp, &signature).verify(&["whsec_old", SECRET]));
    }

    #[test]
    fn rejects_a_replayed_delivery_older_than_the_tolerance() {
        let timestamp_ms = now_ms() - DEFAULT_WEBHOOK_TOLERANCE_MS - 1_000;
        let signature = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        assert!(!delivery(&timestamp, &signature).verify(&[SECRET]));
    }

    #[test]
    fn honours_a_custom_tolerance() {
        let timestamp_ms = now_ms() - 5_000;
        let signature = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        assert!(!delivery(&timestamp, &signature).verify_with_tolerance(&[SECRET], 1_000));
        assert!(delivery(&timestamp, &signature).verify_with_tolerance(&[SECRET], 60_000));
    }

    #[test]
    fn rejects_malformed_signature_values() {
        let timestamp_ms = now_ms();
        let valid = sign(SECRET, timestamp_ms, BODY);
        let timestamp = timestamp_ms.to_string();
        let digest_only = valid.trim_start_matches(SIGNATURE_PREFIX).to_owned();
        for signature in [digest_only.as_str(), "v1=nothex", ""] {
            assert!(!delivery(&timestamp, signature).verify(&[SECRET]));
        }
    }

    #[test]
    fn rejects_a_non_numeric_timestamp() {
        let signature = sign(SECRET, now_ms(), BODY);
        assert!(!delivery("not-a-timestamp", &signature).verify(&[SECRET]));
    }

    #[test]
    fn parses_a_delivery_body_into_a_typed_event() {
        let raw = r#"{"type":"event","id":"0","channel":"trading","confirmed":true,"seq":7,"sent_at_ms":1721818200123,"data":{"type":"order_filled_v2","maker":"0xmaker"}}"#;
        let event = parse_webhook_event(raw).expect("valid delivery");
        assert_eq!(event.channel, "trading");
        assert_eq!(event.id, "0");
        assert_eq!(event.seq, 7);
        assert_eq!(event.sent_at_ms, 1_721_818_200_123);
        assert!(matches!(event.data, Payload::Trading(_)));
    }

    #[test]
    fn parse_returns_none_for_invalid_json() {
        assert!(parse_webhook_event("{not json").is_none());
    }

    #[test]
    fn parse_returns_none_for_a_non_event_frame() {
        assert!(parse_webhook_event(r#"{"type":"pong"}"#).is_none());
    }
}
