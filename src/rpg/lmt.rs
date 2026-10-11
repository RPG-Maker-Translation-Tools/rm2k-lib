//! `LMT` structures.

#![allow(clippy::struct_excessive_bools)]

use super::{enums::*, *};
use crate::{
    field::{DbStr, List},
    lcf_strings, lcf_struct,
    raw::Rect,
};

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Encounter indexed {
        0x01 troop_id: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct MapInfo<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 parent_map: i32 = 0,
        0x03 indentation: i32 = 0,
        0x04 r#type: TreeMapMapType = TreeMapMapType(-1),
        0x05 scrollbar_x: i32 = 0,
        0x06 scrollbar_y: i32 = 0,
        0x07 expanded_node: bool = false,
        0x0B music_type: MapInfoMusicType = MapInfoMusicType(0), persist,
        0x0C music: Music<'a>, persist,
        0x15 background_type: MapInfoBGMType = MapInfoBGMType(0), persist,
        0x16 background_name: DbStr<'a>,
        0x1F teleport: MapInfoTriState = MapInfoTriState(0), persist,
        0x20 escape: MapInfoTriState = MapInfoTriState(0), persist,
        0x21 save: MapInfoTriState = MapInfoTriState(0), persist,
        0x29 encounters: List<Encounter>, persist,
        0x2C encounter_steps: i32 = 25,
        0x33 area_rect: Rect, persist,
    }
}

lcf_strings!(MapInfo<'a> { name, background_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Start {
        0x01 party_map_id: i32 = 0,
        0x02 party_x: i32 = 0,
        0x03 party_y: i32 = 0,
        0x0B boat_map_id: i32 = 0,
        0x0C boat_x: i32 = 0,
        0x0D boat_y: i32 = 0,
        0x15 ship_map_id: i32 = 0,
        0x16 ship_x: i32 = 0,
        0x17 ship_y: i32 = 0,
        0x1F airship_map_id: i32 = 0,
        0x20 airship_x: i32 = 0,
        0x21 airship_y: i32 = 0,
    }
}
