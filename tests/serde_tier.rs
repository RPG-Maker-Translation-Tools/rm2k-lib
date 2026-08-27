//! The `serde` tier, checked against the property that matters: LCF -> JSON -> LCF must not
//! change a byte.
//!
//! Serialising is easy to get subtly wrong for this format because text is not text - it is bytes
//! in an unknown encoding. A representation that looks fine on ASCII and quietly mangles
//! Shift-JIS would pass any casual inspection, so every test here runs non-UTF-8 data through.

#![cfg(feature = "serde")]

use rm2k::{
    field::{LcfStruct, List, Packed, UnknownChunks},
    raw::{EventCommand, EventCommandList},
    rpg::{Actor, Database, Learning, Map, TreeMap},
    {DbStr, Engine, SaveOpt, file},
};

/// Shift-JIS for "あい" - deliberately not valid UTF-8.
const SHIFT_JIS: &[u8] = &[0x82, 0xA0, 0x82, 0xA2];

fn populated() -> Database<'static> {
    let mut db = Database::defaults(Engine::R2K);
    db.system.system_name = DbStr::from_vec(SHIFT_JIS.to_vec());

    let mut actor = Actor::defaults(Engine::R2K);
    actor.id = 1;
    actor.name = DbStr::from_vec(SHIFT_JIS.to_vec());
    actor.title = DbStr::from_bytes(b"Hero");
    actor.state_ranks = Packed(vec![1, 2, 3]);
    actor.skills = List(vec![Learning {
        id: 1,
        level: 5,
        skill_id: 2,
        unknown_chunks: UnknownChunks::new(),
    }]);
    db.actors.push(actor);

    db
}

#[test]
fn a_database_survives_json() {
    let db = populated();
    let json = serde_json::to_string(&db).unwrap();
    let back: Database<'static> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, db);
}

#[test]
fn shift_jis_survives_json_intact() {
    let db = populated();
    let json = serde_json::to_string(&db).unwrap();
    let back: Database<'static> = serde_json::from_str(&json).unwrap();

    assert_eq!(back.actors[0].name.as_bytes(), SHIFT_JIS);
    // And it must not have been mangled into replacement characters on the way through.
    assert!(!json.contains('\u{FFFD}'));
    assert!(
        json.contains("\\\\x82"),
        "bytes escape rather than being lost: {json:.200}"
    );
}

#[test]
fn lcf_to_json_to_lcf_is_byte_identical() {
    // The property the whole tier exists for.
    let db = populated();

    let mut original = Vec::new();
    file::save_database(&db, &mut original, Engine::R2K, SaveOpt::default(), b"").unwrap();

    let json = serde_json::to_string(&db).unwrap();
    let round: Database<'static> = serde_json::from_str(&json).unwrap();

    let mut after = Vec::new();
    file::save_database(&round, &mut after, Engine::R2K, SaveOpt::default(), b"").unwrap();

    assert_eq!(original, after);
}

#[test]
fn ascii_text_stays_readable_in_the_json() {
    let db = populated();
    let json = serde_json::to_string(&db).unwrap();
    // The point of the escape rather than a byte array: a human can read and edit this.
    assert!(json.contains("\"Hero\""), "ASCII must not become an array of numbers");
}

#[test]
fn a_map_with_event_commands_survives_json() {
    let mut map = Map::defaults(Engine::R2K);
    let mut event = rm2k::rpg::Event::defaults(Engine::R2K);
    event.id = 1;
    let mut page = rm2k::rpg::EventPage::defaults(Engine::R2K);
    page.id = 1;
    page.event_commands = EventCommandList(vec![EventCommand {
        code: 10110,
        indent: 0,
        string: DbStr::from_vec(SHIFT_JIS.to_vec()),
        parameters: vec![1, 2, 3],
    }]);
    event.pages = List(vec![page]);
    map.events = List(vec![event]);

    let json = serde_json::to_string(&map).unwrap();
    let back: Map<'static> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, map);
    assert_eq!(back.events[0].pages[0].event_commands[0].string.as_bytes(), SHIFT_JIS);
}

#[test]
fn a_tree_map_survives_json() {
    let mut tree = TreeMap::defaults(Engine::R2K);
    tree.active_node = 3;
    tree.tree_order = vec![0, 1, 2];
    let mut info = rm2k::rpg::MapInfo::defaults(Engine::R2K);
    info.id = 1;
    info.name = DbStr::from_vec(SHIFT_JIS.to_vec());
    tree.maps = List(vec![info]);

    let json = serde_json::to_string(&tree).unwrap();
    let back: TreeMap<'static> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, tree);
}

#[test]
fn a_bit_array_serialises_as_booleans_not_as_text() {
    let mut skill = rm2k::rpg::Skill::defaults(Engine::R2K);
    skill.state_effects = rm2k::BitArray::from_vec(vec![0, 1, 1, 0]);
    let json = serde_json::to_string(&skill).unwrap();
    assert!(json.contains("[false,true,true,false]"), "got {json:.400}");

    let back: rm2k::rpg::Skill<'static> = serde_json::from_str(&json).unwrap();
    assert_eq!(back.state_effects.as_bytes(), &[0, 1, 1, 0]);
}

#[test]
fn a_deserialised_value_owns_its_data() {
    // `Database<'static>` from a short-lived string: only possible because text deserialises
    // owned rather than borrowing from the input.
    let detached: Database<'static> = {
        let json = serde_json::to_string(&populated()).unwrap();
        serde_json::from_str(&json).unwrap()
    };
    assert_eq!(detached.actors[0].name.as_bytes(), SHIFT_JIS);
}

#[test]
fn enums_and_flags_serialise() {
    let mut troop_page = rm2k::rpg::TroopPageCondition::defaults(Engine::R2K);
    troop_page.flags.switch_a = true;
    let json = serde_json::to_string(&troop_page).unwrap();
    let back: rm2k::rpg::TroopPageCondition = serde_json::from_str(&json).unwrap();
    assert_eq!(back, troop_page);
    assert!(back.flags.switch_a);
}

#[test]
fn an_unknown_enum_value_survives() {
    // The reason enumerations are transparent newtypes rather than real Rust enums: RPG Maker
    // writes values the schema does not name, and a real enum would have to reject or lose them.
    let mut animation = rm2k::rpg::Animation::defaults(Engine::R2K);
    animation.scope = rm2k::rpg::enums::AnimationScope(4242);
    let json = serde_json::to_string(&animation).unwrap();
    let back: rm2k::rpg::Animation<'static> = serde_json::from_str(&json).unwrap();
    assert_eq!(back.scope.get(), 4242);
}
