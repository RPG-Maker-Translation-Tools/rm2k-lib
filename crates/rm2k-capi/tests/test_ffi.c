#include "../assets/rm2k.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;
static int checks = 0;

#define CHECK(cond) \
    do { \
        checks++; \
        if (!(cond)) { \
            printf("FAIL %s:%d  %s\n", __FILE__, __LINE__, #cond); \
            failures++; \
        } \
    } while (0)

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

/* --------------------------------------------------------------------------------------------
 * Tests that need no game files
 * --------------------------------------------------------------------------------------------
 */

static void test_null_arguments_are_rejected(void) {
    Rm2kError err = RM2K_OK;
    CHECK(rm2k_database_load(NULL, 0, NULL, &err) == NULL);
    CHECK(err == RM2K_ERR_NULL_ARGUMENT);

    /* Every free must tolerate NULL. */
    rm2k_database_free(NULL);
    rm2k_map_free(NULL);
    rm2k_tree_map_free(NULL);
    rm2k_save_free(NULL);
    rm2k_diagnostics_free(NULL);
    rm2k_buffer_free(NULL, 0);

    CHECK(rm2k_diagnostics_count(NULL) == 0);
    CHECK(!rm2k_diagnostics_get(NULL, 0, NULL));
}

static void test_truncated_input_is_an_error_not_a_crash(void) {
    const uint8_t empty[1] = { 0 };
    Rm2kError err = RM2K_OK;
    CHECK(rm2k_database_load(empty, 0, NULL, &err) == NULL);
    CHECK(err == RM2K_ERR_UNEXPECTED_EOF);
}

static void test_diagnostic_names(void) {
    CHECK(strcmp(rm2k_diagnostic_kind_name(RM2K_DIAG_UNKNOWN_CHUNK), "unknown chunk") == 0);
    CHECK(strcmp(rm2k_diagnostic_kind_name(RM2K_DIAG_BAD_MAGIC), "unexpected file magic") == 0);
    /* Out of range must clamp rather than read past the table. */
    CHECK(rm2k_diagnostic_kind_name(200) != NULL);
}

static void test_chunk_ids_are_exported(void) {
    CHECK(RM2K_CHUNK_MAP_EVENTS == 0x51);
    CHECK(RM2K_CHUNK_EVENT_PAGES == 0x05);
    CHECK(RM2K_CHUNK_EVENT_PAGE_COMMANDS == 0x34);
    CHECK(RM2K_CHUNK_DATABASE_TROOPS == 0x0F);
}

/* --------------------------------------------------------------------------------------------
 * Tests against a real game
 * --------------------------------------------------------------------------------------------
 */

static void test_database_round_trip(const char* path) {
    size_t len = 0;
    uint8_t* bytes = read_file(path, &len);
    if (!bytes) {
        printf("skip: cannot read %s\n", path);
        return;
    }

    Rm2kDiagnostics* diag = rm2k_diagnostics_new();
    Rm2kError err = RM2K_OK;
    Rm2kDatabase* db = rm2k_database_load(bytes, len, diag, &err);
    CHECK(db != NULL);
    CHECK(err == RM2K_OK);
    if (!db) {
        free(bytes);
        rm2k_diagnostics_free(diag);
        return;
    }

    printf("  %zu diagnostics\n", rm2k_diagnostics_count(diag));

    Rm2kBytes header = rm2k_database_header(db);
    CHECK(header.len == 11);
    CHECK(memcmp(header.data, "LcfDataBase", 11) == 0);

    /* Actor names come back as raw bytes. */
    size_t actors = rm2k_database_list_len(db, RM2K_LIST_ACTORS);
    printf("  %zu actors, engine %s\n", actors, rm2k_database_engine(db) == RM2K_ENGINE_2K3 ? "2003" : "2000");
    CHECK(actors > 0);

    Rm2kNamed first;
    CHECK(rm2k_database_list_get(db, RM2K_LIST_ACTORS, 0, &first));
    printf("  actor[0] id=%d name=\"%.*s\"\n", first.id, (int)first.name.len, first.name.data);
    CHECK(!rm2k_database_list_get(db, RM2K_LIST_ACTORS, actors, &first));

    /* Round-trip must be byte-exact. */
    size_t out_len = 0;
    uint8_t* out = rm2k_database_save(db, rm2k_database_engine(db), true, &out_len);
    CHECK(out != NULL);
    CHECK(out_len == len);
    CHECK(out && memcmp(out, bytes, len < out_len ? len : out_len) == 0);
    printf(
        "  round-trip %s (%zu bytes in, %zu out)\n",
        (out && out_len == len && memcmp(out, bytes, len) == 0) ? "EXACT" : "DIFFERS",
        len,
        out_len
    );
    rm2k_buffer_free(out, out_len);

    rm2k_database_free(db);
    rm2k_diagnostics_free(diag);
    free(bytes);
}

static void test_zero_copy_walk(const char* path) {
    size_t len = 0;
    uint8_t* bytes = read_file(path, &len);
    if (!bytes) {
        printf("skip: cannot read %s\n", path);
        return;
    }

    /* No handle, no allocation: the walkers work straight on our own buffer. */
    Rm2kBytes header, body;
    CHECK(rm2k_split_header(bytes, len, &header, &body));
    CHECK(header.len == 10);
    CHECK(memcmp(header.data, "LcfMapUnit", 10) == 0);

    Rm2kCommandLists lists;
    CHECK(rm2k_command_lists_init(&lists, RM2K_SCOPE_MAP_EVENTS, body.data, body.len, 0, NULL));

    size_t list_count = 0, command_count = 0, message_count = 0;
    Rm2kCommandList list;
    while (rm2k_command_lists_next(&lists, NULL, &list)) {
        list_count++;

        Rm2kEventCommands commands;
        CHECK(rm2k_event_commands_init(&commands, list.commands.data, list.commands.len, 0));

        Rm2kEventCommand command;
        while (rm2k_event_commands_next(&commands, NULL, &command)) {
            command_count++;
            if (command.code == 10110) {
                message_count++;
                if (message_count == 1) {
                    printf("  first message: \"%.*s\"\n", (int)command.string.len, command.string.data);
                }
                /* Parameters decode on demand. */
                int32_t param = -1;
                if (command.param_count > 0) {
                    CHECK(rm2k_event_command_param(&command, 0, &param));
                }
                CHECK(!rm2k_event_command_param(&command, command.param_count, &param));
            }
        }
    }
    printf("  %zu command lists, %zu commands, %zu messages\n", list_count, command_count, message_count);
    CHECK(command_count > 0);

    /* The generic chunk walker reaches the same data by id. */
    Rm2kChunk events;
    CHECK(rm2k_chunk_find(body.data, body.len, 0, RM2K_CHUNK_MAP_EVENTS, NULL, &events));

    Rm2kListWalker walker;
    CHECK(rm2k_list_walker_init(&walker, events.data.data, events.data.len, events.offset, NULL));
    size_t event_count = 0;
    Rm2kListItem item;
    while (rm2k_list_walker_next(&walker, NULL, &item))
        event_count++;
    printf("  %zu events via the generic walker\n", event_count);
    CHECK(event_count > 0);

    free(bytes);
}

int main(int argc, char** argv) {
    printf("rm2k C API tests\n\n");

    printf("standalone:\n");
    test_null_arguments_are_rejected();
    test_truncated_input_is_an_error_not_a_crash();
    test_diagnostic_names();
    test_chunk_ids_are_exported();

    if (argc > 1) {
        char path[1024];

        snprintf(path, sizeof path, "%s/RPG_RT.ldb", argv[1]);
        printf("\ndatabase (%s):\n", path);
        test_database_round_trip(path);

        /* Scan a handful of maps: the first one is often a title screen with no
         * dialogue, and the message/parameter paths are the ones worth exercising.
         */
        printf("\nzero-copy walk:\n");
        for (int i = 1; i <= 8; i++) {
            snprintf(path, sizeof path, "%s/Map%04d.lmu", argv[1], i);
            FILE* probe = fopen(path, "rb");
            if (!probe)
                continue;
            fclose(probe);
            printf("  -- Map%04d.lmu\n", i);
            test_zero_copy_walk(path);
        }
    } else {
        printf("\n(pass a game directory to run the file-backed tests)\n");
    }

    printf("\n%d checks, %d failures\n", checks, failures);
    return failures == 0 ? 0 : 1;
}
