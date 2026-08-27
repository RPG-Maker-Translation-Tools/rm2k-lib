#ifndef RM2K_H
#define RM2K_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Mirrors Rm2kError (src/lib.rs) exactly, in declaration order. */
typedef enum Rm2kError {
    RM2K_OK = 0,
    RM2K_ERR_UNEXPECTED_EOF = 1,
    RM2K_ERR_BAD_MAGIC = 2,
    RM2K_ERR_VARINT_OVERFLOW = 3,
    RM2K_ERR_CHUNK_OVERRUN = 4,
    RM2K_ERR_STRICT = 5,
    RM2K_ERR_NULL_ARGUMENT = 6
} Rm2kError;

/* Mirrors Rm2kEngine (src/lib.rs) exactly, in declaration order. */
typedef enum Rm2kEngine {
    RM2K_ENGINE_2K = 0,
    RM2K_ENGINE_2K3 = 1
} Rm2kEngine;

/* Mirrors Rm2kDbList (src/lib.rs) exactly, in declaration order. */
typedef enum Rm2kDbList {
    RM2K_LIST_ACTORS = 0,
    RM2K_LIST_SKILLS = 1,
    RM2K_LIST_ITEMS = 2,
    RM2K_LIST_ENEMIES = 3,
    RM2K_LIST_TROOPS = 4,
    RM2K_LIST_STATES = 5,
    RM2K_LIST_CLASSES = 6,
    RM2K_LIST_ANIMATIONS = 7,
    RM2K_LIST_CHIPSETS = 8,
    RM2K_LIST_TERRAINS = 9,
    RM2K_LIST_ATTRIBUTES = 10,
    RM2K_LIST_COMMON_EVENTS = 11
} Rm2kDbList;

/* Mirrors Rm2kEventScope (src/lib.rs) exactly, in declaration order. */
typedef enum Rm2kEventScope {
    RM2K_SCOPE_MAP_EVENTS = 0,
    RM2K_SCOPE_COMMON_EVENTS = 1,
    RM2K_SCOPE_BATTLE_EVENTS = 2
} Rm2kEventScope;

/* Diagnostic kinds, for rm2k_diagnostic_kind_name(). Mirrors rm2k::DiagKind. */
#define RM2K_DIAG_UNKNOWN_CHUNK 0
#define RM2K_DIAG_SHORT_CHUNK 1
#define RM2K_DIAG_TRAILING_BYTES 2
#define RM2K_DIAG_BAD_VARINT 3
#define RM2K_DIAG_CHUNK_OVERRUN 4
#define RM2K_DIAG_BAD_MAGIC 5
#define RM2K_DIAG_EVENT_LIST_CORRUPT 6
#define RM2K_DIAG_RAGGED_ARRAY 7

/* A borrowed run of bytes. `data` is NULL when `len` is 0. */
typedef struct Rm2kBytes {
    const uint8_t* data;
    size_t len;
} Rm2kBytes;

typedef struct Rm2kChunk {
    uint32_t id; /* the chunk's field id */
    uint32_t offset; /* file-absolute offset of the payload */
    Rm2kBytes data;
} Rm2kChunk;

typedef struct Rm2kListItem {
    int32_t id;
    uint32_t offset;
    Rm2kBytes body; /* the element's chunk stream */
} Rm2kListItem;

typedef struct Rm2kCommandList {
    int32_t owner_id; /* the owning event, common event or troop */
    int32_t page_id; /* the owning page, or 0 for a common event, which has none */
    uint32_t offset;
    Rm2kBytes commands; /* feed to rm2k_event_commands_init() */
} Rm2kCommandList;

typedef struct Rm2kEventCommand {
    int32_t code;
    int32_t indent;
    uint32_t param_count;
    Rm2kBytes string; /* verbatim; see the note on strings above */
    Rm2kBytes _params; /* private; read via rm2k_event_command_param() */
} Rm2kEventCommand;

typedef struct Rm2kDiagnostic {
    uint32_t offset;
    uint32_t chunk_id;
    uint8_t kind; /* one of the RM2K_DIAG_* values */
} Rm2kDiagnostic;

typedef struct Rm2kNamed {
    int32_t id;
    Rm2kBytes name;
} Rm2kNamed;

