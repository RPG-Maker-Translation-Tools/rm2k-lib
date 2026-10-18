//! Tests for `rm2k::owned` - the `IntoOwned` detach point from a borrowed `Database<'a>` to
//! `Database<'static>`.

use rm2k::{
    DbStr, Engine, LcfStruct,
    field::UnindexedList,
    owned::IntoOwned,
    raw::MoveCommand,
    rpg::{Database, SaveEasyRpgText},
};

#[test]
fn a_database_outlives_the_buffer_it_was_parsed_from() {
    let owned: Database<'static> = {
        let mut bytes = Vec::new();
        let mut db = Database::defaults(Engine::R2K);
        db.system.system_name = DbStr::from_bytes(b"System");
        rm2k::file::save_database(&db, &mut bytes, Engine::R2K, rm2k::SaveOpt::default(), b"").unwrap();

        let loaded = rm2k::file::load_database(&bytes).unwrap();
        loaded.value.into_owned()
        // `bytes` is dropped here; `owned` must not reference it.
    };

    assert_eq!(owned.system.system_name.as_bytes(), b"System");
}

#[test]
fn detaching_preserves_every_byte() {
    let mut db = Database::defaults(Engine::R2K);
    // Deliberately not UTF-8: detaching must copy bytes, not text.
    db.system.system_name = DbStr::from_bytes(&[0x82, 0xA0, 0x82, 0xA2]);
    let detached = db.clone().into_owned();
    assert_eq!(detached.system.system_name.as_bytes(), db.system.system_name.as_bytes());
}

#[test]
fn an_unindexed_list_detaches_its_elements() {
    let mut text = SaveEasyRpgText::defaults(Engine::R2K);
    text.text = DbStr::from_bytes(&[0x82, 0xA0]);
    let detached = UnindexedList(vec![text]).into_owned();
    assert_eq!(detached[0].text.as_bytes(), &[0x82, 0xA0]);
}

#[test]
fn a_move_command_detaches_its_string() {
    let command = MoveCommand {
        code: 41,
        string: DbStr::from_bytes(b"graphic"),
        parameter_a: 1,
        parameter_b: 2,
        parameter_c: 3,
    };
    let detached = command.into_owned();
    assert_eq!(detached.string.as_bytes(), b"graphic");
    assert_eq!(detached.parameter_c, 3);
}
