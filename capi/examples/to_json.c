#include "rm2k.h"
#include "yyjson/yyjson.h"

#include <stdio.h>
#include <stdlib.h>

// This example is built against a genuinely no_std rm2k-capi (see the build steps below), so the
// library has no allocator of its own - it must be given one via rm2k_set_allocator before any
// other rm2k_* call that allocates. Plain malloc/free happen to be what's available here, but
// nothing about the hook shape requires that; a real freestanding target would back this with its
// own heap.
static uint8_t* host_alloc(size_t size, size_t align) {
    if (size == 0) {
        size = 1;
    }
    // Plain malloc only guarantees alignof(max_align_t) - 16 bytes on every platform this example
    // targets - so anything within that is a plain malloc; anything stricter goes through the
    // platform's aligned allocator (MSVC's UCRT has no standard aligned_alloc).
    if (align <= 16) {
        return (uint8_t*)malloc(size);
    }
#if defined(_WIN32)
    return (uint8_t*)_aligned_malloc(size, align);
#else
    size_t rounded = (size + align - 1) & ~(align - 1);
    return (uint8_t*)aligned_alloc(align, rounded);
#endif
}

static void host_free(uint8_t* ptr, size_t size, size_t align) {
    (void)size;
    if (align <= 16) {
        free(ptr);
        return;
    }
#if defined(_WIN32)
    _aligned_free(ptr);
#else
    free(ptr);
#endif
}

static uint8_t* read_file(const char* path, size_t* out_len) {
    FILE* f = fopen(path, "rb");
    if (!f)
        return NULL;
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    if (size < 0) {
        fclose(f);
        return NULL;
    }
    uint8_t* buf = malloc((size_t)size ? (size_t)size : 1);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)size, f);
    fclose(f);
    *out_len = got;
    return buf;
}

/* Latin-1 -> UTF-8, one code point per input byte. Caller frees the result with
 * free(). */
static char* latin1_to_utf8(const uint8_t* data, size_t len, size_t* out_len) {
    char* buf = malloc(len * 2 + 1);
    size_t w = 0;
    for (size_t i = 0; i < len; i++) {
        uint8_t b = data[i];
        if (b < 0x80) {
            buf[w++] = (char)b;
        } else {
            buf[w++] = (char)(0xC0 | (b >> 6));
            buf[w++] = (char)(0x80 | (b & 0x3F));
        }
    }
    buf[w] = '\0';
    *out_len = w;
    return buf;
}

/* Adds `key`: <bytes, as Latin-1-mapped UTF-8> to `obj`, copying the encoded
 * text into `doc`. */
static void add_bytes(yyjson_mut_doc* doc, yyjson_mut_val* obj, const char* key, Rm2kBytes bytes) {
    size_t utf8_len = 0;
    char* utf8 = latin1_to_utf8(bytes.data, bytes.len, &utf8_len);
    yyjson_mut_obj_add_strncpy(doc, obj, key, utf8, utf8_len);
    free(utf8);
}

static const char* const LIST_NAMES[] = {
    "actors",  "skills",     "items",    "enemies",  "troops",     "states",
    "classes", "animations", "chipsets", "terrains", "attributes", "commonEvents",
};
static const Rm2kDbList LIST_IDS[] = {
    RM2K_DB_LIST_ACTORS,   RM2K_DB_LIST_SKILLS,   RM2K_DB_LIST_ITEMS,      RM2K_DB_LIST_ENEMIES,
    RM2K_DB_LIST_TROOPS,   RM2K_DB_LIST_STATES,   RM2K_DB_LIST_CLASSES,    RM2K_DB_LIST_ANIMATIONS,
    RM2K_DB_LIST_CHIPSETS, RM2K_DB_LIST_TERRAINS, RM2K_DB_LIST_ATTRIBUTES, RM2K_DB_LIST_COMMON_EVENTS,
};
#define LIST_COUNT (sizeof(LIST_IDS) / sizeof(LIST_IDS[0]))

static yyjson_mut_doc* build_database_json(const Rm2kDatabase* db) {
    yyjson_mut_doc* doc = yyjson_mut_doc_new(NULL);
    yyjson_mut_val* root = yyjson_mut_obj(doc);
    yyjson_mut_doc_set_root(doc, root);

    yyjson_mut_obj_add_str(doc, root, "engine", rm2k_database_engine(db) == RM2K_ENGINE_R2K3 ? "2k3" : "2k");
    add_bytes(doc, root, "header", rm2k_database_header(db));

    yyjson_mut_val* lists = yyjson_mut_obj(doc);
    yyjson_mut_obj_add_val(doc, root, "lists", lists);

    for (size_t l = 0; l < LIST_COUNT; l++) {
        yyjson_mut_val* arr = yyjson_mut_arr(doc);
        yyjson_mut_obj_add_val(doc, lists, LIST_NAMES[l], arr);

        size_t count = rm2k_database_list_len(db, LIST_IDS[l]);
        for (size_t i = 0; i < count; i++) {
            Rm2kNamed entry;
            if (!rm2k_database_list_get(db, LIST_IDS[l], i, &entry))
                break;

            yyjson_mut_val* item = yyjson_mut_arr_add_obj(doc, arr);
            yyjson_mut_obj_add_int(doc, item, "id", entry.id);
            add_bytes(doc, item, "name", entry.name);
        }
    }

    return doc;
}

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: to_json <RPG_RT.ldb> [out.json]\n");
        return 2;
    }

    rm2k_set_allocator(host_alloc, host_free);

    size_t len = 0;
    uint8_t* data = read_file(argv[1], &len);
    if (!data) {
        fprintf(stderr, "could not read %s\n", argv[1]);
        return 1;
    }

    Rm2kDiagnostics* diagnostics = rm2k_diagnostics_new();
    Rm2kError error = RM2K_ERROR_OK;
    Rm2kDatabase* db = rm2k_database_load(data, len, diagnostics, &error);
    free(data);

    if (!db) {
        fprintf(stderr, "load failed: error %d\n", (int)error);
        rm2k_diagnostics_free(diagnostics);
        return 1;
    }

    size_t diag_count = rm2k_diagnostics_count(diagnostics);
    if (diag_count > 0) {
        fprintf(stderr, "%zu diagnostic(s) while loading:\n", diag_count);
        for (size_t i = 0; i < diag_count; i++) {
            Rm2kDiagnostic d;
            if (rm2k_diagnostics_get(diagnostics, i, &d)) {
                fprintf(
                    stderr,
                    "  chunk 0x%x @ offset %u: %s\n",
                    d.chunk_id,
                    d.offset,
                    rm2k_diagnostic_kind_name(d.kind)
                );
            }
        }
    }

    yyjson_mut_doc* doc = build_database_json(db);

    FILE* out = stdout;
    if (argc >= 3) {
        out = fopen(argv[2], "wb");
        if (!out) {
            fprintf(stderr, "could not open %s for writing\n", argv[2]);
            yyjson_mut_doc_free(doc);
            rm2k_database_free(db);
            rm2k_diagnostics_free(diagnostics);
            return 1;
        }
    }

    yyjson_write_err write_err;
    if (!yyjson_mut_write_fp(out, doc, YYJSON_WRITE_PRETTY, NULL, &write_err)) {
        fprintf(stderr, "JSON write failed: %s\n", write_err.msg);
    }

    if (out != stdout)
        fclose(out);
    yyjson_mut_doc_free(doc);
    rm2k_database_free(db);
    rm2k_diagnostics_free(diagnostics);
    return 0;
}
