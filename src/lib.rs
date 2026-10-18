#![cfg_attr(not(test), no_std)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::wildcard_imports,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::single_match_else
)]
// The README's example needs `file`, which needs `alloc`, so the page is only attached when it can compile.
#![cfg_attr(feature = "alloc", doc = include_str!("../README.md"))]

extern crate self as rm2k;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod cursor;
pub mod diag;
pub mod engine;
pub mod error;
pub mod event;
#[cfg(feature = "alloc")]
pub mod field;
#[cfg(feature = "alloc")]
pub mod file;
#[cfg(feature = "alloc")]
pub mod owned;
#[cfg(feature = "alloc")]
pub mod raw;
#[cfg(feature = "alloc")]
pub mod rpg;
#[cfg(feature = "alloc")]
pub mod schema;
#[cfg(feature = "serde")]
mod serde;
#[cfg(feature = "alloc")]
pub mod setup;
pub mod sink;
pub mod wire;

pub use cursor::{Chunk, ChunkIter, Ctx, Cursor, ListItem, ListWalker};
pub use diag::{DiagKind, DiagSink, Diagnostic, NullSink, SliceSink, StrictSink};
pub use engine::{Engine, SaveOpt};
pub use error::{ReadError, WriteError};
pub use event::{EventCommandIter, EventCommandRef, MoveCommandIter, MoveCommandRef};
#[cfg(feature = "alloc")]
pub use field::{
    BitArray, DbStr, LcfField, LcfIndexed, LcfStruct, List, Packed, UnindexedList, UnknownChunk, UnknownChunks,
};
#[cfg(feature = "alloc")]
pub use raw::{
    DatabaseVersion, EmptyBlock, Equipment, EventCommand, EventCommandList, MoveCommand, MoveCommandList, Parameters,
    Rect, StrList,
};
#[cfg(feature = "alloc")]
pub use setup::Setup;
pub use sink::{CountSink, Sink};
