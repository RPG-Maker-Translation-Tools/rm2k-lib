#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

extern crate alloc;

use alloc::vec::Vec;
use core::{
    ffi::c_char,
    mem::{align_of, size_of},
    ptr,
};

/// Host allocation hook: returns a pointer to at least `size` bytes aligned to `align` (a power of
/// two), or `NULL` on failure. Not assumed to be `malloc`; see `rm2k_set_allocator`.
pub type Rm2kAllocFn = unsafe extern "C" fn(size: usize, align: usize) -> *mut u8;

/// Host deallocation hook: gets back exactly the `(ptr, size, align)` triple a prior `Rm2kAllocFn`
/// call produced. See `rm2k_set_allocator`.
pub type Rm2kFreeFn = unsafe extern "C" fn(ptr: *mut u8, size: usize, align: usize);

/// Registers the host's allocator.
///
/// Must be called exactly once, before any other `rm2k_*` function that allocates, in a `no_std`
/// build. No effect, and not needed, in the default `std` build, which uses Rust's allocator. See
/// the README's `no_std` contract.
///
/// # Safety
///
/// `alloc`/`free` must be valid for the remaining lifetime of the process, safe to call from any
/// thread this library's functions are called from, and `free` must accept exactly the
/// `(ptr, size, align)` triples a prior call to `alloc` handed out (same values, not just an
/// equivalent layout).
#[no_mangle]
pub unsafe extern "C" fn rm2k_set_allocator(alloc: Rm2kAllocFn, free: Rm2kFreeFn) {
    #[cfg(not(feature = "std"))]
    host_alloc::set(alloc, free);
    #[cfg(feature = "std")]
    let _ = (alloc, free);
}

// The allocator and panic handler a `no_std` build needs. The `std` build has Rust's own.
#[cfg(not(feature = "std"))]
mod host_alloc {
    use super::{Rm2kAllocFn, Rm2kFreeFn};
    use core::{
        alloc::{GlobalAlloc, Layout},
        ptr,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static ALLOC_FN: AtomicUsize = AtomicUsize::new(0);
    static FREE_FN: AtomicUsize = AtomicUsize::new(0);

    pub fn set(alloc: Rm2kAllocFn, free: Rm2kFreeFn) {
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
            // SAFETY: the only value ever stored is an `Rm2kAllocFn` cast to `usize` by `set`.
            let alloc: Rm2kAllocFn = unsafe { core::mem::transmute(f) };
            unsafe { alloc(layout.size(), layout.align()) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let f = FREE_FN.load(Ordering::Acquire);
            if f == 0 {
                return;
            }
            // SAFETY: see `alloc` above; `set` always sets both atomics together.
            let free: Rm2kFreeFn = unsafe { core::mem::transmute(f) };
            unsafe { free(ptr, layout.size(), layout.align()) };
        }
    }

    #[global_allocator]
    static HOST_ALLOC: HostAlloc = HostAlloc;

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        extern "C" {
            fn abort() -> !;
        }
        unsafe { abort() }
    }

    // The prebuilt `alloc` references the unwinder's personality even under `panic = "abort"`, and ELF linkers
    // (not MSVC's) need it defined. Nothing unwinds here, because the panic handler aborts, so it is never called.
    #[no_mangle]
    pub extern "C" fn rust_eh_personality() {}
}

use rm2k::cursor::{ChunkIter, Ctx, ListWalker};
use rm2k::diag::{Diagnostic, NullSink};
use rm2k::event::{CommandLists, EventCommandIter};
use rm2k::owned::IntoOwned;
use rm2k::{DiagKind, DiagSink, Engine, ReadError, SaveOpt, file};

/// What went wrong while loading, or `RM2K_ERROR_OK`.
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

/// The RPG Maker version a file belongs to.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kEngine {
    /// RPG Maker 2000.
    R2K = 0,
    /// RPG Maker 2003.
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

/// One of the database's named lists; see `rm2k_database_list_len`.
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

/// A borrowed run of bytes. `data` is `NULL` when `len` is `0`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kBytes {
    pub data: *const u8,
    pub len: usize,
}

impl Rm2kBytes {
    const fn new(bytes: &[u8]) -> Self {
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

/// One chunk of a chunk stream.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kChunk {
    /// The chunk's field id.
    pub id: u32,
    /// File-absolute offset of the payload.
    pub offset: u32,
    pub data: Rm2kBytes,
}

/// One element of an indexed list.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kListItem {
    pub id: i32,
    pub offset: u32,
    /// The element's chunk stream.
    pub body: Rm2kBytes,
}