typedef struct Rm2kDiagnostics Rm2kDiagnostics;
typedef struct Rm2kDatabase Rm2kDatabase;
typedef struct Rm2kMap Rm2kMap;
typedef struct Rm2kTreeMap Rm2kTreeMap;
typedef struct Rm2kSave Rm2kSave;

/*
 * The zero-copy walkers. Treat the contents as opaque: initialize with the
 * matching _init() function and pass by pointer. Sizes are asserted against the
 * Rust types at compile time, so a mismatch is a build error there rather than
 * corruption here.
 */
typedef struct Rm2kChunkIter {
    _Alignas(8) uint8_t _opaque[64];
} Rm2kChunkIter;

typedef struct Rm2kListWalker {
    _Alignas(8) uint8_t _opaque[96];
} Rm2kListWalker;

typedef struct Rm2kCommandLists {
    _Alignas(8) uint8_t _opaque[256];
} Rm2kCommandLists;

typedef struct Rm2kEventCommands {
    _Alignas(8) uint8_t _opaque[64];
} Rm2kEventCommands;

/*
 * Optional everywhere. Pass NULL to any function taking an Rm2kDiagnostics* to
 * discard recovered problems - which is also the only fully allocation-free
 * option.
 */
Rm2kDiagnostics* rm2k_diagnostics_new(void);
void rm2k_diagnostics_free(Rm2kDiagnostics* diagnostics);
size_t rm2k_diagnostics_count(const Rm2kDiagnostics* diagnostics);
bool rm2k_diagnostics_get(const Rm2kDiagnostics* diagnostics, size_t index, Rm2kDiagnostic* out);
void rm2k_diagnostics_clear(Rm2kDiagnostics* diagnostics);

/* A fixed English label for a diagnostic kind. NUL-terminated, static, never
 * freed. */
const char* rm2k_diagnostic_kind_name(uint8_t kind);

/* Host allocation hooks, required only when this library was built without its default `std`
 * feature (a genuinely no_std/freestanding build). `alloc` must return a pointer to at least
 * `size` bytes aligned to `align` (a power of two), or NULL on failure; `free` gets back exactly
 * the (ptr, size, align) triple a prior `alloc` call produced. Neither is assumed to be
 * malloc/free - back them with whatever allocator the target actually has. */
typedef uint8_t* (*Rm2kAllocFn)(size_t size, size_t align);
typedef void (*Rm2kFreeFn)(uint8_t* ptr, size_t size, size_t align);

/* Registers the host's allocator. Must be called exactly once, before any other rm2k_* function
 * that allocates, in a no_std build of this library; has no effect (and need not be called) in
 * the default std-linked build, which uses Rust's own allocator. */
void rm2k_set_allocator(Rm2kAllocFn alloc, Rm2kFreeFn free);

/* Release a buffer from rm2k_*_save(). The length must be the one that call
 * reported. */
void rm2k_buffer_free(uint8_t* ptr, size_t len);

/*
 * load: returns NULL on failure and writes the reason through out_error when
 * non-NULL. save: returns a buffer you own, or NULL. Release with
 * rm2k_buffer_free(). header: the file's magic string as it was read; borrows
 * from the handle.
 */

Rm2kDatabase* rm2k_database_load(const uint8_t* data, size_t len, Rm2kDiagnostics* diagnostics, Rm2kError* out_error);
uint8_t* rm2k_database_save(const Rm2kDatabase* handle, Rm2kEngine engine, bool preserve_header, size_t* out_len);
void rm2k_database_free(Rm2kDatabase* handle);
Rm2kBytes rm2k_database_header(const Rm2kDatabase* handle);

Rm2kMap* rm2k_map_load(const uint8_t* data, size_t len, Rm2kDiagnostics* diagnostics, Rm2kError* out_error);
uint8_t* rm2k_map_save(const Rm2kMap* handle, Rm2kEngine engine, bool preserve_header, size_t* out_len);
void rm2k_map_free(Rm2kMap* handle);
Rm2kBytes rm2k_map_header(const Rm2kMap* handle);

Rm2kTreeMap* rm2k_tree_map_load(const uint8_t* data, size_t len, Rm2kDiagnostics* diagnostics, Rm2kError* out_error);
uint8_t* rm2k_tree_map_save(const Rm2kTreeMap* handle, Rm2kEngine engine, bool preserve_header, size_t* out_len);
void rm2k_tree_map_free(Rm2kTreeMap* handle);
Rm2kBytes rm2k_tree_map_header(const Rm2kTreeMap* handle);

