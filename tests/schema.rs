//! Tests for the `lcf_struct!`, `lcf_flags!` and `lcf_enum!` macros (`rm2k::schema`).
//!
//! These build small synthetic types with the macros themselves, the same way `src/rpg/*.rs`
//! does for the real schema, so the tests exercise the macros' generated behaviour (companion
//! chunks, `persist`, `is2k3`, indexed lists, flag packing, unknown enum values, ...) rather than
//! any one real struct. The macros are `#[macro_export]`, so they - like everything else they
//! generate - are `pub`, and this lives as an ordinary integration test rather than inline.

use rm2k::{
    diag::{DiagKind, Diagnostic},
    {BitArray, ChunkIter, Ctx, DbStr, Engine, LcfField, LcfStruct, List, NullSink, Packed, UnknownChunks},
    {lcf_enum, lcf_flags, lcf_struct},
};

lcf_struct! {
    /// Two plain compressed-integer fields. No borrowed field, so no lifetime parameter.
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Learning indexed {
        0x01 level: i32 = 1,
        0x02 skill_id: i32 = 1,
    }
}

lcf_struct! {
    /// Exercises every modifier at once: an engine-dependent default, a 2003-only field, a
    /// borrowed string, a packed array with a companion size chunk, and an indexed list.
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Actor<'a> indexed {
        0x01 name: DbStr<'a>,
        0x08 final_level: i32 = 50,
        0x39 class_id: i32 = 0, is2k3,
        0x3F skills: List<Learning>, persist,
        0x48 state_ranks: Packed<u8>, size = 0x47,
        0x4A attribute_ranks: BitArray<'a>, count = 0x49,
    }
}

lcf_flags! {
    /// Modelled on `RPG::TroopPageCondition`, including its 2003-only tail.
    pub struct CondFlags {
        switch_a,
        switch_b,
        variable,
        turn,
        fatigue,
        enemy_hp,
        actor_hp,
        turn_enemy is2k3,
        turn_actor is2k3,
        command_actor is2k3,
    }
}

lcf_enum! {
    /// `RPG::Actor::Condition`
    pub struct ActorCondition {
        NORMAL = 0,
        POISON = 1,
    }
}

fn read_actor(buf: &[u8], engine: Engine) -> Actor<'_> {
    let mut sink = NullSink;
    let mut ctx = Ctx::new(engine, &mut sink);
    let mut chunks = ChunkIter::new(buf, 0);
    Actor::read_stream(&mut chunks, &mut ctx)
}

#[test]
fn defaults_do_not_depend_on_the_engine() {
    // Worth pinning: liblcf's `50|99` notation looks like a per-engine default and is not one.
    // Its generator turns any piped default into a -1 sentinel and resolves the real value
    // afterwards in `Actor::Setup(bool)`, so nothing here varies by engine.
    assert_eq!(
        Actor::defaults(Engine::R2K).final_level,
        Actor::defaults(Engine::R2K3).final_level
    );
    assert_eq!(Learning::defaults(Engine::R2K).level, 1);
}

#[test]
fn absent_chunks_keep_defaults() {
    let actor = read_actor(&[0x00], Engine::R2K3);
    assert_eq!(actor.final_level, 50);
    assert!(actor.name.is_empty());
    assert!(actor.skills.is_empty());
}

#[test]
fn reads_a_string_and_an_integer() {
    // 0x01 len=3 "Bob" | 0x08 len=1 0x2A | terminator
    let buf = [0x01, 0x03, b'B', b'o', b'b', 0x08, 0x01, 0x2A, 0x00];
    let actor = read_actor(&buf, Engine::R2K);
    assert_eq!(actor.name.as_bytes(), b"Bob");
    assert_eq!(actor.final_level, 42);
}

#[test]
fn unknown_chunks_do_not_derail_the_fields_around_them() {
    let buf = [0x7F, 0x02, 0xDE, 0xAD, 0x01, 0x03, b'B', b'o', b'b', 0x00];
    let actor = read_actor(&buf, Engine::R2K);
    assert_eq!(actor.name.as_bytes(), b"Bob");
    // Kept rather than dropped; see the `unknown chunks` section at the bottom of this file.
    assert_eq!(actor.unknown_chunks.len(), 1);
}

#[test]
fn unknown_chunks_are_reported() {
    let buf = [0x7F, 0x02, 0xDE, 0xAD, 0x00];
    let mut sink: Vec<Diagnostic> = Vec::new();
    // Scoped so the context releases its borrow of `sink` before the assertions read it.
    {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut chunks = ChunkIter::new(&buf, 0);
        let _ = Actor::read_stream(&mut chunks, &mut ctx);
    }
    assert_eq!(sink.len(), 1);
    assert_eq!(sink[0].kind, DiagKind::UnknownChunk);
    assert_eq!(sink[0].chunk_id, 0x7F);
    assert_eq!(sink[0].struct_name, "Actor");
}