/// One event command list, found by `rm2k_command_lists_next`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kCommandList {
    /// The owning event, common event or troop.
    pub owner_id: i32,
    /// The owning page, or `0` for a common event, which has none.
    pub page_id: i32,
    pub offset: u32,
    /// Feed to `rm2k_event_commands_init`.
    pub commands: Rm2kBytes,
}

/// One event command, found by `rm2k_event_commands_next`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kEventCommand {
    pub code: i32,
    pub indent: i32,
    pub param_count: u32,
    /// The command's string, verbatim: strings are raw bytes in the game's code page.
    pub string: Rm2kBytes,
    /// Private; read the parameters via `rm2k_event_command_param`.
    pub raw_params: Rm2kBytes,
}

/// One recovered problem, found by `rm2k_diagnostics_get`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Rm2kDiagnostic {
    pub offset: u32,
    pub chunk_id: u32,
    /// One of the `RM2K_DIAG_*` values.
    pub kind: u8,
    /// The struct the problem was found in. A static label, valid for the life of the process.
    pub struct_name: Rm2kBytes,
}

// SAFETY: `struct_name` only ever points at a `&'static str` (`rm2k::Diagnostic::struct_name` is `'static`),
// which is immutable for the life of the process. Sharing or moving the pointer across threads is therefore sound.
// The fields are `pub`, so a Rust caller who builds one by hand must keep to the same rule.
unsafe impl Send for Rm2kDiagnostic {}
// SAFETY: as for `Send`: the pointee is immutable static data, so concurrent reads cannot race.
unsafe impl Sync for Rm2kDiagnostic {}

/// Diagnostic kind: a chunk whose id the schema does not know; kept and written back verbatim.
///
/// These mirror `rm2k::DiagKind`; `rm2k_diagnostic_kind_name` labels them.
pub const RM2K_DIAG_UNKNOWN_CHUNK: u8 = 0;
/// Diagnostic kind: a chunk too short for the field it belongs to.
pub const RM2K_DIAG_SHORT_CHUNK: u8 = 1;
/// Diagnostic kind: bytes left over at the end of a chunk after its field was read.
pub const RM2K_DIAG_TRAILING_BYTES: u8 = 2;
/// Diagnostic kind: a malformed compressed integer.
pub const RM2K_DIAG_BAD_VARINT: u8 = 3;
/// Diagnostic kind: a chunk whose length runs past the end of its parent.
pub const RM2K_DIAG_CHUNK_OVERRUN: u8 = 4;
/// Diagnostic kind: an unexpected file magic string.
pub const RM2K_DIAG_BAD_MAGIC: u8 = 5;
/// Diagnostic kind: an event command list that is not terminated.
pub const RM2K_DIAG_EVENT_LIST_CORRUPT: u8 = 6;
/// Diagnostic kind: an array payload whose length is not a multiple of its element size.
pub const RM2K_DIAG_RAGGED_ARRAY: u8 = 7;
/// Diagnostic kind: a string list ran past the load's entry budget, so the rest of that list was dropped.
pub const RM2K_DIAG_ENTRY_BUDGET_EXHAUSTED: u8 = 8;

const fn diag_kind_index(kind: DiagKind) -> u8 {
    match kind {
        DiagKind::ShortChunk => RM2K_DIAG_SHORT_CHUNK,
        DiagKind::TrailingBytes => RM2K_DIAG_TRAILING_BYTES,
        DiagKind::BadVarint => RM2K_DIAG_BAD_VARINT,
        DiagKind::ChunkOverrun => RM2K_DIAG_CHUNK_OVERRUN,
        DiagKind::BadMagic => RM2K_DIAG_BAD_MAGIC,
        DiagKind::EventListCorrupt => RM2K_DIAG_EVENT_LIST_CORRUPT,
        DiagKind::RaggedArray => RM2K_DIAG_RAGGED_ARRAY,
        DiagKind::EntryBudgetExhausted => RM2K_DIAG_ENTRY_BUDGET_EXHAUSTED,
        // `DiagKind` is `#[non_exhaustive]`: unknown chunks and kinds added later both report as "unknown chunk",
        // never as another kind's label.
        _ => RM2K_DIAG_UNKNOWN_CHUNK,
    }
}

/// Opaque collector of recovered problems. Release it with `rm2k_diagnostics_free`.
pub struct Rm2kDiagnostics(Vec<Diagnostic>);

