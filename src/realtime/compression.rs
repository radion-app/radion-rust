//! Zlib decompression for binary realtime frames.

use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::{RadionError, Result};

/// Query parameter asking the server to send frames as zlib-compressed binary.
const COMPRESS_QUERY: &str = "compress=zlib";

/// Append `compress=zlib` to a WebSocket URL, preserving any existing query.
pub(crate) fn with_compress_query(url: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{COMPRESS_QUERY}")
}

/// Inflate a zlib (RFC 1950) frame into the JSON text it carries.
///
/// # Errors
///
/// Returns [`RadionError::Decompression`] if the bytes are not valid zlib or do
/// not inflate to UTF-8.
pub(crate) fn inflate(bytes: &[u8]) -> Result<String> {
    let mut text = String::new();
    ZlibDecoder::new(bytes)
        .read_to_string(&mut text)
        .map_err(RadionError::decompression)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    use super::*;

    fn deflate(text: &str) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(text.as_bytes()).expect("writes");
        encoder.finish().expect("finishes")
    }

    #[test]
    fn query_is_appended_to_a_bare_url() {
        assert_eq!(
            with_compress_query("wss://api.radion.app/ws"),
            "wss://api.radion.app/ws?compress=zlib"
        );
    }

    #[test]
    fn query_preserves_an_existing_query() {
        assert_eq!(
            with_compress_query("wss://api.radion.app/ws?api-key=k1"),
            "wss://api.radion.app/ws?api-key=k1&compress=zlib"
        );
    }

    #[test]
    fn inflates_a_zlib_frame() {
        let raw = r#"{"type":"pong"}"#;
        assert_eq!(inflate(&deflate(raw)).unwrap(), raw);
    }

    #[test]
    fn inflates_a_frame_larger_than_one_read() {
        let raw = format!(r#"{{"type":"event","data":"{}"}}"#, "x".repeat(100_000));
        assert_eq!(inflate(&deflate(&raw)).unwrap(), raw);
    }

    #[test]
    fn rejects_bytes_that_are_not_zlib() {
        // Raw deflate has no RFC 1950 header, so it must not be accepted.
        let err = inflate(b"not zlib at all").unwrap_err();
        assert!(matches!(err, RadionError::Decompression(_)));
    }

    #[test]
    fn rejects_a_truncated_frame() {
        let compressed = deflate(r#"{"type":"pong"}"#);
        let truncated = &compressed[..compressed.len() - 4];
        assert!(matches!(
            inflate(truncated),
            Err(RadionError::Decompression(_))
        ));
    }
}
