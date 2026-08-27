//! Tests for `rm2k::owned` - the `IntoOwned` detach point from a borrowed `Database<'a>` to
//! `Database<'static>`.

use rm2k::{DbStr, Engine, LcfStruct, owned::IntoOwned, rpg::Database};

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
