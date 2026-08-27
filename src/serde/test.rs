use super::*;

#[test]
fn ascii_text_stays_readable() {
    assert_eq!(escape(b"Hello, world!"), "Hello, world!");
}

#[test]
fn non_ascii_bytes_escape_reversibly() {
    // Shift-JIS, which is not valid UTF-8 and must survive exactly.
    let bytes = [0x82u8, 0xA0, 0x82, 0xA2];
    let text = escape(&bytes);
    assert_eq!(text, "\\x82\\xa0\\x82\\xa2");
    let back: Vec<u8> = unescape::<serde_json::Error>(&text).unwrap();
    assert_eq!(back, bytes);
}

#[test]
fn a_literal_backslash_round_trips() {
    // The escape must not be confusable with text that already contained a backslash.
    let bytes = br"C:\x41\path";
    let text = escape(bytes);
    let back: Vec<u8> = unescape::<serde_json::Error>(&text).unwrap();
    assert_eq!(back, bytes);
}

#[test]
fn unescaping_rejects_a_truncated_escape() {
    assert!(unescape::<serde_json::Error>("\\x8").is_err());
    assert!(unescape::<serde_json::Error>("\\q").is_err());
}

#[test]
fn every_byte_round_trips() {
    let all: Vec<u8> = (0..=255u8).collect();
    let back: Vec<u8> = unescape::<serde_json::Error>(&escape(&all)).unwrap();
    assert_eq!(back, all);
}
