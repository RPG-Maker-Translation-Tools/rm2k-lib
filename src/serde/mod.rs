//! `serde` support for the data model.
//!
//! Enabled by the `serde` feature. Schema structs, enums and flag bundles derive
//! [`Serialize`] and [`Deserialize`]; hand-written types get their impls here. Any `serde` format
//! crate converts a loaded file to and from JSON, YAML, TOML, and the like.
//!
//! [`DbStr`] is a run of bytes in an unknown encoding, not a string, so there is no honest
//! lossless mapping onto a JSON string. Two representations, chosen by the format:
//!
//! - **Human-readable formats** (JSON, TOML, YAML) get a string with a **reversible escape**:
//!   printable ASCII passes through, `\` doubles to `\\`, and every other byte becomes `\xNN`.
//!   The result is readable for the ASCII text that most fields hold, exact for the bytes that
//!   are not, and parses back to the identical bytes.
//! - **Binary formats** (postcard, bincode, CBOR, `MessagePack`) get `serialize_bytes`, which is
//!   compact and needs no escaping at all.

use crate::{
    field::{BitArray, DbStr, List, Packed, PackedPrim, UnindexedList},
    raw::{
        DatabaseVersion, EmptyBlock, Equipment, EventCommand, EventCommandList, MoveCommand, MoveCommandList,
        Parameters, Rect, StrList,
    },
    rpg::TreeMap,
};
use alloc::{string::String, vec::Vec};
use serde::{
    de::Visitor,
    {Deserialize, Deserializer, Serialize, Serializer},
};

/// Render bytes as a reversible escaped string.
fn escape(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\\\"),
            // Printable ASCII, minus the backslash handled above.
            0x20..=0x5B | 0x5D..=0x7E => out.push(byte as char),
            _ => {
                out.push_str("\\x");
                out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
                out.push(char::from_digit(u32::from(byte & 0x0F), 16).unwrap_or('0'));
            }
        }
    }
    out
}

/// Invert [`escape`].
fn unescape<E: serde::de::Error>(text: &str) -> Result<Vec<u8>, E> {
    let mut out = Vec::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            // Anything the escape did not produce is passed through as its UTF-8 bytes, so a
            // string that was never escaped in the first place still round-trips.
            let mut buf = [0u8; 4];
            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            continue;
        }
        match chars.next() {
            Some('\\') => out.push(b'\\'),
            Some('x') => {
                let hi = chars.next().and_then(|c| c.to_digit(16));
                let lo = chars.next().and_then(|c| c.to_digit(16));
                match (hi, lo) {
                    (Some(hi), Some(lo)) => out.push((hi * 16 + lo) as u8),
                    _ => return Err(E::custom("truncated \\xNN escape")),
                }
            }
            _ => return Err(E::custom("unknown escape sequence")),
        }
    }
    Ok(out)
}

/// Serialize a byte run: escaped text for readable formats, raw bytes for binary ones.
fn serialize_bytes<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        serializer.serialize_str(&escape(bytes))
    } else {
        serializer.serialize_bytes(bytes)
    }
}

/// Accepts either representation, so a file written as JSON can be read back as postcard.
struct BytesVisitor;

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("an escaped string or a byte array")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Vec<u8>, E> {
        unescape(value)
    }

    fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Vec<u8>, E> {
        Ok(value.to_vec())
    }

    fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Vec<u8>, E> {
        Ok(value)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<u8>, A::Error> {
        let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(byte) = seq.next_element::<u8>()? {
            out.push(byte);
        }
        Ok(out)
    }
}

fn deserialize_bytes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(BytesVisitor)
    } else {
        deserializer.deserialize_byte_buf(BytesVisitor)
    }
}

impl Serialize for DbStr<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_bytes(self.as_bytes(), serializer)
    }
}

impl<'de> Deserialize<'de> for DbStr<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_bytes(deserializer).map(DbStr::from_vec)
    }
}

impl Serialize for BitArray<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for BitArray<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let flags = Vec::<bool>::deserialize(deserializer)?;
        Ok(BitArray::from_vec(flags.into_iter().map(u8::from).collect()))
    }
}

/// Newtypes over a `Vec` serialize as the sequence, with no wrapper object.
macro_rules! transparent_seq {
    ($($name:ident<$param:ident $(: $bound:path)?>),* $(,)?) => {
        $(
            impl<$param: Serialize $(+ $bound)?> Serialize for $name<$param> {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.0.serialize(serializer)
                }
            }

            impl<'de, $param: Deserialize<'de> $(+ $bound)?> Deserialize<'de> for $name<$param> {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    Vec::<$param>::deserialize(deserializer).map($name)
                }
            }
        )*
    };
}

