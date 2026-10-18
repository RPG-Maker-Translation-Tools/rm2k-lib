//! The LCF wire codec: compressed integers, fixed-width primitives, file magics.
//!
//! Pure functions over byte slices, no cursor state; [`Cursor`](crate::cursor::Cursor) adds
//! position tracking and diagnostics. Allocation-free, no `unsafe`.
//!
//! # Compressed integers
//!
//! LCF varints are **big-endian** base-128 with an MSB continuation bit, unlike protobuf's LEB128;
//! an off-the-shelf varint crate decodes them wrong.
//!
//! A well-formed encoding is at most 5 bytes. Longer ones are malformed, but their bytes must still
//! be consumed to keep the cursor in sync: [`decode_varint`] reads until a byte with the high bit
//! clear, decodes to `0`, and sets `overflow` so the caller can record a
//! [`DiagKind::BadVarint`](crate::diag::DiagKind::BadVarint). Accumulation is in a 32-bit register
//! (`value = (value << 7) | (byte & 0x7F)`), so a maximal 5-byte encoding wraps.

use crate::sink::Sink;

pub const LDB_HEADER: &[u8] = b"LcfDataBase";
pub const LMU_HEADER: &[u8] = b"LcfMapUnit";
pub const LMT_HEADER: &[u8] = b"LcfMapTree";
pub const LSD_HEADER: &[u8] = b"LcfSaveData";

/// Most bytes a 32-bit compressed integer may occupy.
pub const MAX_VARINT_LEN: usize = 5;
/// Most bytes a 64-bit compressed integer may occupy.
pub const MAX_VARINT64_LEN: usize = 10;

/// A decoded compressed integer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Varint {
    /// The value, already zeroed when `overflow` is set.
    pub value: u32,
    /// Bytes consumed, including the over-long tail when `overflow` is set.
    pub consumed: usize,
    /// The encoding ran past the byte cap; liblcf substitutes `0`.
    pub overflow: bool,
}

/// Decode a 32-bit compressed integer from the front of `buf`.
///
/// Returns `None` when `buf` ends before a terminating byte (high bit clear) is found. liblcf
/// treats that as `0` at EOF; callers should do the same after recording a diagnostic.
#[must_use]
pub fn decode_varint(buf: &[u8]) -> Option<Varint> {
    let mut value: u32 = 0;
    let mut consumed = 0usize;

    loop {
        let byte = *buf.get(consumed)?;
        consumed += 1;
        value = (value << 7) | u32::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }

    let overflow = consumed > MAX_VARINT_LEN;
    Some(Varint {
        value: if overflow { 0 } else { value },
        consumed,
        overflow,
    })
}

/// A decoded 64-bit compressed integer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Varint64 {
    /// The value, already zeroed when `overflow` is set.
    pub value: u64,
    /// Bytes consumed, including the over-long tail when `overflow` is set.
    pub consumed: usize,
    /// The encoding ran past the byte cap; liblcf substitutes `0`.
    pub overflow: bool,
}

/// Decode a 64-bit compressed integer from the front of `buf`.
///
/// Same as [`decode_varint`] with a 10-byte cap (`LcfReader::ReadUInt64`,
/// `src/reader_lcf.cpp:106`). Used only by the sparse string-vector encoding, where a value above
/// `u32::MAX` is a run-length gap marker, not a length (`src/dbstring_struct.cpp:93`).
#[must_use]
pub fn decode_varint64(buf: &[u8]) -> Option<Varint64> {
    let mut value: u64 = 0;
    let mut consumed = 0usize;

    loop {
        let byte = *buf.get(consumed)?;
        consumed += 1;
        value = (value << 7) | u64::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            break;
        }
    }

    let overflow = consumed > MAX_VARINT64_LEN;
    Some(Varint64 {
        value: if overflow { 0 } else { value },
        consumed,
        overflow,
    })
}

