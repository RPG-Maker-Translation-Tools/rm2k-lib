//! Which RPG Maker produced the file.
//!
//! Changes the wire format, not just metadata. Roughly 90 schema fields exist only in 2003 and are
//! skipped when writing 2000. Other fields have per-engine defaults; the writer elides fields
//! equal to their default, so those matter too. Flag bundles suffer most: skipping a 2003-only
//! flag shifts every later bit.

/// The engine a file targets.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Engine {
    R2K,
    R2K3,
}

impl Engine {
    #[must_use]
    #[inline]
    pub const fn is_2k3(self) -> bool {
        matches!(self, Self::R2K3)
    }

    /// Pick between a 2000 value and a 2003 value.
    ///
    /// This is what the schema macro's `engine!(a, b)` default expands to.
    #[must_use]
    #[inline]
    pub const fn pick<T: Copy>(self, r2k: T, r2k3: T) -> T {
        match self {
            Self::R2K => r2k,
            Self::R2K3 => r2k3,
        }
    }

    /// Classify a database's `system.ldb_id`.
    #[must_use]
    #[inline]
    pub const fn from_ldb_id(ldb_id: i32) -> Self {
        if ldb_id == 2003 { Self::R2K3 } else { Self::R2K }
    }
}

impl Default for Engine {
    #[inline]
    fn default() -> Self {
        Self::R2K
    }
}

/// Options affecting how a file is written.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SaveOpt {
    /// Keep the header bytes exactly as they were read rather than regenerating them.
    pub preserve_header: bool,
}
