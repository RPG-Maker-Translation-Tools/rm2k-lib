//! The `lcf_struct!`, `lcf_flags!` and `lcf_enum!` schema macros: the declarations that build
//! every struct, flag bundle and enum in [`crate::rpg`].
//!
//! ```rust
//! # use rm2k::lcf_struct;
//! lcf_struct! {
//!     /// RPG::Learning
//!     #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
//!     pub struct Learning indexed {
//!         0x01 level:    i32 = 1,
//!         0x02 skill_id: i32 = 1,
//!     }
//! }
//! ```
//!
//! Every generated struct carries a `pub unknown_chunks: `[`UnknownChunks`](crate::field::UnknownChunks)
//! field. A chunk whose id matches no field is kept verbatim, anchored next to the last field
//! that *was* recognised, and written back in that position - so a file carrying ids this schema
//! does not know still round-trips byte-for-byte. It is still reported as a
//! [`DiagKind::UnknownChunk`](crate::diag::DiagKind::UnknownChunk): preserved is not understood.
//!
//! Two consequences for callers of this macro:
//!
//! - A struct literal must initialise `unknown_chunks` (`UnknownChunks::new()` for a fresh
//!   value). [`defaults`](crate::field::LcfStruct::defaults) already does.
//! - With the `serde` feature enabled, the struct **must** derive `Serialize`/`Deserialize`, as
//!   above. The generated field carries `serde` attributes, and that attribute only exists where
//!   a `serde` derive registers it.
//!
//! Field types are ordinary Rust types, and the codec is chosen
//! by [`LcfField`](crate::field::LcfField). [`Packed<i16>`](crate::field::Packed) (a raw packed
//! array) and [`List<T>`](crate::field::List) (a counted list of id-prefixed chunk streams) are
//! distinct types precisely because they are different wire forms.
//!
//! | Declaration | Wire form |
//! |---|---|
//! | `i32` | compressed integer |
//! | `bool` | compressed integer on read, one byte on write |
//! | `u8`, `i8`, `i16`, `u32`, `f64` | fixed-width little-endian |
//! | `DbStr<'a>` | raw bytes; the chunk length *is* the length |
//! | `BitArray<'a>` | one byte per boolean |
//! | `Packed<T>` | packed array, count derived from chunk length |
//! | `List<T>` | `varint(count)` then id-prefixed chunk streams |
//! | any `LcfStruct` | a nested chunk stream |
//! | a `lcf_flags!` struct | bit-packed booleans, LSB first |
//!
//! # Per-field modifiers
//!
//! - `= expr` - explicit default. Without it the field's
//!   [`field_default`](crate::field::LcfField::field_default) is used.
//! - `persist` - write the chunk even when it equals its default.
//! - `persist2k` - `persist`, but only under RPG Maker 2000; under 2003 the field is elided at
//!   its default like any other. Exactly two fields need it: `Terms::encounter` (chunk 0x01) and
//!   `Terms::escape_success` (chunk 0x03).
//! - `is2k3` - the field exists only in RPG Maker 2003; skip it entirely when writing 2000.
//! - `size = 0xNN` - emit a companion chunk `0xNN` holding this field's **byte length**.
//! - `count = 0xNN` - emit a companion chunk `0xNN` holding this field's **element count**.
//! - `size_persist` / `count_persist` - as above, but never elided.
//! - `db_version` - the one field ([`DatabaseVersion`](crate::raw::DatabaseVersion)) that is
//!   always written under RPG Maker 2003 and elided under 2000.
//!
//! A companion chunk is read and discarded - the chunk length already says how long the payload
//! is - but must be written back, and its elision is decided by its own element count, not its
//! sibling's value. Its id is always one below the sibling's, so declaring it as a modifier keeps
//! the emitted order right without needing a separate field entry.
//!
//! # No engine-dependent defaults
//!
//! Three `Actor` fields look, from the schema tables, as though their default depends on the
//! engine ("50 under 2000, 99 under 2003"). It does not: the schema stores a `-1` sentinel for
//! all three, and the per-engine value is substituted afterwards by
//! [`Setup::setup`](crate::setup::Setup::setup), a separate, opt-in step.