/// Encoded length in bytes of `value` as a compressed integer.
///
/// Mirrors `LcfReader::IntSize` (`src/reader_lcf.cpp:330`): `ceil(bits / 7)`, minimum 1.
#[must_use]
pub const fn varint_len(value: u32) -> u32 {
    let mut x = value;
    let mut result = 0u32;
    loop {
        x >>= 7;
        result += 1;
        if x == 0 {
            return result;
        }
    }
}

/// Encoded length in bytes of `value` as a 64-bit compressed integer.
#[must_use]
pub const fn varint64_len(value: u64) -> u32 {
    let mut x = value;
    let mut result = 0u32;
    loop {
        x >>= 7;
        result += 1;
        if x == 0 {
            return result;
        }
    }
}

/// Encode `value` into `out`, returning how many bytes were written.
///
/// Mirrors `LcfWriter::WriteInt` (`src/writer_lcf.cpp:50`), which emits 7-bit groups from bit 28
/// downwards - so it never writes more than 5 bytes and silently drops anything above bit 34.
#[must_use]
pub fn encode_varint(value: u32, out: &mut [u8; MAX_VARINT_LEN]) -> usize {
    let mut len = 0usize;
    let mut shift: i32 = 28;
    while shift >= 0 {
        if value >= (1u32 << shift) || shift == 0 {
            let group = ((value >> shift) & 0x7F) as u8;
            out[len] = if shift > 0 { group | 0x80 } else { group };
            len += 1;
        }
        shift -= 7;
    }
    len
}

/// Encode `value` as a 64-bit compressed integer into `out`, returning bytes written.
#[must_use]
pub fn encode_varint64(value: u64, out: &mut [u8; MAX_VARINT64_LEN]) -> usize {
    let mut len = 0usize;
    let mut shift: i32 = 56;
    while shift >= 0 {
        if value >= (1u64 << shift) || shift == 0 {
            let group = ((value >> shift) & 0x7F) as u8;
            out[len] = if shift > 0 { group | 0x80 } else { group };
            len += 1;
        }
        shift -= 7;
    }
    len
}

/// Write `value` to `sink` as a compressed integer.
///
/// # Errors
/// Propagates the sink's error. Infallible for `Vec<u8>`.
#[inline]
pub fn write_varint<S: Sink>(sink: &mut S, value: u32) -> Result<(), S::Error> {
    let mut buf = [0u8; MAX_VARINT_LEN];
    let len = encode_varint(value, &mut buf);
    sink.write(&buf[..len])
}

/// Write `value` to `sink` as a 64-bit compressed integer.
///
/// # Errors
/// Propagates the sink's error. Infallible for `Vec<u8>`.
#[inline]
pub fn write_varint64<S: Sink>(sink: &mut S, value: u64) -> Result<(), S::Error> {
    let mut buf = [0u8; MAX_VARINT64_LEN];
    let len = encode_varint64(value, &mut buf);
    sink.write(&buf[..len])
}

/// Read a little-endian `i16` from the front of `buf`.
#[must_use]
#[inline]
pub fn read_i16_le(buf: &[u8]) -> Option<i16> {
    buf.get(..2).map(|b| i16::from_le_bytes([b[0], b[1]]))
}

/// Read a little-endian `u32` from the front of `buf`.
#[must_use]
#[inline]
pub fn read_u32_le(buf: &[u8]) -> Option<u32> {
    buf.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Read a little-endian `i32` from the front of `buf`.
///
/// Raw form used inside packed arrays. A bare `Int32` **field** is a compressed integer instead
/// ([`decode_varint`]). liblcf's comment on `LcfWriter::Write<int32_t>(&Vec<int32_t>)` says
/// otherwise, and is wrong (`src/writer_lcf.cpp:101`).
#[must_use]
#[inline]
pub fn read_i32_le(buf: &[u8]) -> Option<i32> {
    buf.get(..4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Read a little-endian `f64` from the front of `buf`.
#[must_use]
#[inline]
pub fn read_f64_le(buf: &[u8]) -> Option<f64> {
    buf.get(..8)
        .map(|b| f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}
