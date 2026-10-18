//! Sample files built with the typed API. Shared by the C API's unit tests and the fuzz seed
//! generator through `#[path]`, so they exercise the same bytes.

use rm2k::{
    DbStr, Engine, LcfStruct, SaveOpt,
    field::List,
    file,
    raw::{EventCommand, EventCommandList},
    rpg::{Event, EventPage, Map},
};

/// One event command with two parameters, carrying `text` verbatim.
pub fn command(code: i32, text: &'static [u8]) -> EventCommand<'static> {
    EventCommand {
        code,
        indent: 0,
        string: DbStr::from_bytes(text),
        parameters: vec![1, 2],
    }
}

/// A whole `LMU` file: a map with two events, the first having two pages. The second event's text
/// is not UTF-8, so a walker must hand those bytes back untouched.
pub fn map_with_events() -> Vec<u8> {
    let mut map = Map::defaults(Engine::R2K);

    let mut first = Event::defaults(Engine::R2K);
    first.id = 7;
    for (page_id, text) in [(1i32, &b"page one"[..]), (2, &b"page two"[..])] {
        let mut page = EventPage::defaults(Engine::R2K);
        page.id = page_id;
        page.event_commands = EventCommandList(vec![command(10110, text)]);
        first.pages.push(page);
    }

    let mut second = Event::defaults(Engine::R2K);
    second.id = 9;
    let mut page = EventPage::defaults(Engine::R2K);
    page.id = 1;
    page.event_commands = EventCommandList(vec![command(10110, &[0x82, 0xA0, 0x82, 0xA2])]);
    second.pages.push(page);

    map.events = List(vec![first, second]);

    file::to_vec(|out| file::save_map(&map, out, Engine::R2K, SaveOpt::default(), b""))
}