/// Creates an empty collector. Release it with `rm2k_diagnostics_free`.
///
/// Diagnostics are optional everywhere: pass `NULL` wherever an `Rm2kDiagnostics*` is taken to
/// discard them. That is also the only fully allocation-free option.
#[no_mangle]
pub extern "C" fn rm2k_diagnostics_new() -> *mut Rm2kDiagnostics {
    alloc::boxed::Box::into_raw(alloc::boxed::Box::new(Rm2kDiagnostics(Vec::new())))
}

/// Releases a collector. `NULL` is a no-op.
///
/// # Safety
///
/// `diagnostics` must have come from `rm2k_diagnostics_new` and not been freed already.
#[no_mangle]
pub unsafe extern "C" fn rm2k_diagnostics_free(diagnostics: *mut Rm2kDiagnostics) {
    if !diagnostics.is_null() {
        drop(unsafe { alloc::boxed::Box::from_raw(diagnostics) });
    }
}

/// The number of recorded diagnostics.
///
/// # Safety
///
/// `diagnostics` must be `NULL` or a live collector.
#[no_mangle]
pub unsafe extern "C" fn rm2k_diagnostics_count(diagnostics: *const Rm2kDiagnostics) -> usize {
    unsafe { diagnostics.as_ref() }.map_or(0, |d| d.0.len())
}

/// Writes diagnostic `index` to `out` and returns `true`, or returns `false` when `index` is out of
/// range.
///
/// # Safety
///
/// `diagnostics` must be `NULL` or a live collector; `out` must be writable or `NULL`.
#[no_mangle]
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
        struct_name: Rm2kBytes::new(entry.struct_name.as_bytes()),
    };
    true
}

/// Discards every recorded diagnostic.
///
/// # Safety
///
/// `diagnostics` must be `NULL` or a live collector.
#[no_mangle]
pub unsafe extern "C" fn rm2k_diagnostics_clear(diagnostics: *mut Rm2kDiagnostics) {
    if let Some(d) = unsafe { diagnostics.as_mut() } {
        d.0.clear();
    }
}

/// Fixed English label for a diagnostic kind (one of the `RM2K_DIAG_*` values).
///
/// NUL-terminated, static, never freed. The only `char*` this API returns: the labels are this
/// crate's, not file data, so no allocation is needed.
#[no_mangle]
pub extern "C" fn rm2k_diagnostic_kind_name(kind: u8) -> *const c_char {
    const NAMES: [&str; 9] = [
        "unknown chunk\0",
        "chunk too short for field\0",
        "trailing bytes in chunk\0",
        "malformed compressed integer\0",
        "chunk length past end of parent\0",
        "unexpected file magic\0",
        "event command list not terminated\0",
        "array payload not a multiple of element size\0",
        "string list over the load's entry budget\0",
    ];
    // An unknown code gets the same label as an unknown kind: "unknown chunk", matching `diag_kind_index`.
    NAMES.get(usize::from(kind)).unwrap_or(&NAMES[0]).as_ptr().cast()
}

// SAFETY: the caller contract on every entry point is that `diagnostics` is NULL or live.
fn with_sink<R>(diagnostics: *mut Rm2kDiagnostics, body: impl FnOnce(&mut dyn DiagSink) -> R) -> R {
    match unsafe { diagnostics.as_mut() } {
        Some(collector) => body(&mut collector.0),
        None => body(&mut NullSink),
    }
}

// Always 2000: no field's default depends on the engine, so reading never consults it. See `rm2k::file`.
fn with_ctx<R>(diagnostics: *mut Rm2kDiagnostics, body: impl FnOnce(&mut Ctx<'_>) -> R) -> R {
    with_sink(diagnostics, |sink| {
        let mut ctx = Ctx::new(Engine::R2K, sink);
        body(&mut ctx)
    })
}

/// Releases a buffer from `rm2k_*_save`. The length must be the one that call reported.
///
/// # Safety
///
/// `ptr`/`len` must be exactly what a `rm2k_*_save` call returned, freed once. The length
/// is required because the allocation is reconstituted as a boxed slice to release it.
#[no_mangle]
pub unsafe extern "C" fn rm2k_buffer_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(unsafe { alloc::boxed::Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, len)) });
    }
}

