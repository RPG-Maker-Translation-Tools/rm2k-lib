//! Tests for the wire codec (`rm2k::wire`) - compressed integers and little-endian primitives.
//!
//! Lives here rather than inline in `src/wire.rs` because every item under test is `pub`, so
//! there is nothing a black-box test needs that an external crate cannot already reach.

use rm2k::wire::{
    MAX_VARINT_LEN, MAX_VARINT64_LEN, decode_varint, decode_varint64, encode_varint, encode_varint64, read_f64_le,
    read_i16_le, read_u32_le, varint_len,
};

#[test]
fn varint_is_big_endian_base128() {
    // 0x81 0x00 => (1 << 7) | 0 == 128. Little-endian LEB128 would decode this as 1.
    let v = decode_varint(&[0x81, 0x00]).unwrap();
    assert_eq!(v.value, 128);
    assert_eq!(v.consumed, 2);
    assert!(!v.overflow);
}

#[test]
fn varint_single_byte() {
    for n in 0u32..=0x7F {
        let v = decode_varint(&[n as u8]).unwrap();
        assert_eq!(v.value, n);
        assert_eq!(v.consumed, 1);
    }
}

#[test]
fn varint_round_trips() {
    for value in [
        0u32,
        1,
        0x7F,
        0x80,
        0x3FFF,
        0x4000,
        0xFFFF,
        0x0010_0000,
        0x7FFF_FFFF,
        u32::MAX,
    ] {
        let mut buf = [0u8; MAX_VARINT_LEN];
        let len = encode_varint(value, &mut buf);
        assert_eq!(len as u32, varint_len(value), "length mismatch for {value:#x}");
        let decoded = decode_varint(&buf[..len]).unwrap();
        assert_eq!(decoded.value, value, "round-trip failed for {value:#x}");
        assert_eq!(decoded.consumed, len);
    }
}

#[test]
fn varint_len_matches_liblcf_int_size() {
    assert_eq!(varint_len(0), 1);
    assert_eq!(varint_len(0x7F), 1);
    assert_eq!(varint_len(0x80), 2);
    assert_eq!(varint_len(0x3FFF), 2);
    assert_eq!(varint_len(0x4000), 3);
    assert_eq!(varint_len(u32::MAX), 5);
}

#[test]
fn varint_over_five_bytes_yields_zero_but_still_consumes() {
    // Six continuation bytes then a terminator: liblcf consumes all seven and returns 0.
    let bytes = [0x81, 0x81, 0x81, 0x81, 0x81, 0x81, 0x00];
    let v = decode_varint(&bytes).unwrap();
    assert_eq!(v.value, 0);
    assert_eq!(v.consumed, 7);
    assert!(v.overflow);
}

#[test]
fn varint_unterminated_is_none() {
    assert!(decode_varint(&[0x81, 0x81]).is_none());
    assert!(decode_varint(&[]).is_none());
}

#[test]
fn varint64_round_trips() {
    for value in [
        0u64,
        1,
        0x7F,
        0x80,
        u64::from(u32::MAX),
        0x8_0000_0000,
        0x8_0000_0000 - 3,
    ] {
        let mut buf = [0u8; MAX_VARINT64_LEN];
        let len = encode_varint64(value, &mut buf);
        assert_eq!(len as u32, rm2k::wire::varint64_len(value));
        let decoded = decode_varint64(&buf[..len]).unwrap();
        assert_eq!(decoded.value, value);
        assert_eq!(decoded.consumed, len);
        assert!(!decoded.overflow);
    }
}

#[test]
fn primitives_are_little_endian() {
    assert_eq!(read_i16_le(&[0x01, 0x02]), Some(0x0201));
    assert_eq!(read_u32_le(&[0x01, 0x02, 0x03, 0x04]), Some(0x0403_0201));
    assert_eq!(read_i16_le(&[0x01]), None);
    assert_eq!(read_f64_le(&1.5f64.to_le_bytes()), Some(1.5));
}
