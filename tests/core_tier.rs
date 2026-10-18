//! The core tier's headline claim, checked against the `alloc` tier.
//!
//! `CommandLists` walks a file down to its event commands using nothing but spans into the
//! caller's buffer. These tests build a map with the typed API, serialize it, then walk it back
//! with the allocation-free API and assert the two agree - which is the only way to be sure the
//! hardcoded chunk ids in `event::path` actually match the schema.
//!
//! Everything the core tier touches here is exercised through fixed-size buffers and
//! `SliceSink`/`NullSink`, never `Vec`, so this would still compile under `--no-default-features`
//! if the fixtures did not have to be built with the typed API first.

use rm2k::{
    ChunkIter, Ctx, DbStr, Engine, NullSink,
    event::{CommandLists, path},
    field::{LcfStruct, List},
    raw::{EventCommand, EventCommandList},
    rpg::{CommonEvent, Database, Event, EventPage, Map, Troop, TroopPage},
};

fn command(code: i32, text: &'static [u8]) -> EventCommand<'static> {
    EventCommand {
        code,
        indent: 0,
        string: DbStr::from_bytes(text),
        parameters: vec![1, 2],
    }
}

/// A map with two events, the first having two pages.
fn map_with_events() -> Vec<u8> {
    let mut map = Map::defaults(Engine::R2K);

    let mut first = Event::defaults(Engine::R2K);
    first.id = 7;
    for (page_id, text) in [(1i32, &b"page one"[..]), (2, b"page two")] {
        let mut page = EventPage::defaults(Engine::R2K);
        page.id = page_id;
        page.event_commands = EventCommandList(vec![command(10110, text)]);
        first.pages.push(page);
    }

    let mut second = Event::defaults(Engine::R2K);
    second.id = 9;
    let mut page = EventPage::defaults(Engine::R2K);
    page.id = 1;
    // Not UTF-8; the core walker must hand these bytes back untouched.
    page.event_commands = EventCommandList(vec![command(10110, &[0x82, 0xA0, 0x82, 0xA2])]);
    second.pages.push(page);

    map.events = List(vec![first, second]);

    let mut body = Vec::new();
    map.write_body(&mut body, Engine::R2K).unwrap();
    body
}

fn walk<'a>(mut lists: CommandLists<'a>, ctx: &mut Ctx<'_>) -> Vec<(i32, i32, i32, Vec<u8>)> {
    let mut out = Vec::new();
    while let Some(list) = lists.next(ctx) {
        let mut commands = list.commands();
        while let Some(cmd) = commands.next(ctx) {
            out.push((list.owner_id, list.page_id, cmd.code, cmd.string.to_vec()));
        }
    }
    out
}

#[test]
fn walks_a_maps_events_without_allocating() {
    let body = map_with_events();
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);

    let found = walk(CommandLists::in_map(&body, 0, &mut ctx), &mut ctx);

    assert_eq!(
        found,
        vec![
            (7, 1, 10110, b"page one".to_vec()),
            (7, 2, 10110, b"page two".to_vec()),
            (9, 1, 10110, vec![0x82, 0xA0, 0x82, 0xA2]),
        ]
    );
}

#[test]
fn the_core_walker_agrees_with_the_typed_api() {
    let body = map_with_events();

    // Typed path.
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut chunks = ChunkIter::new(&body, 0);
    let map = Map::read_stream(&mut chunks, &mut ctx);

    let mut expected = Vec::new();
    for event in map.events.iter() {
        for page in event.pages.iter() {
            for cmd in page.event_commands.iter() {
                expected.push((event.id, page.id, cmd.code, cmd.string.as_bytes().to_vec()));
            }
        }
    }

    // Core path.
    let found = walk(CommandLists::in_map(&body, 0, &mut ctx), &mut ctx);
    assert_eq!(found, expected);
}

