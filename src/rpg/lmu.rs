//! `LMU` structures.

#![allow(clippy::struct_excessive_bools)]

use super::{enums::*, *};
use crate::{
    field::{DbStr, List, Packed},
    lcf_strings, lcf_struct,
    raw::{EventCommandList, MoveCommandList},
};

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageCondition {
        0x01 flags: EventPageConditionFlags, persist,
        0x02 switch_a_id: i32 = 1,
        0x03 switch_b_id: i32 = 1,
        0x04 variable_id: i32 = 1,
        0x05 variable_value: i32 = 0,
        0x06 item_id: i32 = 1,
        0x07 actor_id: i32 = 1, persist,
        0x08 timer_sec: i32 = 0,
        0x09 timer2_sec: i32 = 0, is2k3,
        0x0A compare_operator: EventPageConditionComparison = EventPageConditionComparison(1), is2k3,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct MoveRoute<'a> {
        0x0C move_commands: MoveCommandList<'a>, size = 0x0B, persist,
        0x15 repeat: bool = true,
        0x16 skippable: bool = false,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct EventPage<'a> indexed {
        0x02 condition: EventPageCondition, persist,
        0x15 character_name: DbStr<'a>,
        0x16 character_index: i32 = 0,
        0x17 character_direction: EventPageDirection = EventPageDirection(2), persist,
        0x18 character_pattern: EventPageFrame = EventPageFrame(1),
        0x19 translucent: bool = false, persist,
        0x1F move_type: EventPageMoveType = EventPageMoveType(1), persist,
        0x20 move_frequency: i32 = 3,
        0x21 trigger: EventPageTrigger = EventPageTrigger(0), persist,
        0x22 layer: EventPageLayers = EventPageLayers(0), persist,
        0x23 overlap_forbidden: bool = false, persist,
        0x24 animation_type: EventPageAnimType = EventPageAnimType(0), persist,
        0x25 move_speed: EventPageMoveSpeed = EventPageMoveSpeed(3),
        0x29 move_route: MoveRoute<'a>, persist,
        0x34 event_commands: EventCommandList<'a>, size_persist = 0x33, persist,
    }
}

lcf_strings!(EventPage<'a> { character_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Event<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 x: i32 = 0,
        0x03 y: i32 = 0,
        0x05 pages: List<EventPage<'a>>, persist,
    }
}

lcf_strings!(Event<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Map<'a> {
        0x01 chipset_id: i32 = 1,
        0x02 width: i32 = 20,
        0x03 height: i32 = 15,
        0x0B scroll_type: MapScrollType = MapScrollType(0), persist,
        0x1F parallax_flag: bool = false,
        0x20 parallax_name: DbStr<'a>,
        0x21 parallax_loop_x: bool = false,
        0x22 parallax_loop_y: bool = false,
        0x23 parallax_auto_loop_x: bool = false,
        0x24 parallax_sx: i32 = 0,
        0x25 parallax_auto_loop_y: bool = false,
        0x26 parallax_sy: i32 = 0,
        0x28 generator_flag: bool = false,
        0x29 generator_mode: MapGeneratorMode = MapGeneratorMode(0),
        0x2A top_level: bool = false,
        0x30 generator_tiles: MapGeneratorTiles = MapGeneratorTiles(0),
        0x31 generator_width: i32 = 4,
        0x32 generator_height: i32 = 1,
        0x33 generator_surround: bool = true,
        0x34 generator_upper_wall: bool = true,
        0x35 generator_floor_b: bool = true,
        0x36 generator_floor_c: bool = true,
        0x37 generator_extra_b: bool = true,
        0x38 generator_extra_c: bool = true,
        0x3C generator_x: Packed<u32>,
        0x3D generator_y: Packed<u32>,
        0x3E generator_tile_ids: Packed<i16>,
        0x47 lower_layer: Packed<i16>, persist,
        0x48 upper_layer: Packed<i16>, persist,
        0x51 events: List<Event<'a>>, persist,
        0x5A save_count_2k3e: i32 = 0, is2k3,
        0x5B save_count: i32 = 0,
    }
}

lcf_strings!(Map<'a> { parallax_name });