/// Define a chunked LCF struct.
///
/// See the [module docs](self) for the syntax.
#[macro_export]
macro_rules! lcf_struct {
    // Four entry forms: with or without a lifetime, indexed or not. They differ only in what they
    // prepend to the struct and which generics they hand down, so each delegates to `@def`
    // immediately.
    //
    // A struct with no borrowed field takes no lifetime parameter - an unused one is a hard
    // error, and `PhantomData` would leak into every struct literal.

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident<$lt:lifetime> indexed { $($body:tt)* }
    ) => {
        $crate::lcf_struct!(@def $(#[$meta])* $vis $name $lt [<$lt>] [$name<$lt>] [static]
            [
                /// This element's id, stored ahead of its chunk stream by the containing
                /// [`List`](crate::field::List). Not a chunk of its own.
                pub id: i32,
            ]
            [ id: 0, ] [indexed]
            { $($body)* });
        $crate::lcf_struct!(@indexed $lt [$name<$lt>]);
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident<$lt:lifetime> { $($body:tt)* }
    ) => {
        $crate::lcf_struct!(@def $(#[$meta])* $vis $name $lt [<$lt>] [$name<$lt>] [static] [] [] []
            { $($body)* });
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident indexed { $($body:tt)* }
    ) => {
        $crate::lcf_struct!(@def $(#[$meta])* $vis $name 'lcf [] [$name] []
            [
                /// This element's id, stored ahead of its chunk stream by the containing
                /// [`List`](crate::field::List). Not a chunk of its own.
                pub id: i32,
            ]
            [ id: 0, ] [indexed]
            { $($body)* });
        $crate::lcf_struct!(@indexed 'lcf [$name]);
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident { $($body:tt)* }
    ) => {
        $crate::lcf_struct!(@def $(#[$meta])* $vis $name 'lcf [] [$name] [] [] [] []
            { $($body)* });
    };

    // A macro call cannot expand into struct-field position, so the optional `id` field travels
    // down as a bracketed token list rather than as a nested invocation. The field list is
    // re-parsed here; that is one extra parse of the token stream, not a recursive walk over it,
    // so expansion stays linear even for the 127-field `Terms`.

    (@indexed $lt:lifetime [$($ty:tt)*]) => {
        impl<$lt> $crate::field::LcfIndexed<$lt> for $($ty)* {
            #[inline]
            fn id(&self) -> i32 {
                self.id
            }
            #[inline]
            fn set_id(&mut self, id: i32) {
                self.id = id;
            }
        }
    };

    (@def
        $(#[$meta:meta])*
        $vis:vis $name:ident $lt:lifetime
        [$($gen:tt)*] [$($ty:tt)*] [$($owned_lt:tt)*]
        [ $($id_field:tt)* ]
        [ $($id_init:tt)* ]
        [ $($id_marker:tt)* ]
        {
            $(
                $(#[$fmeta:meta])*
                $id:literal $fname:ident : $fty:ty
                    $(= $default:expr)?
                    $(, $flag:ident $(= $fval:literal)? )*
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        $vis struct $name$($gen)* {
            $($id_field)*
            $(
                $(#[$fmeta])*
                pub $fname: $fty,
            )*
            /// Chunks this schema has no field for, kept verbatim so they survive a load/save
            /// cycle. See [`UnknownChunk`](crate::field::UnknownChunk).
            #[cfg_attr(feature = "serde", serde(default))]
            #[cfg_attr(feature = "serde",
                serde(skip_serializing_if = "rm2k::field::UnknownChunks::is_empty"))]
            pub unknown_chunks: $crate::field::UnknownChunks,
        }

        impl<$lt> $crate::field::LcfStruct<$lt> for $($ty)* {
            const NAME: &'static str = stringify!($name);

            #[allow(unused_variables)]
            fn defaults(engine: $crate::engine::Engine) -> Self {
                Self {
                    $($id_init)*
                    $(
                        $fname: $crate::lcf_struct!(@default engine, $fty $(, $default)?),
                    )*
                    unknown_chunks: $crate::field::UnknownChunks::new(),
                }
            }

            fn apply_chunk(
                &mut self,
                id: u32,
                data: &$lt [u8],
                ctx: &mut $crate::cursor::Ctx<'_>,
            ) -> ::core::option::Option<u32> {
                match id {
                    $(
                        $id => {
                            self.$fname = <$fty as $crate::field::LcfField<$lt>>::read(
                                data, ctx, <Self as $crate::field::LcfStruct>::NAME, $id,
                            );
                            ::core::option::Option::Some($id)
                        }
                    )*
                    _ => {
                        let _ = (data, ctx);
                        // Companion size/count chunks are recognised but discarded: the chunk
                        // framing already carries the length, so the value is redundant on read.
                        // Kept off the matched path so it costs nothing for the ~39 structs that
                        // have no companions at all.
                        //
                        // A companion reports its *sibling's* id, not its own, because that is
                        // the id the writer emits it next to - so an unknown chunk that followed
                        // it anchors to the same place either way.
                        $($(
                            if $crate::lcf_struct!(@is_companion id, $flag $(= $fval)?) {
                                return ::core::option::Option::Some($id);
                            }
                        )*)*
                        ::core::option::Option::None
                    }
                }
            }

            #[inline]
            fn unknown_chunks(&self) -> &$crate::field::UnknownChunks {
                &self.unknown_chunks
            }

            #[inline]
            fn unknown_chunks_mut(&mut self) -> &mut $crate::field::UnknownChunks {
                &mut self.unknown_chunks
            }

            #[allow(unused_variables)]
            fn write_body<S: $crate::sink::Sink>(
                &self,
                sink: &mut S,
                engine: $crate::engine::Engine,
            ) -> ::core::result::Result<(), S::Error> {
                // Anything that preceded the first recognised chunk. See `UnknownChunk::anchor`.
                self.unknown_chunks.write_anchored(sink, 0)?;
                $(
                    {
                        // Each modifier contributes one term, so the rules stay linear in the
                        // number of modifiers rather than combinatorial in them.
                        let allowed = true
                            $( && $crate::lcf_struct!(@allow engine, $flag $(= $fval)?) )*;

                        if allowed {
                            // Only *this* field's default is materialised, and only when the
                            // field is reachable. liblcf builds an entire default struct per
                            // object written (`StructDefault<S>::make`), so for a struct with
                            // array-valued defaults it allocates a vector per element written.
                            // Here nothing is allocated unless this one field's own default is
                            // an array.
                            let fdefault =
                                $crate::lcf_struct!(@default engine, $fty $(, $default)?);

                            // A companion's id is always directly below its sibling's, so
                            // emitting it here keeps the stream in the order liblcf produces.
                            $(
                                $crate::lcf_struct!(@companion
                                    sink, self, fdefault, engine, $fname, $flag $(= $fval)?);
                            )*

                            let force = false
                                $( || $crate::lcf_struct!(@force engine, $flag $(= $fval)?) )*;
                            if force || self.$fname != fdefault {
                                let len =
                                    $crate::field::LcfField::lcf_size(&self.$fname, engine);
                                $crate::wire::write_varint(sink, $id)?;
                                $crate::wire::write_varint(sink, len)?;
                                $crate::field::LcfField::write(&self.$fname, sink, engine)?;
                            }
                        }

                        // Outside the `allowed`/`force` guards on purpose: an unknown chunk keeps
                        // its place relative to the chunks that *are* written, whether or not the
                        // field it anchors to was elided or skipped for this engine.
                        self.unknown_chunks.write_anchored(sink, $id)?;
                    }
                )*
                ::core::result::Result::Ok(())
            }
        }

        $crate::lcf_struct!(@owned_impl
            $lt [$($ty)*] [$($owned_lt)*] $name [$($id_marker)*] [$($fname),*]);
    };

    // Emitted as one rule rather than inline in `@def` because the struct literal mentions
    // `self`, and a `self` token written at a different expansion site would not resolve to the
    // `self` bound by this `fn` - the same hygiene wall that keeps `engine` out of a call-site
    // default expression.
    (@owned_impl $lt:lifetime [$($ty:tt)*] [$($owned_lt:tt)*] $n:ident [indexed] [$($f:ident),*]) => {
        impl<$lt> $crate::owned::IntoOwned for $($ty)* {
            type Owned = $crate::lcf_struct!(@owned_ty $n $($owned_lt)*);

            fn into_owned(self) -> Self::Owned {
                // The literal needs no explicit lifetime; inference takes it from the return type.
                $n {
                    id: self.id,
                    $( $f: $crate::owned::IntoOwned::into_owned(self.$f), )*
                    unknown_chunks: self.unknown_chunks,
                }
            }
        }
    };

    (@owned_impl $lt:lifetime [$($ty:tt)*] [$($owned_lt:tt)*] $n:ident [] [$($f:ident),*]) => {
        impl<$lt> $crate::owned::IntoOwned for $($ty)* {
            type Owned = $crate::lcf_struct!(@owned_ty $n $($owned_lt)*);

            fn into_owned(self) -> Self::Owned {
                $n {
                    $( $f: $crate::owned::IntoOwned::into_owned(self.$f), )*
                    unknown_chunks: self.unknown_chunks,
                }
            }
        }
    };

    // A struct that borrows detaches to `Name<'static>`; one that does not is already owned.
    // The marker exists because `'static` cannot travel inside a macro invocation's argument
    // list - the parser reads it as a label there - but is perfectly fine in type position.
    (@owned_ty $n:ident static) => { $n<'static> };
    (@owned_ty $n:ident) => { $n };

    // This field's default: the schema's explicit value, or the type's own.
    //
    // Materialised per field rather than as a whole default struct. liblcf builds an entire
    // default object for every object it writes (`StructDefault<S>::make`), which for a struct
    // with array-valued defaults means allocating a vector per element written.
    (@default $e:ident, $fty:ty, $default:expr) => { $default };
    (@default $e:ident, $fty:ty) => {
        <$fty as $crate::field::LcfField<'_>>::field_default($e)
    };

    // May this field be written at all? Only `is2k3` ever says no.
    (@allow $e:ident, is2k3) => { $e.is_2k3() };
    (@allow $e:ident, persist) => { true };
    (@allow $e:ident, persist2k) => { true };
    (@allow $e:ident, db_version) => { true };
    (@allow $e:ident, size = $c:literal) => { true };
    (@allow $e:ident, count = $c:literal) => { true };
    (@allow $e:ident, size_persist = $c:literal) => { true };
    (@allow $e:ident, count_persist = $c:literal) => { true };

    // Must this field be written even at its default value?
    //
    // `persist` always says yes. `db_version` says yes only under RPG Maker 2003, where
    // `DatabaseVersionField::IsDefault` returns `false` unconditionally so the chunk is always
    // emitted - with an empty payload when the version is 0 (`src/reader_struct.h`).
    (@force $e:ident, persist) => { true };
    (@force $e:ident, persist2k) => { !$e.is_2k3() };
    (@force $e:ident, db_version) => { $e.is_2k3() };
    (@force $e:ident, is2k3) => { false };
    (@force $e:ident, size = $c:literal) => { false };
    (@force $e:ident, count = $c:literal) => { false };
    (@force $e:ident, size_persist = $c:literal) => { false };
    (@force $e:ident, count_persist = $c:literal) => { false };

    // Is this chunk id a companion of the field carrying this modifier?
    (@is_companion $id:ident, size = $c:literal) => { $id == $c };
    (@is_companion $id:ident, count = $c:literal) => { $id == $c };
    (@is_companion $id:ident, size_persist = $c:literal) => { $id == $c };
    (@is_companion $id:ident, count_persist = $c:literal) => { $id == $c };
    (@is_companion $id:ident, persist) => { false };
    (@is_companion $id:ident, persist2k) => { false };
    (@is_companion $id:ident, db_version) => { false };
    (@is_companion $id:ident, is2k3) => { false };

    // Emit the companion chunk, if this modifier is one.
    //
    // liblcf elides it when the sibling's element count matches the default's - note that is the
    // *count*, not the value, and it is decided separately from whether the sibling itself is written.
    // Seven of the thirty-two companions carry `PersistIfDefault`, hence the `_persist` variants.
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, size = $c:literal) => {
        if $crate::field::LcfField::element_count(&$self.$f)
            != $crate::field::LcfField::element_count(&$d)
        {
            $crate::lcf_struct!(@emit_companion $s, $c,
                $crate::field::LcfField::lcf_size(&$self.$f, $e));
        }
    };
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, size_persist = $c:literal) => {
        $crate::lcf_struct!(@emit_companion $s, $c,
            $crate::field::LcfField::lcf_size(&$self.$f, $e));
    };
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, count = $c:literal) => {
        if $crate::field::LcfField::element_count(&$self.$f)
            != $crate::field::LcfField::element_count(&$d)
        {
            $crate::lcf_struct!(@emit_companion $s, $c,
                $crate::field::LcfField::element_count(&$self.$f));
        }
    };
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, count_persist = $c:literal) => {
        $crate::lcf_struct!(@emit_companion $s, $c,
            $crate::field::LcfField::element_count(&$self.$f));
    };
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, persist) => {};
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, persist2k) => {};
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, db_version) => {};
    (@companion $s:ident, $self:ident, $d:ident, $e:ident, $f:ident, is2k3) => {};

    (@emit_companion $s:ident, $c:literal, $payload:expr) => {{
        let payload = $payload;
        $crate::wire::write_varint($s, $c)?;
        $crate::wire::write_varint($s, $crate::wire::varint_len(payload))?;
        $crate::wire::write_varint($s, payload)?;
    }};
}

/// Define a bit-packed boolean bundle.
///
/// Flag bundles are the one place LCF really does pack bits, eight to a byte, least significant
/// first. Not to be confused with [`BitArray`](crate::field::BitArray), whose misleading liblcf
/// which stores one whole byte per boolean.
///
/// ```
/// # use rm2k::lcf_flags;
/// lcf_flags! {
///     /// RPG::TroopPageCondition flags.
///     pub struct TroopPageConditionFlags {
///         switch_a,
///         switch_b,
///         turn_enemy is2k3,
///     }
/// }
/// ```
///
/// Writing a 2000 file **skips** 2003-only flags entirely, which shifts every later flag down a
/// bit position. Reading does **not** skip them: it fills the fields
/// in declaration order from consecutive bits regardless of engine.
/// So a 2003 bundle written as 2000 and read back does not
/// round-trip its *values*. That is a liblcf bug, but files in the wild were produced by it, so
/// reproducing it exactly is what keeps *bytes* round-tripping - which is the property that
/// matters. Do not "fix" this without a corpus to check against.
#[macro_export]
macro_rules! lcf_flags {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$fmeta:meta])*
                $fname:ident $($f2k3:ident)?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        $vis struct $name {
            $(
                $(#[$fmeta])*
                pub $fname: bool,
            )*
        }

        $crate::impl_identity_owned!($name);

        impl $name {
            /// Every flag clear.
            ///
            /// A `const` rather than only `Default` so it can seed a struct-update expression in
            /// a const context - which is how the schema spells a bundle whose default is a raw
            /// bit pattern (`SavePicture::flags` defaults to 97).
            pub const DEFAULT: Self = Self { $( $fname: false, )* };
        }

        impl<'lcf> $crate::field::LcfField<'lcf> for $name {
            #[inline]
            fn field_default(_engine: $crate::engine::Engine) -> Self {
                Self::DEFAULT
            }

            fn read(
                data: &'lcf [u8],
                _ctx: &mut $crate::cursor::Ctx<'_>,
                _owner: &'static str,
                _id: u32,
            ) -> Self {
                let mut out = Self::default();
                let mut bit = 0usize;
                $(
                    // Bits past the end of the payload read as `false`, which is how liblcf's
                    // `byteidx >= length` break leaves them.
                    out.$fname = data
                        .get(bit / 8)
                        .is_some_and(|byte| (byte >> (bit % 8)) & 1 != 0);
                    bit += 1;
                )*
                let _ = bit;
                out
            }

            #[allow(unused_variables)]
            fn lcf_size(&self, engine: $crate::engine::Engine) -> u32 {
                let mut bits = 0u32;
                $(
                    if $crate::lcf_flags!(@allow engine $(, $f2k3)?) {
                        bits += 1;
                    }
                )*
                bits.div_ceil(8)
            }

            #[allow(unused_variables)]
            fn write<S: $crate::sink::Sink>(
                &self,
                sink: &mut S,
                engine: $crate::engine::Engine,
            ) -> ::core::result::Result<(), S::Error> {
                let mut byte = 0u8;
                let mut bit = 0u32;
                $(
                    if $crate::lcf_flags!(@allow engine $(, $f2k3)?) {
                        byte |= u8::from(self.$fname) << bit;
                        bit += 1;
                        if bit == 8 {
                            sink.write(&[byte])?;
                            byte = 0;
                            bit = 0;
                        }
                    }
                )*
                if bit != 0 {
                    sink.write(&[byte])?;
                }
                ::core::result::Result::Ok(())
            }
        }
    };

    (@allow $e:ident, is2k3) => { $e.is_2k3() };
    (@allow $e:ident) => { true };
}

