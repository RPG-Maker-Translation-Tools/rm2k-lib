//! Tests for `rm2k::setup` - RPG_RT's opt-in post-load fixups.

use rm2k::{
    Engine, LcfStruct, Packed,
    rpg::{Actor, Database},
    setup::Setup,
};

#[test]
fn sentinels_resolve_per_engine() {
    let mut actor = Actor::defaults(Engine::R2K);
    assert_eq!(actor.final_level, -1, "the schema stores the sentinel");
    actor.setup(Engine::R2K);
    assert_eq!(actor.final_level, 50);
    assert_eq!(actor.exp_base, 30);
    assert_eq!(actor.exp_inflation, 30);

    let mut actor = Actor::defaults(Engine::R2K3);
    actor.setup(Engine::R2K3);
    assert_eq!(actor.final_level, 99);
    assert_eq!(actor.exp_base, 300);
    assert_eq!(actor.exp_inflation, 300);
}

#[test]
fn an_explicit_value_is_never_overwritten() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.final_level = 12;
    actor.exp_base = 7;
    actor.setup(Engine::R2K);
    assert_eq!(actor.final_level, 12);
    assert_eq!(actor.exp_base, 7);
}

#[test]
fn curves_are_padded_to_the_engine_maximum_not_the_actors_level() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.final_level = 5;
    actor.setup(Engine::R2K);
    // 50, not 5 - this is the liblcf behaviour, odd as it looks.
    assert_eq!(actor.parameters.maxhp.0.len(), 50);
    assert_eq!(actor.parameters.maxhp.0[0], 1);
    assert_eq!(actor.parameters.maxsp.0[0], 0, "maxsp fills with 0, the others with 1");
}

#[test]
fn padding_only_ever_grows() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.parameters.maxhp = Packed(vec![7; 200]);
    actor.setup(Engine::R2K);
    assert_eq!(actor.parameters.maxhp.0.len(), 200);
    assert_eq!(actor.parameters.maxhp.0[0], 7);
}

#[test]
fn setup_is_idempotent() {
    let mut once = Actor::defaults(Engine::R2K3);
    once.setup(Engine::R2K3);
    let mut twice = once.clone();
    twice.setup(Engine::R2K3);
    assert_eq!(once, twice);
}

#[test]
fn setup_changes_the_bytes_which_is_why_it_is_opt_in() {
    // The whole reason this is not run on load: it makes a load/save cycle non-idempotent.
    let mut db = Database::defaults(Engine::R2K);
    db.actors.push(Actor::defaults(Engine::R2K));

    let mut before = Vec::new();
    db.write_body(&mut before, Engine::R2K).unwrap();

    db.setup(Engine::R2K);
    let mut after = Vec::new();
    db.write_body(&mut after, Engine::R2K).unwrap();

    assert_ne!(before, after, "padding the stat curves must show up in the output");
}
