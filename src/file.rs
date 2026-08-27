//! Loading and saving whole files.
//!
//! Every LCF file is a length-prefixed ASCII magic followed by the root struct's chunk stream:
//!
//! ```text
//! varint(len) len*u8 <chunk stream>
//! ```
//!
//! `Database` and `Save` omit the terminating zero id that every other chunk stream ends with.
//! And `TreeMap` is not a chunk stream at all; see [`TreeMap`].
//!
//! Loading takes no [`Engine`]: no field's default actually depends on it (see the
//! [schema docs](crate::schema) for why that is not obvious from the raw tables). Saving does
//! need one, and [`Engine::from_ldb_id`] recovers it from a loaded database.

use crate::{
    cursor::{ChunkIter, Ctx, Cursor},
    diag::{DiagKind, DiagSink, NullSink},
    engine::{Engine, SaveOpt},
    error::ReadError,
    field::LcfStruct,
    rpg::{Database, Map, Save, TreeMap},
    sink::Sink,
    wire,
};
use alloc::vec::Vec;

/// Read the length-prefixed magic at the head of a file.
///
/// The bytes are returned as they appear. A wrong length is fatal, wrong content
/// is a warning it parses through - and that split is preserved here.
///
/// # Errors
///
/// [`ReadError::UnexpectedEof`] if the buffer is too short for the declared magic.
pub fn read_magic<'a>(cur: &mut Cursor<'a>) -> Result<&'a [u8], ReadError> {
    let offset = cur.offset();
    cur.lstring().ok_or(ReadError::UnexpectedEof { offset })
}

/// What a successful load produced.
///
/// The header travels alongside the value because saving can be asked to preserve it verbatim,
/// and a few games ship files whose magic is not the canonical one.
#[derive(Clone, Debug)]
pub struct Loaded<'a, T> {
    /// The parsed root struct.
    pub value: T,
    /// The file's magic string, exactly as it appeared.
    pub header: &'a [u8],
}

impl<T> core::ops::Deref for Loaded<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        &self.value
    }
}

/// Generate a load/save pair for one format.
macro_rules! format_io {
    (
        $(#[$meta:meta])*
        $load:ident, $load_with:ident, $save:ident, $ty:ident, $magic:path, $zero:literal
    ) => {
        $(#[$meta])*
        ///
        /// # Errors
        ///
        /// Fails only when the file is too short to hold a header. A wrong magic, an unknown
        /// chunk or a truncated field are all recovered from and reported as diagnostics.
        pub fn $load(buf: &[u8]) -> Result<Loaded<'_, $ty<'_>>, ReadError> {
            let mut sink = NullSink;
            $load_with(buf, &mut sink)
        }

        $(#[$meta])*
        ///
        /// Recoverable problems are reported to `diag`; pass
        /// [`StrictSink`](crate::diag::StrictSink) to stop at the first one.
        ///
        /// # Errors
        ///
        /// As [`$load`].
        pub fn $load_with<'a>(
            buf: &'a [u8],
            diag: &mut dyn DiagSink,
        ) -> Result<Loaded<'a, $ty<'a>>, ReadError> {
            let mut cur = Cursor::new(buf);
            let header = read_magic(&mut cur)?;

            // The engine is irrelevant while reading; see the module docs.
            let mut ctx = Ctx::new(Engine::R2K, diag);
            if header != $magic {
                ctx.report(<$ty as LcfStruct>::NAME, 0, 0, DiagKind::BadMagic);
            }

            let mut chunks = ChunkIter::from_cursor(cur);
            let value = <$ty as LcfStruct>::read_stream(&mut chunks, &mut ctx);
            Ok(Loaded { value, header })
        }

        $(#[$meta])*
        ///
        /// # Errors
        ///
        /// Propagates the sink's error. Infallible when writing into a `Vec<u8>`.
        pub fn $save<S: Sink>(
            value: &$ty<'_>,
            sink: &mut S,
            engine: Engine,
            opt: SaveOpt,
            header: &[u8],
        ) -> Result<(), S::Error> {
            let header = if opt.preserve_header && !header.is_empty() { header } else { $magic };
            wire::write_varint(sink, header.len() as u32)?;
            sink.write(header)?;
            value.write_body(sink, engine)?;
            if $zero {
                sink.write(&[0])?;
            }
            Ok(())
        }
    };
}

format_io! {
    /// The `LDB` database (`RPG_RT.ldb`).
    load_database, load_database_with, save_database, Database, wire::LDB_HEADER, false
}

format_io! {
    /// An `LMU` map (`MapNNNN.lmu`).
    load_map, load_map_with, save_map, Map, wire::LMU_HEADER, true
}

format_io! {
    /// An `LSD` save (`SaveNN.lsd`).
    load_save, load_save_with, save_save, Save, wire::LSD_HEADER, false
}

/// Load the `LMT` map tree (`RPG_RT.lmt`).
///
/// Separate from the others because [`TreeMap`] is not a chunk stream: it is a bare sequence of
/// values read straight from the file.
///
/// # Errors
///
/// [`ReadError::UnexpectedEof`] if the buffer is too short to hold a header.
pub fn load_tree_map(buf: &[u8]) -> Result<Loaded<'_, TreeMap<'_>>, ReadError> {
    let mut sink = NullSink;
    load_tree_map_with(buf, &mut sink)
}

/// Load the `LMT` map tree, reporting recoverable problems to `diag`.
///
/// # Errors
///
/// As [`load_tree_map`].
pub fn load_tree_map_with<'a>(buf: &'a [u8], diag: &mut dyn DiagSink) -> Result<Loaded<'a, TreeMap<'a>>, ReadError> {
    let mut cur = Cursor::new(buf);
    let header = read_magic(&mut cur)?;

    let mut ctx = Ctx::new(Engine::R2K, diag);
    if header != wire::LMT_HEADER {
        ctx.report("TreeMap", 0, 0, DiagKind::BadMagic);
    }

    let value = TreeMap::read_inline(&mut cur, &mut ctx);
    Ok(Loaded { value, header })
}

/// Save the `LMT` map tree.
///
/// # Errors
///
/// Propagates the sink's error.
pub fn save_tree_map<S: Sink>(
    value: &TreeMap<'_>,
    sink: &mut S,
    engine: Engine,
    opt: SaveOpt,
    header: &[u8],
) -> Result<(), S::Error> {
    let header = if opt.preserve_header && !header.is_empty() {
        header
    } else {
        wire::LMT_HEADER
    };
    wire::write_varint(sink, header.len() as u32)?;
    sink.write(header)?;
    value.write_inline(sink, engine)
}

/// Serialize a value into a fresh `Vec`.
///
/// A convenience over the `save_*` functions for the common case; they take a
/// [`Sink`](crate::sink::Sink) so they also work against a fixed buffer with no allocator.
#[must_use]
pub fn to_vec<T, F>(save: F) -> Vec<u8>
where
    F: FnOnce(&mut Vec<u8>) -> Result<T, core::convert::Infallible>,
{
    let mut out = Vec::new();
    let _ = save(&mut out);
    out
}