/// Define the named constants for an LCF enumeration.
///
/// LCF stores every enumeration as a plain compressed integer and RPG Maker writes values this
/// schema does not know about, so this generates a transparent `i32` newtype plus associated
/// constants rather than a real `enum`. An unrecognised value is preserved exactly and written
/// back unchanged - a real `enum` would have to either reject it or lose it, and both break
/// round-tripping.
///
/// ```
/// # use rm2k::lcf_enum;
/// lcf_enum! {
///     /// RPG::Actor::Condition
///     pub struct ActorCondition {
///         NORMAL = 0,
///         POISON = 1,
///     }
/// }
/// # assert_eq!(ActorCondition::POISON.get(), 1);
/// # assert_eq!(ActorCondition(7).name(), None);
/// ```
#[macro_export]
macro_rules! lcf_enum {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$vmeta:meta])*
                $vname:ident = $value:literal
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        $vis struct $name(pub i32);

        $crate::impl_identity_owned!($name);

        impl $name {
            $(
                $(#[$vmeta])*
                pub const $vname: Self = Self($value);
            )*

            /// The underlying value, including one this schema does not name.
            #[must_use]
            #[inline]
            pub const fn get(self) -> i32 {
                self.0
            }

            /// The constant's name, or `None` if the value is not one this schema knows.
            #[must_use]
            pub const fn name(self) -> Option<&'static str> {
                match self.0 {
                    $( $value => Some(stringify!($vname)), )*
                    _ => None,
                }
            }
        }

        impl From<i32> for $name {
            #[inline]
            fn from(value: i32) -> Self {
                Self(value)
            }
        }

        impl<'lcf> $crate::field::LcfField<'lcf> for $name {
            #[inline]
            fn field_default(engine: $crate::engine::Engine) -> Self {
                Self(<i32 as $crate::field::LcfField<'lcf>>::field_default(engine))
            }

            #[inline]
            fn read(
                data: &'lcf [u8],
                ctx: &mut $crate::cursor::Ctx<'_>,
                owner: &'static str,
                id: u32,
            ) -> Self {
                Self(<i32 as $crate::field::LcfField<'lcf>>::read(data, ctx, owner, id))
            }

            #[inline]
            fn lcf_size(&self, engine: $crate::engine::Engine) -> u32 {
                $crate::field::LcfField::lcf_size(&self.0, engine)
            }

            #[inline]
            fn write<S: $crate::sink::Sink>(
                &self,
                sink: &mut S,
                engine: $crate::engine::Engine,
            ) -> ::core::result::Result<(), S::Error> {
                $crate::field::LcfField::write(&self.0, sink, engine)
            }
        }
    };
}