Rm2kSave* rm2k_save_load(const uint8_t* data, size_t len, Rm2kDiagnostics* diagnostics, Rm2kError* out_error);
uint8_t* rm2k_save_save(const Rm2kSave* handle, Rm2kEngine engine, bool preserve_header, size_t* out_len);
void rm2k_save_free(Rm2kSave* handle);
Rm2kBytes rm2k_save_header(const Rm2kSave* handle);

/* Which engine the database says it targets (system.ldb_id). */
Rm2kEngine rm2k_database_engine(const Rm2kDatabase* handle);

/*
 * Enumerate one of the database's named lists. A typed accessor per FIELD would
 * be ~1200 functions; per LIST it is two, and covers what tools actually ask
 * for. Everything else is reachable through the zero-copy walkers below.
 *
 * The name in Rm2kNamed borrows from the handle.
 */
size_t rm2k_database_list_len(const Rm2kDatabase* handle, Rm2kDbList list);
bool rm2k_database_list_get(const Rm2kDatabase* handle, Rm2kDbList list, size_t index, Rm2kNamed* out);

/*
 * Split a file into its magic and its root chunk stream. The walkers below
 * operate on the BODY, not on a whole file, so this is normally your first
 * call.
 */
bool rm2k_split_header(const uint8_t* data, size_t len, Rm2kBytes* out_header, Rm2kBytes* out_body);

bool rm2k_chunk_iter_init(Rm2kChunkIter* iter, const uint8_t* data, size_t len, uint32_t base);
bool rm2k_chunk_iter_next(Rm2kChunkIter* iter, Rm2kDiagnostics* diagnostics, Rm2kChunk* out);

/* Find one chunk by id. O(chunks): a chunk stream has no index. */
bool rm2k_chunk_find(
    const uint8_t* data,
    size_t len,
    uint32_t base,
    uint32_t id,
    Rm2kDiagnostics* diagnostics,
    Rm2kChunk* out
);

bool rm2k_list_walker_init(
    Rm2kListWalker* walker,
    const uint8_t* data,
    size_t len,
    uint32_t base,
    Rm2kDiagnostics* diagnostics
);
bool rm2k_list_walker_next(Rm2kListWalker* walker, Rm2kDiagnostics* diagnostics, Rm2kListItem* out);

/* Find every event command list under a file root. `data` is a BODY; see
 * rm2k_split_header(). */
bool rm2k_command_lists_init(
    Rm2kCommandLists* lists,
    Rm2kEventScope scope,
    const uint8_t* data,
    size_t len,
    uint32_t base,
    Rm2kDiagnostics* diagnostics
);
bool rm2k_command_lists_next(Rm2kCommandLists* lists, Rm2kDiagnostics* diagnostics, Rm2kCommandList* out);

bool rm2k_event_commands_init(Rm2kEventCommands* iter, const uint8_t* data, size_t len, uint32_t base);
bool rm2k_event_commands_next(Rm2kEventCommands* iter, Rm2kDiagnostics* diagnostics, Rm2kEventCommand* out);

/* O(index): compressed integers are not fixed width, so there is no direct
 * indexing. */
bool rm2k_event_command_param(const Rm2kEventCommand* command, size_t index, int32_t* out);

/*
 * The same eight ids as rm2k::event::path, for navigating by hand rather than
 * through rm2k_command_lists_init(). Deliberately not the full set: liblcf
 * generates ~1200 chunk ids across four headers, and nothing here needs the
 * rest.
 */
extern const uint32_t RM2K_CHUNK_MAP_EVENTS;
extern const uint32_t RM2K_CHUNK_EVENT_PAGES;
extern const uint32_t RM2K_CHUNK_EVENT_PAGE_COMMANDS;
extern const uint32_t RM2K_CHUNK_DATABASE_COMMON_EVENTS;
extern const uint32_t RM2K_CHUNK_COMMON_EVENT_COMMANDS;
extern const uint32_t RM2K_CHUNK_DATABASE_TROOPS;
extern const uint32_t RM2K_CHUNK_TROOP_PAGES;
extern const uint32_t RM2K_CHUNK_TROOP_PAGE_COMMANDS;

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RM2K_H */