#[test]
fn companion_chunks_are_recognised_and_discarded() {
    // 0x47 is the size companion of 0x48; it must not be reported as unknown.
    let buf = [0x47, 0x01, 0x03, 0x48, 0x03, 0x01, 0x02, 0x03, 0x00];
    let mut sink: Vec<Diagnostic> = Vec::new();
    // Scoped so the context releases its borrow of `sink` before the assertions read it.
    let actor = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut chunks = ChunkIter::new(&buf, 0);
        Actor::read_stream(&mut chunks, &mut ctx)
    };
    assert!(sink.is_empty(), "companion chunk must not be reported: {sink:?}");
    assert_eq!(&actor.state_ranks.0, &[1, 2, 3]);
}

#[test]
fn companion_size_chunk_is_written_before_its_sibling() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.state_ranks = Packed(vec![7, 8]);
    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();

    // skills is `persist`: 0x3F, len 1, varint(0). Then the companion, then the sibling.
    assert_eq!(out, vec![0x3F, 0x01, 0x00, 0x47, 0x01, 0x02, 0x48, 0x02, 7, 8]);
}

#[test]
fn companion_count_chunk_holds_the_element_count_not_the_byte_length() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.attribute_ranks = BitArray::from_bytes(&[0, 1, 0, 1, 1]);
    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();
    // 0x49 companion carries 5 (elements), 0x4A carries the 5 raw bytes.
    let expected = vec![0x3F, 0x01, 0x00, 0x49, 0x01, 0x05, 0x4A, 0x05, 0, 1, 0, 1, 1];
    assert_eq!(out, expected);
}

#[test]
fn companion_is_elided_when_the_element_count_matches_the_default() {
    let actor = Actor::defaults(Engine::R2K);
    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();
    assert!(!out.contains(&0x47), "empty array must not emit a size companion");
    assert!(!out.contains(&0x49));
}

#[test]
fn nested_indexed_list_round_trips() {
    let actor = Actor {
        id: 3,
        name: DbStr::from_bytes(b"Alex"),
        final_level: 50,
        class_id: 0,
        skills: List(vec![
            Learning {
                id: 1,
                level: 5,
                skill_id: 12,
                unknown_chunks: UnknownChunks::new(),
            },
            Learning {
                id: 2,
                level: 9,
                skill_id: 7,
                unknown_chunks: UnknownChunks::new(),
            },
        ]),
        state_ranks: Packed(vec![1, 2, 3]),
        attribute_ranks: BitArray::from_bytes(&[1, 0]),
        unknown_chunks: UnknownChunks::new(),
    };

    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();
    assert_eq!(
        out.len() as u32,
        actor.body_size(Engine::R2K),
        "size must predict write"
    );

    let mut round = read_actor(&out, Engine::R2K);
    round.id = 3; // supplied by the containing list, not by the chunk stream
    assert_eq!(round, actor);
}

#[test]
fn default_fields_are_elided() {
    let actor = Actor::defaults(Engine::R2K);
    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();
    // Only the persisted empty list: id 0x3F, length 1, payload varint(0).
    assert_eq!(out, vec![0x3F, 0x01, 0x00]);
}

#[test]
fn r2k3_only_fields_are_skipped_for_r2k() {
    let mut actor = Actor::defaults(Engine::R2K);
    actor.class_id = 7;
    let mut as_2k = Vec::new();
    actor.write_body(&mut as_2k, Engine::R2K).unwrap();
    assert!(!as_2k.contains(&0x39), "2003-only chunk must not appear in a 2000 file");

    let mut actor3 = Actor::defaults(Engine::R2K3);
    actor3.class_id = 7;
    let mut as_2k3 = Vec::new();
    actor3.write_body(&mut as_2k3, Engine::R2K3).unwrap();
    assert!(as_2k3.contains(&0x39));
}

#[test]
fn size_agrees_with_write_for_every_engine() {
    for engine in [Engine::R2K, Engine::R2K3] {
        let mut actor = Actor::defaults(engine);
        actor.name = DbStr::from_bytes(b"a longer name");
        actor.class_id = 12;
        actor.state_ranks = Packed(vec![9; 40]);
        actor.skills = List(vec![Learning {
            id: 1,
            level: 2,
            skill_id: 3,
            unknown_chunks: UnknownChunks::new(),
        }]);

        let mut counted = rm2k::sink::CountSink::new();
        actor.write_body(&mut counted, engine).unwrap();
        assert_eq!(counted.len(), u64::from(actor.body_size(engine)), "{engine:?}");
    }
}