/// Declare which of a struct's fields hold text.
///
/// Generates [`Strings`](crate::field::Strings), which is what a translation tool needs to walk a
/// struct's text without naming all 127 fields of `Terms` by hand.
///
/// Separate from `lcf_struct!` because the macro cannot tell a `DbStr` field from an `i32` one -
/// field types are opaque `ty` fragments to it. The schema translator knows, so it emits this
/// alongside each struct.
///
/// ```
/// # use rm2k::{lcf_struct, lcf_strings, DbStr, UnknownChunks, field::Strings};
/// lcf_struct! {
///     #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
///     #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
///     pub struct Actor<'a> indexed {
///         0x01 name:  DbStr<'a>,
///         0x02 title: DbStr<'a>,
///         0x04 index: i32 = 0,
///     }
/// }
/// lcf_strings!(Actor<'a> { name, title });
///
/// # let actor = Actor { id: 1, name: DbStr::from_bytes(b"Alex"), title: DbStr::from_bytes(b"Hero"),
/// #                     index: 0, unknown_chunks: UnknownChunks::new() };
/// let mut found = Vec::new();
/// actor.visit_strings(&mut |field, value| found.push((field, value.as_bytes().to_vec())));
/// assert_eq!(found[0].0, "name");
/// ```
#[macro_export]
macro_rules! lcf_strings {
    ($name:ident<$lt:lifetime> { $($field:ident),* $(,)? }) => {
        impl<$lt> $crate::field::Strings<$lt> for $name<$lt> {
            fn visit_strings(
                &self,
                f: &mut dyn FnMut(&'static str, &$crate::field::DbStr<$lt>),
            ) {
                $( f(stringify!($field), &self.$field); )*
            }
        }
    };
}
