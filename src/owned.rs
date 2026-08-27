//! Detaching a loaded file from the buffer it borrows.
//!
//! Loading is zero-copy: a `Database<'a>` holds `&'a [u8]` slices pointing into the bytes you
//! read off disk, so parsing a 4 MB `RPG_RT.ldb` copies none of its text.
//!
//! ```
//! # use rm2k::{file, rpg::Database, owned::IntoOwned, Engine, SaveOpt, LcfStruct};
//! # let mut bytes = Vec::new();
//! # file::save_database(&Database::defaults(Engine::R2K), &mut bytes,
//! #                     Engine::R2K, SaveOpt::default(), b"").unwrap();
//! fn load_it(bytes: Vec<u8>) -> Database<'static> {
//!     let db = file::load_database(&bytes).unwrap();
//!     db.value.into_owned()
//! }
//! # let _ = load_it(bytes);
//! ```

use crate::field::{BitArray, DbStr, List, Packed, PackedPrim, UnindexedList};
use alloc::vec::Vec;

/// A value that can be detached from the buffer it borrows.
pub trait IntoOwned {
    /// The same shape, owning its data.
    type Owned: 'static;

    /// Copy anything still borrowed, so the result outlives the input buffer.
    fn into_owned(self) -> Self::Owned;
}

/// Implement a no-op detach for types that never borrow.
#[macro_export]
macro_rules! impl_identity_owned {
    ($($ty:ty),* $(,)?) => {
        $(
            impl $crate::owned::IntoOwned for $ty {
                type Owned = Self;
                #[inline]
                fn into_owned(self) -> Self {
                    self
                }
            }
        )*
    };
}

impl_identity_owned!(i8, u8, i16, u32, i32, f64, bool);

impl IntoOwned for DbStr<'_> {
    type Owned = DbStr<'static>;
    #[inline]
    fn into_owned(self) -> DbStr<'static> {
        DbStr::into_owned(self)
    }
}

impl IntoOwned for BitArray<'_> {
    type Owned = BitArray<'static>;
    #[inline]
    fn into_owned(self) -> BitArray<'static> {
        BitArray::into_owned(self)
    }
}

impl<T: PackedPrim + 'static> IntoOwned for Packed<T> {
    type Owned = Self;
    #[inline]
    fn into_owned(self) -> Self {
        self
    }
}

impl<T: IntoOwned> IntoOwned for Vec<T> {
    type Owned = Vec<T::Owned>;
    #[inline]
    fn into_owned(self) -> Vec<T::Owned> {
        self.into_iter().map(IntoOwned::into_owned).collect()
    }
}

impl<T: IntoOwned> IntoOwned for List<T> {
    type Owned = List<T::Owned>;
    #[inline]
    fn into_owned(self) -> List<T::Owned> {
        List(self.0.into_owned())
    }
}

impl<T: IntoOwned> IntoOwned for UnindexedList<T> {
    type Owned = UnindexedList<T::Owned>;
    #[inline]
    fn into_owned(self) -> UnindexedList<T::Owned> {
        UnindexedList(self.0.into_owned())
    }
}

// The raw codecs: `Rect`, `Equipment` and `Parameters` own their data outright, while the three
// command/string lists borrow.
impl_identity_owned!(
    crate::raw::EmptyBlock,
    crate::raw::DatabaseVersion,
    crate::raw::Rect,
    crate::raw::Equipment,
    crate::raw::Parameters,
);

impl IntoOwned for crate::raw::EventCommand<'_> {
    type Owned = crate::raw::EventCommand<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::raw::EventCommand {
            code: self.code,
            indent: self.indent,
            string: self.string.into_owned(),
            parameters: self.parameters,
        }
    }
}

impl IntoOwned for crate::raw::EventCommandList<'_> {
    type Owned = crate::raw::EventCommandList<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::raw::EventCommandList(self.0.into_owned())
    }
}

impl IntoOwned for crate::raw::MoveCommand<'_> {
    type Owned = crate::raw::MoveCommand<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::raw::MoveCommand {
            code: self.code,
            string: self.string.into_owned(),
            parameter_a: self.parameter_a,
            parameter_b: self.parameter_b,
            parameter_c: self.parameter_c,
        }
    }
}

impl IntoOwned for crate::raw::MoveCommandList<'_> {
    type Owned = crate::raw::MoveCommandList<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::raw::MoveCommandList(self.0.into_owned())
    }
}

impl IntoOwned for crate::raw::StrList<'_> {
    type Owned = crate::raw::StrList<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::raw::StrList(self.0.into_owned())
    }
}

impl IntoOwned for crate::rpg::TreeMap<'_> {
    type Owned = crate::rpg::TreeMap<'static>;
    fn into_owned(self) -> Self::Owned {
        crate::rpg::TreeMap {
            maps: self.maps.into_owned(),
            tree_order: self.tree_order,
            active_node: self.active_node,
            start: self.start,
        }
    }
}