#[test]
fn nested_struct_field_round_trips() {
    lcf_struct! {
        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
        pub struct Sound<'a> {
            0x01 name: DbStr<'a>,
            0x02 volume: i32 = 100,
        }
    }
    lcf_struct! {
        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
        pub struct Holder<'a> {
            0x01 sound: Sound<'a>,
            0x02 label: DbStr<'a>,
        }
    }

    let holder = Holder {
        sound: Sound {
            name: DbStr::from_bytes(b"bell"),
            volume: 80,
            unknown_chunks: UnknownChunks::new(),
        },
        label: DbStr::from_bytes(b"x"),
        unknown_chunks: UnknownChunks::new(),
    };
    let mut out = Vec::new();
    holder.write_body(&mut out, Engine::R2K).unwrap();
    assert_eq!(out.len() as u32, holder.body_size(Engine::R2K));

    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    let mut chunks = ChunkIter::new(&out, 0);
    assert_eq!(Holder::read_stream(&mut chunks, &mut ctx), holder);
}

// ---- flags -------------------------------------------------------------------------------

fn read_flags(data: &[u8]) -> CondFlags {
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    <CondFlags as LcfField>::read(data, &mut ctx, "T", 1)
}

#[test]
fn flags_pack_lsb_first() {
    // bit 0 and bit 2 set => switch_a and variable.
    let f = read_flags(&[0b0000_0101]);
    assert!(f.switch_a);
    assert!(!f.switch_b);
    assert!(f.variable);
}

#[test]
fn flags_spanning_two_bytes() {
    // Flag 7 (turn_enemy) is bit 7 of byte 0; flag 8 (turn_actor) is bit 0 of byte 1.
    let f = read_flags(&[0b1000_0000, 0b0000_0001]);
    assert!(f.turn_enemy);
    assert!(f.turn_actor);
    assert!(!f.command_actor);
}

#[test]
fn flags_past_the_payload_read_as_false() {
    let f = read_flags(&[0xFF]);
    assert!(f.turn_enemy, "bit 7 is present");
    assert!(!f.turn_actor, "bit 8 is past the one-byte payload");
}

#[test]
fn flags_round_trip_under_r2k3() {
    let flags = CondFlags {
        switch_a: true,
        variable: true,
        turn_enemy: true,
        command_actor: true,
        ..CondFlags::default()
    };
    let mut out = Vec::new();
    flags.write(&mut out, Engine::R2K3).unwrap();
    assert_eq!(out.len() as u32, flags.lcf_size(Engine::R2K3));
    assert_eq!(read_flags(&out), flags);
}

#[test]
fn r2k_output_drops_the_2k3_flags_and_shrinks_to_one_byte() {
    let flags = CondFlags {
        switch_a: true,
        turn_enemy: true,
        ..CondFlags::default()
    };
    let mut out = Vec::new();
    flags.write(&mut out, Engine::R2K).unwrap();
    // Seven 2000-era flags fit in one byte; the three 2003 flags are not written at all.
    assert_eq!(flags.lcf_size(Engine::R2K), 1);
    assert_eq!(flags.lcf_size(Engine::R2K3), 2);
    assert_eq!(out, vec![0b0000_0001]);
}

// ---- enums -------------------------------------------------------------------------------

#[test]
fn enum_constants_and_names() {
    assert_eq!(ActorCondition::NORMAL.get(), 0);
    assert_eq!(ActorCondition::POISON.name(), Some("POISON"));
    assert_eq!(ActorCondition(99).name(), None);
}

#[test]
fn unknown_enum_values_survive_a_round_trip() {
    // A real Rust `enum` would have to reject or lose this; the newtype keeps it exactly.
    let value = ActorCondition(4242);
    let mut out = Vec::new();
    value.write(&mut out, Engine::R2K).unwrap();
    let mut sink = NullSink;
    let mut ctx = Ctx::new(Engine::R2K, &mut sink);
    assert_eq!(<ActorCondition as LcfField>::read(&out, &mut ctx, "T", 1), value);
}

// ---- unrecognised chunks -----------------------------------------------------------------

/// `id`, `len`, payload - one chunk, hand-built.
fn chunk(id: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![id, payload.len() as u8];
    out.extend_from_slice(payload);
    out
}

/// `Actor::skills` is `persist`, so an empty list is still emitted at the end of every body.
/// Nothing to do with unknown chunks - it just has to be in every expected buffer below.
fn empty_skills() -> Vec<u8> {
    chunk(0x3F, &[0])
}

