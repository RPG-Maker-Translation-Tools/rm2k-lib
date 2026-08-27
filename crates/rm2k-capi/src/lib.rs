#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use alloc::vec::Vec;
use core::{
    ffi::c_char,
    mem::{align_of, size_of},
    ptr,
};

// Wires `alloc`'s global allocator to a host-registered allocator, and supplies a panic handler -
// both required, and both absent, when this crate is built without its default `std` feature.
// With `std`, Rust's own allocator and unwinding machinery already cover this, so none of it is
// compiled in. See README.md's `no_std` section for the caller-facing contract.
#[cfg(not(feature = "std"))]
mod host_alloc {
    use core::{
        alloc::{GlobalAlloc, Layout},
        sync::atomic::{AtomicUsize, Ordering},
    };

    pub type Rm2kAllocFn = unsafe extern "C" fn(size: usize, align: usize) -> *mut u8;
    pub type Rm2kFreeFn = unsafe extern "C" fn(ptr: *mut u8, size: usize, align: usize);

    static ALLOC_FN: AtomicUsize = AtomicUsize::new(0);
    static FREE_FN: AtomicUsize = AtomicUsize::new(0);

    // SAFETY (caller contract, see README.md): `alloc`/`free` must be valid for the remaining
    // lifetime of the process, safe to call from any thread this library's functions are called
    // from, and `free` must accept exactly the `(ptr, size, align)` triples a prior call to
    // `alloc` handed out (same values, not just an equivalent layout).
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn rm2k_set_allocator(alloc: Rm2kAllocFn, free: Rm2kFreeFn) {
        ALLOC_FN.store(alloc as usize, Ordering::Release);
        FREE_FN.store(free as usize, Ordering::Release);
    }

    struct HostAlloc;

    unsafe impl GlobalAlloc for HostAlloc {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let f = ALLOC_FN.load(Ordering::Acquire);
            if f == 0 {
                return ptr::null_mut();
            }
            // SAFETY: the only value ever stored is an `Rm2kAllocFn` cast to `usize` by
            // `rm2k_set_allocator`.
            let alloc: Rm2kAllocFn = unsafe { core::mem::transmute(f) };
            unsafe { alloc(layout.size(), layout.align()) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let f = FREE_FN.load(Ordering::Acquire);
            if f == 0 {
                return;
            }
            // SAFETY: see `alloc` above; `rm2k_set_allocator` always sets both atomics together.
            let free: Rm2kFreeFn = unsafe { core::mem::transmute(f) };
            unsafe { free(ptr, layout.size(), layout.align()) };
        }
    }

    #[global_allocator]
    static HOST_ALLOC: HostAlloc = HostAlloc;

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        unsafe extern "C" {
            fn abort() -> !;
        }
        unsafe { abort() }
    }
}

#[cfg(not(feature = "std"))]
pub use host_alloc::rm2k_set_allocator;

use rm2k::cursor::{ChunkIter, Ctx, ListWalker};
use rm2k::diag::{Diagnostic, NullSink};
use rm2k::event::{CommandLists, EventCommandIter};
use rm2k::owned::IntoOwned;
use rm2k::{DiagKind, DiagSink, Engine, ReadError, SaveOpt, file};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kError {
    Ok = 0,
    UnexpectedEof = 1,
    BadMagic = 2,
    VarintOverflow = 3,
    ChunkOverrun = 4,
    Strict = 5,
    NullArgument = 6,
}