#[test]
fn walks_common_events_which_have_no_pages() {
    let mut db = Database::defaults(Engine::R2K);
    let mut common = CommonEvent::defaults(Engine::R2K);
    common.id = 3;
    common.event_commands = EventCommandList(vec![command(10110, b"common")]);
    db.commonevents = List(vec![common]);

    let mut body = Vec::new();
    db.write_body(&mut body, Engine::R2K).unwrap();

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let found = walk(CommandLists::in_common_events(&body, 0, &mut ctx), &mut ctx);

    // page_id is 0: a common event has no pages.
    assert_eq!(found, vec![(3, 0, 10110, b"common".to_vec())]);
}

#[test]
fn walks_battle_events() {
    let mut db = Database::defaults(Engine::R2K);
    let mut troop = Troop::defaults(Engine::R2K);
    troop.id = 2;
    let mut page = TroopPage::defaults(Engine::R2K);
    page.id = 1;
    page.event_commands = EventCommandList(vec![command(10110, b"battle")]);
    troop.pages = List(vec![page]);
    db.troops = List(vec![troop]);

    let mut body = Vec::new();
    db.write_body(&mut body, Engine::R2K).unwrap();

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let found = walk(CommandLists::in_battle_events(&body, 0, &mut ctx), &mut ctx);

    assert_eq!(found, vec![(2, 1, 10110, b"battle".to_vec())]);
}

#[test]
fn an_empty_map_yields_nothing() {
    let mut body = Vec::new();
    Map::defaults(Engine::R2K).write_body(&mut body, Engine::R2K).unwrap();

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut lists = CommandLists::in_map(&body, 0, &mut ctx);
    assert!(lists.next(&mut ctx).is_none());
}

#[test]
fn garbage_never_panics_and_never_hangs() {
    let body = map_with_events();
    for cut in 0..body.len() {
        let mut sink = NullSink;
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut lists = CommandLists::in_map(&body[..cut], 0, &mut ctx);
        // A bound, so a corrupt count that survived the sanity check cannot loop forever.
        let mut guard = 0;
        while lists.next(&mut ctx).is_some() {
            guard += 1;
            assert!(guard < 10_000, "walker failed to terminate at cut {cut}");
        }
    }
}

#[test]
fn the_named_chunk_ids_match_the_schema() {
    // If the schema ever moves one of these, `CommandLists` would silently find nothing. Pin them
    // by checking the ids actually present in a serialised struct.
    let mut body = Vec::new();
    let mut map = Map::defaults(Engine::R2K);
    map.events = List(vec![Event::defaults(Engine::R2K)]);
    map.write_body(&mut body, Engine::R2K).unwrap();

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    assert!(
        ChunkIter::find(&body, 0, path::MAP_EVENTS, &mut ctx, "Map").is_some(),
        "MAP_EVENTS is stale"
    );

    let mut event_body = Vec::new();
    let mut event = Event::defaults(Engine::R2K);
    event.pages = List(vec![EventPage::defaults(Engine::R2K)]);
    event.write_body(&mut event_body, Engine::R2K).unwrap();
    assert!(
        ChunkIter::find(&event_body, 0, path::EVENT_PAGES, &mut ctx, "Event").is_some(),
        "EVENT_PAGES is stale"
    );

    let mut page_body = Vec::new();
    let mut page = EventPage::defaults(Engine::R2K);
    page.event_commands = EventCommandList(vec![command(1, b"x")]);
    page.write_body(&mut page_body, Engine::R2K).unwrap();
    assert!(
        ChunkIter::find(&page_body, 0, path::EVENT_PAGE_COMMANDS, &mut ctx, "EventPage").is_some(),
        "EVENT_PAGE_COMMANDS is stale"
    );
}

#[test]
fn an_owner_without_a_command_chunk_is_skipped() {
    // A common-event list holding one element, id 1, whose only chunk is its name "a". There is no
    // command chunk to find, so the walker must move past it and report nothing.
    // Outer chunk 0x19 (the list), length 6: count 1 | element id 1 | chunk 0x01 "a" | terminator.
    let body = [0x19, 0x06, 0x01, 0x01, 0x01, 0x01, 0x61, 0x00];
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let found = walk(CommandLists::in_common_events(&body, 0, &mut ctx), &mut ctx);
    assert!(found.is_empty());
}