#[test]
fn unknown_chunks_are_kept_and_written_back_in_place() {
    // 0x02 and 0x40 belong to no `Actor` field; 0x01 and 0x08 do. 0x40 sits between two known
    // chunks, so it also pins that the anchor puts it back in the middle rather than at an end.
    let mut buf = chunk(0x01, b"Alex");
    buf.extend(chunk(0x02, b"\xDE\xAD"));
    buf.extend(chunk(0x08, &[60]));
    buf.extend(chunk(0x40, b"\xBE\xEF"));

    let actor = read_actor(&buf, Engine::R2K);
    assert_eq!(actor.name.as_bytes(), b"Alex");
    assert_eq!(actor.final_level, 60);

    let kept: Vec<(u32, u32, &[u8])> = actor
        .unknown_chunks
        .iter()
        .map(|chunk| (chunk.id, chunk.anchor, chunk.data.as_slice()))
        .collect();
    assert_eq!(
        kept,
        vec![
            // Anchored to 0x01, the last field recognised before it.
            (0x02, 0x01, &b"\xDE\xAD"[..]),
            (0x40, 0x08, &b"\xBE\xEF"[..]),
        ]
    );

    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();

    let mut expected = buf.clone();
    expected.extend(empty_skills());
    assert_eq!(
        out, expected,
        "unknown chunks must be written back where they were found"
    );
    assert_eq!(
        out.len() as u32,
        actor.body_size(Engine::R2K),
        "size must predict write"
    );
}

#[test]
fn an_unknown_chunk_before_any_known_one_anchors_to_zero() {
    let mut buf = chunk(0x40, b"\x01");
    buf.extend(chunk(0x01, b"Alex"));

    let actor = read_actor(&buf, Engine::R2K);
    assert_eq!(actor.unknown_chunks.len(), 1);
    assert_eq!(actor.unknown_chunks[0].anchor, 0, "0 means 'before every field'");

    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();

    let mut expected = buf.clone();
    expected.extend(empty_skills());
    assert_eq!(out, expected);
}

#[test]
fn an_unknown_chunk_survives_its_anchor_field_being_elided() {
    // 0x39 is `is2k3`, so writing as 2000 skips it - the chunk anchored to it must still land.
    let mut buf = chunk(0x01, b"Alex");
    buf.extend(chunk(0x39, &[7]));
    buf.extend(chunk(0x40, b"\xBE\xEF"));

    let actor = read_actor(&buf, Engine::R2K3);
    assert_eq!(actor.class_id, 7);
    assert_eq!(actor.unknown_chunks[0].anchor, 0x39);

    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();

    let mut expected = chunk(0x01, b"Alex");
    expected.extend(chunk(0x40, b"\xBE\xEF"));
    expected.extend(empty_skills());
    assert_eq!(out, expected);
}

#[test]
fn a_kept_chunk_is_still_reported_as_a_diagnostic() {
    // Preserved is not understood: a caller asking for strictness must still hear about it.
    let blank = Diagnostic {
        offset: 0,
        chunk_id: 0,
        struct_name: "",
        kind: DiagKind::UnknownChunk,
    };
    let mut slots = [blank; 4];
    let mut sink = rm2k::SliceSink::new(&mut slots);
    let buf = chunk(0x40, b"\x01");

    let actor = {
        let mut ctx = Ctx::new(Engine::R2K, &mut sink);
        let mut chunks = ChunkIter::new(&buf, 0);
        Actor::read_stream(&mut chunks, &mut ctx)
    };

    assert_eq!(actor.unknown_chunks.len(), 1);
    assert_eq!(sink.as_slice().len(), 1);
    assert_eq!(sink.as_slice()[0].kind, DiagKind::UnknownChunk);
    assert_eq!(sink.as_slice()[0].chunk_id, 0x40);
}

#[test]
fn unknown_chunks_nested_in_a_list_element_round_trip() {
    // The store is per struct, so a list element keeps its own - the case a single file-level
    // side table could not represent.
    let mut elem = chunk(0x01, &[5]);
    elem.extend(chunk(0x40, b"\xBE\xEF"));
    elem.push(0); // element terminator

    let mut payload = vec![1u8, 3u8]; // one element, id 3
    payload.extend(elem);

    let mut buf = chunk(0x01, b"Alex");
    buf.extend(chunk(0x3F, &payload));

    let actor = read_actor(&buf, Engine::R2K);
    assert_eq!(actor.skills.len(), 1);
    assert_eq!(actor.skills[0].level, 5);
    assert_eq!(actor.skills[0].unknown_chunks.len(), 1);
    assert_eq!(actor.skills[0].unknown_chunks[0].id, 0x40);

    let mut out = Vec::new();
    actor.write_body(&mut out, Engine::R2K).unwrap();
    assert_eq!(out, buf);
}