impl From<ReadError> for Rm2kError {
    fn from(err: ReadError) -> Self {
        match err {
            ReadError::UnexpectedEof { .. } => Self::UnexpectedEof,
            ReadError::BadMagic { .. } => Self::BadMagic,
            ReadError::VarintOverflow { .. } => Self::VarintOverflow,
            ReadError::ChunkOverrun { .. } => Self::ChunkOverrun,
            ReadError::Strict(_) => Self::Strict,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kEngine {
    R2K = 0,
    R2K3 = 1,
}

impl From<Rm2kEngine> for Engine {
    fn from(engine: Rm2kEngine) -> Self {
        match engine {
            Rm2kEngine::R2K => Self::R2K,
            Rm2kEngine::R2K3 => Self::R2K3,
        }
    }
}

impl From<Engine> for Rm2kEngine {
    fn from(engine: Engine) -> Self {
        match engine {
            Engine::R2K => Self::R2K,
            Engine::R2K3 => Self::R2K3,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kDbList {
    Actors = 0,
    Skills = 1,
    Items = 2,
    Enemies = 3,
    Troops = 4,
    States = 5,
    Classes = 6,
    Animations = 7,
    Chipsets = 8,
    Terrains = 9,
    Attributes = 10,
    CommonEvents = 11,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kBytes {
    pub data: *const u8,
    pub len: usize,
}

impl Rm2kBytes {
    fn new(bytes: &[u8]) -> Self {
        Self {
            data: bytes.as_ptr(),
            len: bytes.len(),
        }
    }

    const EMPTY: Self = Self {
        data: ptr::null(),
        len: 0,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kChunk {
    pub id: u32,
    pub offset: u32,
    pub data: Rm2kBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kListItem {
    pub id: i32,
    pub offset: u32,
    pub body: Rm2kBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kCommandList {
    pub owner_id: i32,
    // 0 for a common event, which has no page.
    pub page_id: i32,
    pub offset: u32,
    pub commands: Rm2kBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kEventCommand {
    pub code: i32,
    pub indent: i32,
    pub param_count: u32,
    pub string: Rm2kBytes,
    params: Rm2kBytes,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kDiagnostic {
    pub offset: u32,
    pub chunk_id: u32,
    pub kind: u8,
}

fn diag_kind_index(kind: DiagKind) -> u8 {
    match kind {
        DiagKind::UnknownChunk => 0,
        DiagKind::ShortChunk => 1,
        DiagKind::TrailingBytes => 2,
        DiagKind::BadVarint => 3,
        DiagKind::ChunkOverrun => 4,
        DiagKind::BadMagic => 5,
        DiagKind::EventListCorrupt => 6,
        DiagKind::RaggedArray => 7,
        // `DiagKind` is `#[non_exhaustive]`; anything new reports as "unknown chunk" rather than
        // silently taking on another kind's meaning.
        _ => 0,
    }
}

pub struct Rm2kDiagnostics(Vec<Diagnostic>);

#[unsafe(no_mangle)]
pub extern "C" fn rm2k_diagnostics_new() -> *mut Rm2kDiagnostics {
    alloc::boxed::Box::into_raw(alloc::boxed::Box::new(Rm2kDiagnostics(Vec::new())))
}

// SAFETY: `diagnostics` must have come from `rm2k_diagnostics_new` and not been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_diagnostics_free(diagnostics: *mut Rm2kDiagnostics) {
    if !diagnostics.is_null() {
        drop(unsafe { alloc::boxed::Box::from_raw(diagnostics) });
    }
}

// SAFETY: `diagnostics` must be `NULL` or a live collector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_diagnostics_count(diagnostics: *const Rm2kDiagnostics) -> usize {
    match unsafe { diagnostics.as_ref() } {
        Some(d) => d.0.len(),
        None => 0,
    }
}

// SAFETY: `diagnostics` must be `NULL` or a live collector; `out` must be writable or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_diagnostics_get(
    diagnostics: *const Rm2kDiagnostics,
    index: usize,
    out: *mut Rm2kDiagnostic,
) -> bool {
    let (Some(d), Some(out)) = (unsafe { diagnostics.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    let Some(entry) = d.0.get(index) else { return false };
    *out = Rm2kDiagnostic {
        offset: entry.offset,
        chunk_id: entry.chunk_id,
        kind: diag_kind_index(entry.kind),
    };
    true
}

// SAFETY: `diagnostics` must be `NULL` or a live collector.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_diagnostics_clear(diagnostics: *mut Rm2kDiagnostics) {
    if let Some(d) = unsafe { diagnostics.as_mut() } {
        d.0.clear();
    }
}

// The one place this API hands back a `char*`: these are fixed English labels this crate owns,
// not data out of a file, so a NUL-terminated string needs no allocation to produce.
#[unsafe(no_mangle)]
pub extern "C" fn rm2k_diagnostic_kind_name(kind: u8) -> *const c_char {
    const NAMES: [&str; 8] = [
        "unknown chunk\0",
        "chunk too short for field\0",
        "trailing bytes in chunk\0",
        "malformed compressed integer\0",
        "chunk length past end of parent\0",
        "unexpected file magic\0",
        "event command list not terminated\0",
        "array payload not a multiple of element size\0",
    ];
    let index = usize::from(kind).min(NAMES.len() - 1);
    NAMES[index].as_ptr().cast()
}

// SAFETY: the caller contract on every entry point is that `diagnostics` is NULL or live.
fn with_sink<R>(diagnostics: *mut Rm2kDiagnostics, body: impl FnOnce(&mut dyn DiagSink) -> R) -> R {
    match unsafe { diagnostics.as_mut() } {
        Some(collector) => body(&mut collector.0),
        None => body(&mut NullSink),
    }
}

// The engine is always 2000 here, and that is not a shortcut: no field's default depends on the
// engine, so reading never consults it. See the `rm2k::file` docs.
fn with_ctx<R>(diagnostics: *mut Rm2kDiagnostics, body: impl FnOnce(&mut Ctx<'_>) -> R) -> R {
    with_sink(diagnostics, |sink| {
        let mut ctx = Ctx::new(Engine::R2K, sink);
        body(&mut ctx)
    })
}

// SAFETY: `ptr`/`len` must be exactly what a `rm2k_*_save` call returned, freed once. The length
// is required because the allocation is reconstituted as a `Vec` to release it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_buffer_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { Vec::from_raw_parts(ptr, len, len) });
    }
}

fn leak_vec(mut bytes: Vec<u8>, out_len: *mut usize) -> *mut u8 {
    bytes.shrink_to_fit();
    // `shrink_to_fit` is not contractually exact, so use the real capacity as the length the
    // caller must hand back. They are equal in practice; relying on that would be a leak waiting
    // to happen.
    let len = bytes.len();
    debug_assert_eq!(bytes.capacity(), len);
    let ptr = bytes.as_mut_ptr();
    core::mem::forget(bytes);
    // SAFETY: the caller contract is that `out_len` is writable or NULL.
    if let Some(out) = unsafe { out_len.as_mut() } {
        *out = len;
    }
    ptr
}

// Generates the load/save/free/header quartet for one format. Returns NULL on load failure,
// writing the reason through out_error when it is non-NULL.
//
// SAFETY (the generated $load): `data` must point to `len` readable bytes. `out_error` must be
// writable or `NULL`. `diagnostics` must be `NULL` or a live collector.
macro_rules! owned_format {
    (
        $handle:ident, $inner:ty, $load:ident, $load_fn:path, $save:ident, $save_fn:path,
        $free:ident, $header:ident
    ) => {
        pub struct $handle {
            value: $inner,
            header: Vec<u8>,
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $load(
            data: *const u8,
            len: usize,
            diagnostics: *mut Rm2kDiagnostics,
            out_error: *mut Rm2kError,
        ) -> *mut $handle {
            let write_error = |error: Rm2kError| {
                // SAFETY: caller contract.
                if let Some(out) = unsafe { out_error.as_mut() } {
                    *out = error;
                }
            };

            if data.is_null() {
                write_error(Rm2kError::NullArgument);
                return ptr::null_mut();
            }
            // SAFETY: caller contract.
            let bytes = unsafe { core::slice::from_raw_parts(data, len) };

            let loaded = with_sink(diagnostics, |sink| $load_fn(bytes, sink));

            match loaded {
                Ok((value, header)) => {
                    write_error(Rm2kError::Ok);
                    alloc::boxed::Box::into_raw(alloc::boxed::Box::new($handle { value, header }))
                }
                Err(error) => {
                    write_error(error.into());
                    ptr::null_mut()
                }
            }
        }

        // SAFETY: `handle` must be live. `out_len` must be writable or `NULL`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $save(
            handle: *const $handle,
            engine: Rm2kEngine,
            preserve_header: bool,
            out_len: *mut usize,
        ) -> *mut u8 {
            // SAFETY: caller contract.
            let Some(handle) = (unsafe { handle.as_ref() }) else {
                return ptr::null_mut();
            };
            let mut out = Vec::new();
            let opt = SaveOpt { preserve_header };
            if $save_fn(&handle.value, &mut out, engine.into(), opt, &handle.header).is_err() {
                return ptr::null_mut();
            }
            leak_vec(out, out_len)
        }

        // SAFETY: `handle` must have come from the matching load function and not been freed
        // already.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $free(handle: *mut $handle) {
            if !handle.is_null() {
                drop(unsafe { alloc::boxed::Box::from_raw(handle) });
            }
        }

        // SAFETY: `handle` must be live.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $header(handle: *const $handle) -> Rm2kBytes {
            // SAFETY: caller contract.
            match unsafe { handle.as_ref() } {
                Some(handle) => Rm2kBytes::new(&handle.header),
                None => Rm2kBytes::EMPTY,
            }
        }
    };
}

macro_rules! loader {
    ($name:ident, $fn:path, $ty:ty) => {
        fn $name(bytes: &[u8], diag: &mut dyn rm2k::DiagSink) -> Result<($ty, Vec<u8>), ReadError> {
            let loaded = $fn(bytes, diag)?;
            let header = loaded.header.to_vec();
            Ok((loaded.value.into_owned(), header))
        }
    };
}

loader!(load_database, file::load_database_with, rm2k::rpg::Database<'static>);
loader!(load_map, file::load_map_with, rm2k::rpg::Map<'static>);
loader!(load_tree_map, file::load_tree_map_with, rm2k::rpg::TreeMap<'static>);
loader!(load_save, file::load_save_with, rm2k::rpg::Save<'static>);

owned_format! {
    Rm2kDatabase, rm2k::rpg::Database<'static>,
    rm2k_database_load, load_database,
    rm2k_database_save, file::save_database,
    rm2k_database_free, rm2k_database_header
}

owned_format! {
    Rm2kMap, rm2k::rpg::Map<'static>,
    rm2k_map_load, load_map,
    rm2k_map_save, file::save_map,
    rm2k_map_free, rm2k_map_header
}

owned_format! {
    Rm2kTreeMap, rm2k::rpg::TreeMap<'static>,
    rm2k_tree_map_load, load_tree_map,
    rm2k_tree_map_save, file::save_tree_map,
    rm2k_tree_map_free, rm2k_tree_map_header
}

owned_format! {
    Rm2kSave, rm2k::rpg::Save<'static>,
    rm2k_save_load, load_save,
    rm2k_save_save, file::save_save,
    rm2k_save_free, rm2k_save_header
}

// SAFETY: `handle` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_database_engine(handle: *const Rm2kDatabase) -> Rm2kEngine {
    match unsafe { handle.as_ref() } {
        Some(handle) => Engine::from_ldb_id(handle.value.system.ldb_id).into(),
        None => Rm2kEngine::R2K,
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kNamed {
    pub id: i32,
    pub name: Rm2kBytes,
}

fn with_list<R>(
    db: &rm2k::rpg::Database<'static>,
    list: Rm2kDbList,
    f: impl FnOnce(&mut dyn Iterator<Item = (i32, &[u8])>) -> R,
) -> R {
    macro_rules! go {
        ($field:ident) => {
            f(&mut db.$field.iter().map(|e| (e.id, e.name.as_bytes())))
        };
    }
    match list {
        Rm2kDbList::Actors => go!(actors),
        Rm2kDbList::Skills => go!(skills),
        Rm2kDbList::Items => go!(items),
        Rm2kDbList::Enemies => go!(enemies),
        Rm2kDbList::Troops => go!(troops),
        Rm2kDbList::States => go!(states),
        Rm2kDbList::Classes => go!(classes),
        Rm2kDbList::Animations => go!(animations),
        Rm2kDbList::Chipsets => go!(chipsets),
        Rm2kDbList::Terrains => go!(terrains),
        Rm2kDbList::Attributes => go!(attributes),
        Rm2kDbList::CommonEvents => go!(commonevents),
    }
}

// SAFETY: `handle` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_database_list_len(handle: *const Rm2kDatabase, list: Rm2kDbList) -> usize {
    match unsafe { handle.as_ref() } {
        // A closure rather than `Iterator::count`: the higher-ranked bound on `with_list` needs a
        // callable that is generic over the iterator's lifetime, and a bare method path is not.
        Some(handle) => with_list(&handle.value, list, |entries| entries.count()),
        None => 0,
    }
}

// The returned name borrows from the handle and is valid until it is freed.
//
// SAFETY: `handle` must be live; `out` must be writable or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_database_list_get(
    handle: *const Rm2kDatabase,
    list: Rm2kDbList,
    index: usize,
    out: *mut Rm2kNamed,
) -> bool {
    let (Some(handle), Some(out)) = (unsafe { handle.as_ref() }, unsafe { out.as_mut() }) else {
        return false;
    };
    with_list(&handle.value, list, |entries| match entries.nth(index) {
        Some((id, name)) => {
            *out = Rm2kNamed {
                id,
                name: Rm2kBytes::new(name),
            };
            true
        }
        None => false,
    })
}

// Defines a C-visible inline struct that stores a Rust walker in the caller's memory - opaque:
// initialize it with the matching `_init` function and pass it by pointer, and it borrows the
// buffer it was initialised from, so that buffer must outlive it.
//
// The alternative would be boxing each walker, which would put an allocation on the one path
// whose whole point is not having one. The size is asserted at compile time, so growing the Rust
// type is a build error here rather than memory corruption at the call site.
macro_rules! opaque_inline {
    ($name:ident, $inner:ty, $bytes:literal) => {
        #[repr(C, align(8))]
        #[derive(Clone, Copy)]
        pub struct $name {
            _opaque: [u8; $bytes],
        }

        const _: () = {
            assert!(size_of::<$inner>() <= $bytes, "opaque buffer too small; raise it");
            assert!(align_of::<$inner>() <= 8);
        };

        impl $name {
            // SAFETY: must have been initialised by the matching `_init`, and the buffer it was
            // initialised from must still be alive.
            unsafe fn get<'a>(this: *mut Self) -> Option<&'a mut $inner> {
                unsafe { this.cast::<$inner>().as_mut() }
            }

            unsafe fn put(this: *mut Self, value: $inner) {
                unsafe { this.cast::<$inner>().write(value) };
            }
        }
    };
}

opaque_inline!(Rm2kChunkIter, ChunkIter<'static>, 64);
opaque_inline!(Rm2kListWalker, ListWalker<'static>, 96);
opaque_inline!(Rm2kCommandLists, CommandLists<'static>, 256);
opaque_inline!(Rm2kEventCommands, EventCommandIter<'static>, 64);

// SAFETY: `data` must point to `len` readable bytes that outlive every walker built from them.
// This is the one contract the zero-copy API cannot check, and it is stated on every entry point.
unsafe fn slice<'a>(data: *const u8, len: usize) -> Option<&'a [u8]> {
    if data.is_null() {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(data, len) })
}

// The zero-copy walkers operate on a *body*, not on a whole file, so this is normally the first
// call.
//
// SAFETY: `data` must point to `len` readable bytes; `out_body` and `out_header` must be writable
// or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_split_header(
    data: *const u8,
    len: usize,
    out_header: *mut Rm2kBytes,
    out_body: *mut Rm2kBytes,
) -> bool {
    let Some(bytes) = (unsafe { slice(data, len) }) else {
        return false;
    };
    let mut cursor = rm2k::Cursor::new(bytes);
    let Ok(header) = file::read_magic(&mut cursor) else {
        return false;
    };

    if let Some(out) = unsafe { out_header.as_mut() } {
        *out = Rm2kBytes::new(header);
    }
    if let Some(out) = unsafe { out_body.as_mut() } {
        *out = Rm2kBytes::new(cursor.remaining());
    }
    true
}

// SAFETY: `iter` must be writable; `data` must point to `len` readable bytes that outlive `iter`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_chunk_iter_init(
    iter: *mut Rm2kChunkIter,
    data: *const u8,
    len: usize,
    base: u32,
) -> bool {
    let (false, Some(bytes)) = (iter.is_null(), unsafe { slice(data, len) }) else {
        return false;
    };
    unsafe { Rm2kChunkIter::put(iter, ChunkIter::new(bytes, base)) };
    true
}

// SAFETY: `iter` must have been initialised and its buffer still alive; `out` must be writable or
// `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_chunk_iter_next(
    iter: *mut Rm2kChunkIter,
    diagnostics: *mut Rm2kDiagnostics,
    out: *mut Rm2kChunk,
) -> bool {
    let Some(iter) = (unsafe { Rm2kChunkIter::get(iter) }) else {
        return false;
    };
    let found = with_ctx(diagnostics, |ctx| iter.next(ctx, "C"));
    let Some(chunk) = found else { return false };
    if let Some(out) = unsafe { out.as_mut() } {
        *out = Rm2kChunk {
            id: chunk.id,
            offset: chunk.offset,
            data: Rm2kBytes::new(chunk.data),
        };
    }
    true
}

// SAFETY: `data` must point to `len` readable bytes; `out` must be writable or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_chunk_find(
    data: *const u8,
    len: usize,
    base: u32,
    id: u32,
    diagnostics: *mut Rm2kDiagnostics,
    out: *mut Rm2kChunk,
) -> bool {
    let Some(bytes) = (unsafe { slice(data, len) }) else {
        return false;
    };
    let found = with_ctx(diagnostics, |ctx| ChunkIter::find(bytes, base, id, ctx, "C"));
    let Some(chunk) = found else { return false };
    if let Some(out) = unsafe { out.as_mut() } {
        *out = Rm2kChunk {
            id: chunk.id,
            offset: chunk.offset,
            data: Rm2kBytes::new(chunk.data),
        };
    }
    true
}

// SAFETY: `walker` must be writable; `data` must point to `len` readable bytes that outlive
// `walker`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_list_walker_init(
    walker: *mut Rm2kListWalker,
    data: *const u8,
    len: usize,
    base: u32,
    diagnostics: *mut Rm2kDiagnostics,
) -> bool {
    let (false, Some(bytes)) = (walker.is_null(), unsafe { slice(data, len) }) else {
        return false;
    };
    let inner = with_ctx(diagnostics, |ctx| ListWalker::new(bytes, base, ctx, "C"));
    unsafe { Rm2kListWalker::put(walker, inner) };
    true
}

// SAFETY: `walker` must have been initialised and its buffer still alive; `out` must be writable
// or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_list_walker_next(
    walker: *mut Rm2kListWalker,
    diagnostics: *mut Rm2kDiagnostics,
    out: *mut Rm2kListItem,
) -> bool {
    let Some(walker) = (unsafe { Rm2kListWalker::get(walker) }) else {
        return false;
    };
    let found = with_ctx(diagnostics, |ctx| walker.next(ctx));
    let Some(item) = found else { return false };
    if let Some(out) = unsafe { out.as_mut() } {
        *out = Rm2kListItem {
            id: item.id,
            offset: item.offset,
            body: Rm2kBytes::new(item.body),
        };
    }
    true
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kEventScope {
    // From an LMU body.
    MapEvents = 0,
    // From an LDB body.
    CommonEvents = 1,
    // From an LDB body.
    BattleEvents = 2,
}

// `data` is a *body* - the bytes after the file's magic, as produced by `rm2k_split_header`.
//
// SAFETY: `lists` must be writable; `data` must point to `len` readable bytes that outlive
// `lists`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_command_lists_init(
    lists: *mut Rm2kCommandLists,
    scope: Rm2kEventScope,
    data: *const u8,
    len: usize,
    base: u32,
    diagnostics: *mut Rm2kDiagnostics,
) -> bool {
    let (false, Some(bytes)) = (lists.is_null(), unsafe { slice(data, len) }) else {
        return false;
    };
    let inner = with_ctx(diagnostics, |ctx| match scope {
        Rm2kEventScope::MapEvents => CommandLists::in_map(bytes, base, ctx),
        Rm2kEventScope::CommonEvents => CommandLists::in_common_events(bytes, base, ctx),
        Rm2kEventScope::BattleEvents => CommandLists::in_battle_events(bytes, base, ctx),
    });
    unsafe { Rm2kCommandLists::put(lists, inner) };
    true
}

// SAFETY: `lists` must have been initialised and its buffer still alive; `out` must be writable
// or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_command_lists_next(
    lists: *mut Rm2kCommandLists,
    diagnostics: *mut Rm2kDiagnostics,
    out: *mut Rm2kCommandList,
) -> bool {
    let Some(lists) = (unsafe { Rm2kCommandLists::get(lists) }) else {
        return false;
    };
    let found = with_ctx(diagnostics, |ctx| lists.next(ctx));
    let Some(list) = found else { return false };
    if let Some(out) = unsafe { out.as_mut() } {
        *out = Rm2kCommandList {
            owner_id: list.owner_id,
            page_id: list.page_id,
            offset: list.offset,
            commands: Rm2kBytes::new(list.commands),
        };
    }
    true
}

// SAFETY: `iter` must be writable; `data` must point to `len` readable bytes that outlive `iter`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_event_commands_init(
    iter: *mut Rm2kEventCommands,
    data: *const u8,
    len: usize,
    base: u32,
) -> bool {
    let (false, Some(bytes)) = (iter.is_null(), unsafe { slice(data, len) }) else {
        return false;
    };
    unsafe { Rm2kEventCommands::put(iter, EventCommandIter::new(bytes, base)) };
    true
}

// SAFETY: `iter` must have been initialised and its buffer still alive; `out` must be writable or
// `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_event_commands_next(
    iter: *mut Rm2kEventCommands,
    diagnostics: *mut Rm2kDiagnostics,
    out: *mut Rm2kEventCommand,
) -> bool {
    let Some(iter) = (unsafe { Rm2kEventCommands::get(iter) }) else {
        return false;
    };
    let found = with_ctx(diagnostics, |ctx| iter.next(ctx));
    let Some(command) = found else { return false };
    if let Some(out) = unsafe { out.as_mut() } {
        *out = Rm2kEventCommand {
            code: command.code,
            indent: command.indent,
            param_count: command.param_count(),
            string: Rm2kBytes::new(command.string),
            params: Rm2kBytes::new(command.raw_params()),
        };
    }
    true
}

// O(index): compressed integers are not fixed width, so there is no way to index directly.
//
// SAFETY: `command` must point to a command filled in by `rm2k_event_commands_next`, whose buffer
// is still alive; `out` must be writable or `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rm2k_event_command_param(
    command: *const Rm2kEventCommand,
    index: usize,
    out: *mut i32,
) -> bool {
    let Some(command) = (unsafe { command.as_ref() }) else {
        return false;
    };
    let Some(bytes) = (unsafe { slice(command.params.data, command.params.len) }) else {
        return false;
    };
    let mut cursor = rm2k::Cursor::new(bytes);
    let mut left = command.param_count;
    let mut at = 0usize;
    while left > 0 {
        let Some(value) = cursor.varint() else { return false };
        if at == index {
            if let Some(out) = unsafe { out.as_mut() } {
                *out = value.value as i32;
            }
            return true;
        }
        at += 1;
        left -= 1;
    }
    false
}

// The same eight ids as `rm2k::event::path`, exposed so a C consumer can navigate by hand instead
// of only through `rm2k_command_lists_init`.
pub mod chunk_id {
    use rm2k::event::path;

    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_MAP_EVENTS: u32 = path::MAP_EVENTS;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_EVENT_PAGES: u32 = path::EVENT_PAGES;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_EVENT_PAGE_COMMANDS: u32 = path::EVENT_PAGE_COMMANDS;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_DATABASE_COMMON_EVENTS: u32 = path::DATABASE_COMMON_EVENTS;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_COMMON_EVENT_COMMANDS: u32 = path::COMMON_EVENT_COMMANDS;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_DATABASE_TROOPS: u32 = path::DATABASE_TROOPS;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_TROOP_PAGES: u32 = path::TROOP_PAGES;
    #[unsafe(no_mangle)]
    pub static RM2K_CHUNK_TROOP_PAGE_COMMANDS: u32 = path::TROOP_PAGE_COMMANDS;
}