fn leak_vec(bytes: Vec<u8>, out_len: *mut usize) -> *mut u8 {
    // `into_boxed_slice` shrinks the allocation to exactly its length, which is what `rm2k_buffer_free` frees.
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = alloc::boxed::Box::into_raw(boxed).cast::<u8>();
    // SAFETY: the caller contract is that `out_len` is writable or NULL.
    if let Some(out) = unsafe { out_len.as_mut() } {
        *out = len;
    }
    ptr
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

/// A loaded file together with the magic string it was read with.
///
/// Implemented by the four handle types below so the load/save/free/header quartet is written once.
trait Handle: Sized {
    type Value;

    fn new(value: Self::Value, header: Vec<u8>) -> Self;
    fn value(&self) -> &Self::Value;
    fn header(&self) -> &[u8];
}

macro_rules! handle {
    ($handle:ident, $value:ty) => {
        impl Handle for $handle {
            type Value = $value;

            fn new(value: $value, header: Vec<u8>) -> Self {
                Self { value, header }
            }

            fn value(&self) -> &$value {
                &self.value
            }

            fn header(&self) -> &[u8] {
                &self.header
            }
        }
    };
}

/// Opaque handle to a loaded `RPG_RT.ldb` database. Release it with `rm2k_database_free`.
pub struct Rm2kDatabase {
    value: rm2k::rpg::Database<'static>,
    header: Vec<u8>,
}

handle!(Rm2kDatabase, rm2k::rpg::Database<'static>);

/// Opaque handle to a loaded `MapNNNN.lmu` map. Release it with `rm2k_map_free`.
pub struct Rm2kMap {
    value: rm2k::rpg::Map<'static>,
    header: Vec<u8>,
}

handle!(Rm2kMap, rm2k::rpg::Map<'static>);

/// Opaque handle to a loaded `RPG_RT.lmt` map tree. Release it with `rm2k_tree_map_free`.
pub struct Rm2kTreeMap {
    value: rm2k::rpg::TreeMap<'static>,
    header: Vec<u8>,
}

handle!(Rm2kTreeMap, rm2k::rpg::TreeMap<'static>);

/// Opaque handle to a loaded `.lsd` save file. Release it with `rm2k_save_free`.
pub struct Rm2kSave {
    value: rm2k::rpg::Save<'static>,
    header: Vec<u8>,
}

handle!(Rm2kSave, rm2k::rpg::Save<'static>);

// SAFETY (the four helpers below): the caller contracts documented on the exported wrappers.

unsafe fn load_handle<H: Handle>(
    data: *const u8,
    len: usize,
    diagnostics: *mut Rm2kDiagnostics,
    out_error: *mut Rm2kError,
    load: impl FnOnce(&[u8], &mut dyn DiagSink) -> Result<(H::Value, Vec<u8>), ReadError>,
) -> *mut H {
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

    match with_sink(diagnostics, |sink| load(bytes, sink)) {
        Ok((value, header)) => {
            write_error(Rm2kError::Ok);
            alloc::boxed::Box::into_raw(alloc::boxed::Box::new(H::new(value, header)))
        }
        Err(error) => {
            write_error(error.into());
            ptr::null_mut()
        }
    }
}

unsafe fn save_handle<H: Handle, E>(
    handle: *const H,
    engine: Rm2kEngine,
    preserve_header: bool,
    out_len: *mut usize,
    save: impl FnOnce(&H::Value, &mut Vec<u8>, Engine, SaveOpt, &[u8]) -> Result<(), E>,
) -> *mut u8 {
    // SAFETY: caller contract.
    let Some(handle) = (unsafe { handle.as_ref() }) else {
        return ptr::null_mut();
    };
    let mut out = Vec::new();
    let opt = SaveOpt { preserve_header };
    if save(handle.value(), &mut out, engine.into(), opt, handle.header()).is_err() {
        return ptr::null_mut();
    }
    leak_vec(out, out_len)
}

unsafe fn free_handle<H>(handle: *mut H) {
    if !handle.is_null() {
        // SAFETY: caller contract.
        drop(unsafe { alloc::boxed::Box::from_raw(handle) });
    }
}

unsafe fn handle_header<H: Handle>(handle: *const H) -> Rm2kBytes {
    // SAFETY: caller contract.
    unsafe { handle.as_ref() }.map_or(Rm2kBytes::EMPTY, |handle| Rm2kBytes::new(handle.header()))
}

/// Loads a `RPG_RT.ldb` database. Returns `NULL` on failure, with the reason in `out_error` if
/// non-`NULL`.
///
/// Free with `rm2k_database_free`. `diagnostics` may be `NULL` to discard recovered problems.
///
/// # Safety
///
/// `data` must be `NULL` or point to at least `len` readable bytes. `diagnostics` must be `NULL` or
/// a live collector from `rm2k_diagnostics_new`. `out_error`, if non-null, must be writable.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_load(
    data: *const u8,
    len: usize,
    diagnostics: *mut Rm2kDiagnostics,
    out_error: *mut Rm2kError,
) -> *mut Rm2kDatabase {
    unsafe { load_handle(data, len, diagnostics, out_error, load_database) }
}

/// Writes the loaded `RPG_RT.ldb` database back for `engine`. Returns an owned buffer, or `NULL`.
///
/// Free with `rm2k_buffer_free`, passing the length written to `out_len`. `preserve_header` writes
/// the magic string back as it was read.
///
/// # Safety
///
/// `handle` must be live. `out_len` must be writable or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_save(
    handle: *const Rm2kDatabase,
    engine: Rm2kEngine,
    preserve_header: bool,
    out_len: *mut usize,
) -> *mut u8 {
    unsafe { save_handle(handle, engine, preserve_header, out_len, file::save_database) }
}

/// Releases a `Rm2kDatabase` handle. `NULL` is a no-op.
///
/// # Safety
///
/// `handle` must have come from `rm2k_database_load` and not been freed already.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_free(handle: *mut Rm2kDatabase) {
    unsafe { free_handle(handle) }
}

/// The file's magic string as it was read; borrows from the handle.
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_header(handle: *const Rm2kDatabase) -> Rm2kBytes {
    unsafe { handle_header(handle) }
}

/// Loads a `MapNNNN.lmu` map. Returns `NULL` on failure, with the reason in `out_error` if
/// non-`NULL`.
///
/// Free with `rm2k_map_free`. `diagnostics` may be `NULL` to discard recovered problems.
///
/// # Safety
///
/// `data` must be `NULL` or point to at least `len` readable bytes. `diagnostics` must be `NULL` or
/// a live collector from `rm2k_diagnostics_new`. `out_error`, if non-null, must be writable.
#[no_mangle]
pub unsafe extern "C" fn rm2k_map_load(
    data: *const u8,
    len: usize,
    diagnostics: *mut Rm2kDiagnostics,
    out_error: *mut Rm2kError,
) -> *mut Rm2kMap {
    unsafe { load_handle(data, len, diagnostics, out_error, load_map) }
}

/// Writes the loaded `MapNNNN.lmu` map back for `engine`. Returns an owned buffer, or `NULL`.
///
/// Free with `rm2k_buffer_free`, passing the length written to `out_len`. `preserve_header` writes
/// the magic string back as it was read.
///
/// # Safety
///
/// `handle` must be live. `out_len` must be writable or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn rm2k_map_save(
    handle: *const Rm2kMap,
    engine: Rm2kEngine,
    preserve_header: bool,
    out_len: *mut usize,
) -> *mut u8 {
    unsafe { save_handle(handle, engine, preserve_header, out_len, file::save_map) }
}

/// Releases a `Rm2kMap` handle. `NULL` is a no-op.
///
/// # Safety
///
/// `handle` must have come from `rm2k_map_load` and not been freed already.
#[no_mangle]
pub unsafe extern "C" fn rm2k_map_free(handle: *mut Rm2kMap) {
    unsafe { free_handle(handle) }
}

/// The file's magic string as it was read; borrows from the handle.
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_map_header(handle: *const Rm2kMap) -> Rm2kBytes {
    unsafe { handle_header(handle) }
}

/// Loads a `RPG_RT.lmt` map tree. Returns `NULL` on failure, with the reason in `out_error` if
/// non-`NULL`.
///
/// Free with `rm2k_tree_map_free`. `diagnostics` may be `NULL` to discard recovered problems.
///
/// # Safety
///
/// `data` must be `NULL` or point to at least `len` readable bytes. `diagnostics` must be `NULL` or
/// a live collector from `rm2k_diagnostics_new`. `out_error`, if non-null, must be writable.
#[no_mangle]
pub unsafe extern "C" fn rm2k_tree_map_load(
    data: *const u8,
    len: usize,
    diagnostics: *mut Rm2kDiagnostics,
    out_error: *mut Rm2kError,
) -> *mut Rm2kTreeMap {
    unsafe { load_handle(data, len, diagnostics, out_error, load_tree_map) }
}

/// Writes the loaded `RPG_RT.lmt` map tree back for `engine`. Returns an owned buffer, or `NULL`.
///
/// Free with `rm2k_buffer_free`, passing the length written to `out_len`. `preserve_header` writes
/// the magic string back as it was read.
///
/// # Safety
///
/// `handle` must be live. `out_len` must be writable or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn rm2k_tree_map_save(
    handle: *const Rm2kTreeMap,
    engine: Rm2kEngine,
    preserve_header: bool,
    out_len: *mut usize,
) -> *mut u8 {
    unsafe { save_handle(handle, engine, preserve_header, out_len, file::save_tree_map) }
}