transparent_seq!(List<T>, UnindexedList<T>, Packed<T: PackedPrim>);

impl Serialize for StrList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, 'a> Deserialize<'de> for StrList<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<DbStr<'a>>::deserialize(deserializer).map(StrList)
    }
}

impl Serialize for EventCommandList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, 'a> Deserialize<'de> for EventCommandList<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<EventCommand<'a>>::deserialize(deserializer).map(EventCommandList)
    }
}

impl Serialize for MoveCommandList<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, 'a> Deserialize<'de> for MoveCommandList<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<MoveCommand<'a>>::deserialize(deserializer).map(MoveCommandList)
    }
}

/// Derive-shaped implementations for the raw structs, which are ordinary records.
mod plain {
    use super::{
        DatabaseVersion, DbStr, Deserialize, EmptyBlock, Equipment, EventCommand, MoveCommand, Packed, Parameters,
        Rect, Serialize, TreeMap, Vec,
    };
    use crate::field::List;

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "EmptyBlock")]
    pub struct EmptyBlockDef;

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "DatabaseVersion")]
    pub struct DatabaseVersionDef(pub i32);

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "Rect")]
    pub struct RectDef {
        pub left: u32,
        pub top: u32,
        pub right: u32,
        pub bottom: u32,
    }

    #[allow(clippy::struct_field_names)]
    #[derive(Deserialize, Serialize)]
    #[serde(remote = "Equipment")]
    pub struct EquipmentDef {
        pub weapon_id: i16,
        pub shield_id: i16,
        pub armor_id: i16,
        pub helmet_id: i16,
        pub accessory_id: i16,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "Parameters")]
    pub struct ParametersDef {
        pub maxhp: Packed<i16>,
        pub maxsp: Packed<i16>,
        pub attack: Packed<i16>,
        pub defense: Packed<i16>,
        pub spirit: Packed<i16>,
        pub agility: Packed<i16>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "EventCommand", bound(deserialize = ""))]
    pub struct EventCommandDef<'a> {
        pub code: i32,
        pub indent: i32,
        pub string: DbStr<'a>,
        pub parameters: Vec<i32>,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "MoveCommand", bound(deserialize = ""))]
    pub struct MoveCommandDef<'a> {
        pub code: i32,
        pub string: DbStr<'a>,
        pub parameter_a: i32,
        pub parameter_b: i32,
        pub parameter_c: i32,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(remote = "TreeMap", bound(deserialize = ""))]
    pub struct TreeMapDef<'a> {
        pub maps: List<crate::rpg::MapInfo<'a>>,
        pub tree_order: Vec<i32>,
        pub active_node: i32,
        pub start: crate::rpg::Start,
    }
}

/// Forward a type's `Serialize`/`Deserialize` to its `#[serde(remote)]` definition.
macro_rules! via_remote {
    ($($ty:ty => $def:path),* $(,)?) => {
        $(
            impl Serialize for $ty {
                fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    $def(self, serializer)
                }
            }
        )*
    };
}

via_remote! {
    EmptyBlock => plain::EmptyBlockDef::serialize,
    DatabaseVersion => plain::DatabaseVersionDef::serialize,
    Rect => plain::RectDef::serialize,
    Equipment => plain::EquipmentDef::serialize,
    Parameters => plain::ParametersDef::serialize,
    EventCommand<'_> => plain::EventCommandDef::serialize,
    MoveCommand<'_> => plain::MoveCommandDef::serialize,
    TreeMap<'_> => plain::TreeMapDef::serialize,
}

/// The deserialising half, which needs its own lifetime plumbing.
macro_rules! via_remote_de {
    ($($ty:ty => $def:path),* $(,)?) => {
        $(
            impl<'de> Deserialize<'de> for $ty {
                fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    $def(deserializer)
                }
            }
        )*
    };
}

via_remote_de! {
    EmptyBlock => plain::EmptyBlockDef::deserialize,
    DatabaseVersion => plain::DatabaseVersionDef::deserialize,
    Rect => plain::RectDef::deserialize,
    Equipment => plain::EquipmentDef::deserialize,
    Parameters => plain::ParametersDef::deserialize,
}

impl<'de> Deserialize<'de> for EventCommand<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        plain::EventCommandDef::deserialize(deserializer)
    }
}

impl<'de> Deserialize<'de> for MoveCommand<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        plain::MoveCommandDef::deserialize(deserializer)
    }
}

impl<'de> Deserialize<'de> for TreeMap<'_> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        plain::TreeMapDef::deserialize(deserializer)
    }
}

#[cfg(test)]
#[path = "test.rs"]
mod tests;