/// Releases a `Rm2kTreeMap` handle. `NULL` is a no-op.
///
/// # Safety
///
/// `handle` must have come from `rm2k_tree_map_load` and not been freed already.
#[no_mangle]
pub unsafe extern "C" fn rm2k_tree_map_free(handle: *mut Rm2kTreeMap) {
    unsafe { free_handle(handle) }
}

/// The file's magic string as it was read; borrows from the handle.
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_tree_map_header(handle: *const Rm2kTreeMap) -> Rm2kBytes {
    unsafe { handle_header(handle) }
}

/// Loads a `.lsd` save file. Returns `NULL` on failure, with the reason in `out_error` if
/// non-`NULL`.
///
/// Free with `rm2k_save_free`. `diagnostics` may be `NULL` to discard recovered problems.
///
/// # Safety
///
/// `data` must be `NULL` or point to at least `len` readable bytes. `diagnostics` must be `NULL` or
/// a live collector from `rm2k_diagnostics_new`. `out_error`, if non-null, must be writable.
#[no_mangle]
pub unsafe extern "C" fn rm2k_save_load(
    data: *const u8,
    len: usize,
    diagnostics: *mut Rm2kDiagnostics,
    out_error: *mut Rm2kError,
) -> *mut Rm2kSave {
    unsafe { load_handle(data, len, diagnostics, out_error, load_save) }
}

/// Writes the loaded `.lsd` save file back for `engine`. Returns an owned buffer, or `NULL`.
///
/// Free with `rm2k_buffer_free`, passing the length written to `out_len`. `preserve_header` writes
/// the magic string back as it was read.
///
/// # Safety
///
/// `handle` must be live. `out_len` must be writable or `NULL`.
#[no_mangle]
pub unsafe extern "C" fn rm2k_save_save(
    handle: *const Rm2kSave,
    engine: Rm2kEngine,
    preserve_header: bool,
    out_len: *mut usize,
) -> *mut u8 {
    unsafe { save_handle(handle, engine, preserve_header, out_len, file::save_save) }
}

/// Releases a `Rm2kSave` handle. `NULL` is a no-op.
///
/// # Safety
///
/// `handle` must have come from `rm2k_save_load` and not been freed already.
#[no_mangle]
pub unsafe extern "C" fn rm2k_save_free(handle: *mut Rm2kSave) {
    unsafe { free_handle(handle) }
}

/// The file's magic string as it was read; borrows from the handle.
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_save_header(handle: *const Rm2kSave) -> Rm2kBytes {
    unsafe { handle_header(handle) }
}

/// Which engine the database says it targets (`system.ldb_id`).
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_engine(handle: *const Rm2kDatabase) -> Rm2kEngine {
    unsafe { handle.as_ref() }.map_or(Rm2kEngine::R2K, |handle| {
        Engine::from_ldb_id(handle.value.system.ldb_id).into()
    })
}

/// Classifies a raw `system.ldb_id` value: `2003` is 2003, anything else is 2000.
#[no_mangle]
pub extern "C" fn rm2k_engine_from_ldb_id(ldb_id: i32) -> Rm2kEngine {
    Engine::from_ldb_id(ldb_id).into()
}

/// A database entry's id and name; the name borrows from the handle.
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

/// The number of entries of one of the database's named lists.
///
/// A typed accessor per field would be ~1200 functions; two per list (`rm2k_database_list_len` and
/// `rm2k_database_list_get`) cover what tools ask for. The zero-copy walkers reach the rest.
///
/// # Safety
///
/// `handle` must be live.
#[no_mangle]
pub unsafe extern "C" fn rm2k_database_list_len(handle: *const Rm2kDatabase, list: Rm2kDbList) -> usize {
    // A closure, not `Iterator::count`: `with_list`'s higher-ranked bound needs a callable generic
    // over the iterator's lifetime; a bare method path is not.
    unsafe { handle.as_ref() }.map_or(0, |handle| with_list(&handle.value, list, |entries| entries.count()))
}

/// Writes the entry at `index` of `list` to `out` and returns `true`, or returns `false` when
/// `index` is out of range.
///
/// The returned name borrows from the handle and is valid until it is freed.
///
/// # Safety
///
/// `handle` must be live; `out` must be writable or `NULL`.
#[no_mangle]
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

// The zero-copy walkers are C-visible inline structs holding a Rust walker in caller memory:
// initialize one with its `_init` function and pass it by pointer. It borrows the buffer it was
// initialised from, which must outlive it.
//
// Boxing each walker would put an allocation on the one path meant to have none. The storage is
// `u64` words, not bytes, so it is 8-aligned without `repr(align)` (which cbindgen turns into an
// opaque type). The size is asserted at compile time: growing the Rust type is a build error, not
// memory corruption at the call site.

/// Iterator over the chunks of a chunk stream; see `rm2k_chunk_iter_init`.
///
/// Opaque: initialize with `rm2k_chunk_iter_init` and pass by pointer. Borrows the buffer it was
/// initialised from.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rm2kChunkIter {
    _opaque: [u64; 8],
}

/// Walker over the elements of an indexed list; see `rm2k_list_walker_init`.
///
/// Opaque: initialize with `rm2k_list_walker_init` and pass by pointer. Borrows the buffer it was
/// initialised from.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rm2kListWalker {
    _opaque: [u64; 12],
}

/// Walker over the event command lists of a file; see `rm2k_command_lists_init`.
///
/// Opaque: initialize with `rm2k_command_lists_init` and pass by pointer. Borrows the buffer it was
/// initialised from.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rm2kCommandLists {
    _opaque: [u64; 32],
}

/// Iterator over the commands of an event command list; see `rm2k_event_commands_init`.
///
/// Opaque: initialize with `rm2k_event_commands_init` and pass by pointer. Borrows the buffer it
/// was initialised from.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Rm2kEventCommands {
    _opaque: [u64; 8],
}

macro_rules! opaque_inline {
    ($name:ident, $inner:ty) => {
        const _: () = {
            assert!(
                size_of::<$inner>() <= size_of::<$name>(),
                "opaque buffer too small; raise it"
            );
            assert!(align_of::<$inner>() <= align_of::<$name>());
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

opaque_inline!(Rm2kChunkIter, ChunkIter<'static>);
opaque_inline!(Rm2kListWalker, ListWalker<'static>);
opaque_inline!(Rm2kCommandLists, CommandLists<'static>);
opaque_inline!(Rm2kEventCommands, EventCommandIter<'static>);

// SAFETY: `data` must point to `len` readable bytes that outlive every walker built from them.
// The one contract the zero-copy API cannot check; stated on every entry point.
unsafe fn slice<'a>(data: *const u8, len: usize) -> Option<&'a [u8]> {
    if data.is_null() {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(data, len) })
}

/// The zero-copy walkers take a *body*, not a whole file, so this is normally the first call.
///
/// # Safety
///
/// `data` must point to `len` readable bytes; `out_body` and `out_header` must be writable
/// or `NULL`.
#[no_mangle]
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

/// Initializes `iter` over the chunk stream `data`. `base` is the file-absolute offset of
/// `data[0]`, reported in every chunk's `offset`.
///
/// # Safety
///
/// `iter` must be writable; `data` must point to `len` readable bytes that outlive `iter`.
#[no_mangle]
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

/// Writes the next chunk to `out` and returns `true`, or returns `false` once the stream is
/// exhausted.
///
/// # Safety
///
/// `iter` must have been initialised and its buffer still alive; `out` must be writable or
/// `NULL`.
#[no_mangle]
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

/// Finds one chunk by id. O(chunks): a chunk stream has no index.
///
/// # Safety
///
/// `data` must point to `len` readable bytes; `out` must be writable or `NULL`.
#[no_mangle]
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

/// Initializes `walker` over a list of indexed elements in the chunk stream `data`. `base` is the
/// file-absolute offset of `data[0]`.
///
/// # Safety
///
/// `walker` must be writable; `data` must point to `len` readable bytes that outlive
/// `walker`.
#[no_mangle]
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

/// Writes the next element to `out` and returns `true`, or returns `false` once the list is
/// exhausted.
///
/// # Safety
///
/// `walker` must have been initialised and its buffer still alive; `out` must be writable
/// or `NULL`.
#[no_mangle]
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

/// Where `rm2k_command_lists_init` looks for event command lists.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rm2kEventScope {
    /// From an LMU body.
    MapEvents = 0,
    /// From an LDB body.
    CommonEvents = 1,
    /// From an LDB body.
    BattleEvents = 2,
}

/// Finds every event command list under a file root, in the part of the game `scope` names.
///
/// `data` is a *body* - the bytes after the file's magic, as produced by `rm2k_split_header`.
///
/// # Safety
///
/// `lists` must be writable; `data` must point to `len` readable bytes that outlive
/// `lists`.
#[no_mangle]
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

/// Writes the next event command list to `out` and returns `true`, or returns `false` once there
/// are no more.
///
/// # Safety
///
/// `lists` must have been initialised and its buffer still alive; `out` must be writable
/// or `NULL`.
#[no_mangle]
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

/// Initializes `iter` over the commands of one event command list, whose bytes are
/// `Rm2kCommandList::commands`.
///
/// # Safety
///
/// `iter` must be writable; `data` must point to `len` readable bytes that outlive `iter`.
#[no_mangle]
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

/// Writes the next event command to `out` and returns `true`, or returns `false` once the list is
/// exhausted.
///
/// # Safety
///
/// `iter` must have been initialised and its buffer still alive; `out` must be writable or
/// `NULL`.
#[no_mangle]
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
            raw_params: Rm2kBytes::new(command.raw_params()),
        };
    }
    true
}

/// O(index): compressed integers are not fixed width, so there is no way to index directly.
///
/// # Safety
///
/// `command` must point to a command filled in by `rm2k_event_commands_next`, whose buffer
/// is still alive; `out` must be writable or `NULL`.
// LCF's integers are 32-bit signed but stored as unsigned varints; the bits are reinterpreted, as liblcf does.
#[allow(clippy::cast_possible_wrap)]
#[no_mangle]
pub unsafe extern "C" fn rm2k_event_command_param(
    command: *const Rm2kEventCommand,
    index: usize,
    out: *mut i32,
) -> bool {
    let Some(command) = (unsafe { command.as_ref() }) else {
        return false;
    };
    let Some(bytes) = (unsafe { slice(command.raw_params.data, command.raw_params.len) }) else {
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

// The same eight ids as `rm2k::event::path`, so C can navigate by hand, not only through
// `rm2k_command_lists_init`. Deliberately not the full set: liblcf generates ~1200 chunk ids across
// four headers, and nothing here needs the rest.
//
// Literals, not `= path::X`: cbindgen emits only literal consts as `#define`s, which (unlike an
// exported `extern const`) work in a `case` label and need no `dllimport` from a Windows DLL. The
// assertions below keep them equal to the real ids.
pub mod chunk_id {
    use rm2k::event::path;

    /// Chunk id of a map's event list (`rm2k::event::path::MAP_EVENTS`).
    pub const RM2K_CHUNK_MAP_EVENTS: u32 = 0x51;
    /// Chunk id of an event's pages (`rm2k::event::path::EVENT_PAGES`).
    pub const RM2K_CHUNK_EVENT_PAGES: u32 = 0x05;
    /// Chunk id of an event page's commands (`rm2k::event::path::EVENT_PAGE_COMMANDS`).
    pub const RM2K_CHUNK_EVENT_PAGE_COMMANDS: u32 = 0x34;
    /// Chunk id of the database's common events (`rm2k::event::path::DATABASE_COMMON_EVENTS`).
    pub const RM2K_CHUNK_DATABASE_COMMON_EVENTS: u32 = 0x19;
    /// Chunk id of a common event's commands (`rm2k::event::path::COMMON_EVENT_COMMANDS`).
    pub const RM2K_CHUNK_COMMON_EVENT_COMMANDS: u32 = 0x16;
    /// Chunk id of the database's troops (`rm2k::event::path::DATABASE_TROOPS`).
    pub const RM2K_CHUNK_DATABASE_TROOPS: u32 = 0x0F;
    /// Chunk id of a troop's pages (`rm2k::event::path::TROOP_PAGES`).
    pub const RM2K_CHUNK_TROOP_PAGES: u32 = 0x0B;
    /// Chunk id of a troop page's commands (`rm2k::event::path::TROOP_PAGE_COMMANDS`).
    pub const RM2K_CHUNK_TROOP_PAGE_COMMANDS: u32 = 0x0C;

    const _: () = {
        assert!(RM2K_CHUNK_MAP_EVENTS == path::MAP_EVENTS);
        assert!(RM2K_CHUNK_EVENT_PAGES == path::EVENT_PAGES);
        assert!(RM2K_CHUNK_EVENT_PAGE_COMMANDS == path::EVENT_PAGE_COMMANDS);
        assert!(RM2K_CHUNK_DATABASE_COMMON_EVENTS == path::DATABASE_COMMON_EVENTS);
        assert!(RM2K_CHUNK_COMMON_EVENT_COMMANDS == path::COMMON_EVENT_COMMANDS);
        assert!(RM2K_CHUNK_DATABASE_TROOPS == path::DATABASE_TROOPS);
        assert!(RM2K_CHUNK_TROOP_PAGES == path::TROOP_PAGES);
        assert!(RM2K_CHUNK_TROOP_PAGE_COMMANDS == path::TROOP_PAGE_COMMANDS);
    };
}

#[cfg(test)]
mod tests;
