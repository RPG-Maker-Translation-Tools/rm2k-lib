//! `LSD` structures.

#![allow(clippy::struct_excessive_bools)]

use super::{enums::*, *};
use crate::{
    field::{BitArray, DbStr, List, Packed, UnindexedList},
    lcf_strings, lcf_struct,
    raw::{EventCommandList, StrList},
};

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveTitle<'a> {
        0x01 timestamp: f64 = 0.0,
        0x0B hero_name: DbStr<'a>,
        0x0C hero_level: i32 = 0,
        0x0D hero_hp: i32 = 0,
        0x15 face1_name: DbStr<'a>,
        0x16 face1_id: i32 = 0,
        0x17 face2_name: DbStr<'a>,
        0x18 face2_id: i32 = 0,
        0x19 face3_name: DbStr<'a>,
        0x1A face3_id: i32 = 0,
        0x1B face4_name: DbStr<'a>,
        0x1C face4_id: i32 = 0,
    }
}

lcf_strings!(SaveTitle<'a> { hero_name, face1_name, face2_name, face3_name, face4_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveSystem<'a> {
        0x01 scene: SaveSystemScene = SaveSystemScene(0), persist,
        0x0B frame_count: i32 = 0,
        0x15 graphics_name: DbStr<'a>,
        0x16 message_stretch: SystemStretch = SystemStretch(0),
        0x17 font_id: SystemFont = SystemFont(0),
        0x20 switches: BitArray<'a>, count = 0x1F, persist,
        0x22 variables: Packed<i32>, count = 0x21, persist,
        0x29 message_transparent: i32 = 0,
        0x2A message_position: i32 = 2,
        0x2B message_prevent_overlap: i32 = 1,
        0x2C message_continue_events: i32 = 0,
        0x33 face_name: DbStr<'a>,
        0x34 face_id: i32 = 0,
        0x35 face_right: bool = false,
        0x36 face_flip: bool = false,
        0x37 event_message_active: bool = false,
        0x3D music_stopping: bool = false,
        0x47 title_music: Music<'a>, persist,
        0x48 battle_music: Music<'a>, persist,
        0x49 battle_end_music: Music<'a>, persist,
        0x4A inn_music: Music<'a>, persist,
        0x4B current_music: Music<'a>, persist,
        0x4C before_vehicle_music: Music<'a>, persist,
        0x4D before_battle_music: Music<'a>, persist,
        0x4E stored_music: Music<'a>, persist,
        0x4F boat_music: Music<'a>, persist,
        0x50 ship_music: Music<'a>, persist,
        0x51 airship_music: Music<'a>, persist,
        0x52 gameover_music: Music<'a>, persist,
        0x5B cursor_se: Sound<'a>, persist,
        0x5C decision_se: Sound<'a>, persist,
        0x5D cancel_se: Sound<'a>, persist,
        0x5E buzzer_se: Sound<'a>, persist,
        0x5F battle_se: Sound<'a>, persist,
        0x60 escape_se: Sound<'a>, persist,
        0x61 enemy_attack_se: Sound<'a>, persist,
        0x62 enemy_damaged_se: Sound<'a>, persist,
        0x63 actor_damaged_se: Sound<'a>, persist,
        0x64 dodge_se: Sound<'a>, persist,
        0x65 enemy_death_se: Sound<'a>, persist,
        0x66 item_se: Sound<'a>, persist,
        0x6F transition_out: i8 = -1, persist,
        0x70 transition_in: i8 = -1, persist,
        0x71 battle_start_fadeout: i8 = -1, persist,
        0x72 battle_start_fadein: i8 = -1, persist,
        0x73 battle_end_fadeout: i8 = -1, persist,
        0x74 battle_end_fadein: i8 = -1, persist,
        0x79 teleport_allowed: bool = true,
        0x7A escape_allowed: bool = true,
        0x7B save_allowed: bool = true,
        0x7C menu_allowed: bool = true,
        0x7D background: DbStr<'a>,
        0x83 save_count: i32 = 0,
        0x84 save_slot: i32 = 1,
        0x8C atb_mode: SaveSystemAtbMode = SaveSystemAtbMode(0), is2k3,
        0x24 maniac_strings: StrList<'a>,
        0x2D maniac_message_window_width: i32 = 0,
        0x2E maniac_message_window_height: i32 = 0,
        0x2F maniac_message_font_name: DbStr<'a>,
        0x30 maniac_message_font_size: i32 = 0,
        0x32 maniac_message_hook_flags: ManiacMessageHookFlags,
        0x40 maniac_message_face_width: i32 = 48,
        0x41 maniac_message_face_height: i32 = 48,
        0x42 maniac_message_hook_common_event_id: i32 = 0,
        0x43 maniac_message_hook_callback_system_variable: i32 = 0,
        0x44 maniac_message_hook_callback_system_string_variable: i32 = 0,
        0x45 maniac_message_hook_callback_user_variable: i32 = 0,
        0x46 maniac_message_hook_callback_user_string_variable: i32 = 0,
        0x85 maniac_battle_origin: i32 = 0,
        0x88 maniac_frameskip: i32 = 0,
        0x89 maniac_picture_limit: i32 = 0,
        0x8A maniac_options: Packed<u8>,
        0x8B maniac_joypad_bindings: Packed<u8>,
        0x8E maniac_message_spacing_char: i32 = 0,
        0x8F maniac_message_spacing_line: i32 = 0,
    }
}

lcf_strings!(SaveSystem<'a> { graphics_name, face_name, background, maniac_message_font_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveScreen {
        0x01 tint_finish_red: i32 = 100,
        0x02 tint_finish_green: i32 = 100,
        0x03 tint_finish_blue: i32 = 100,
        0x04 tint_finish_sat: i32 = 100,
        0x0B tint_current_red: f64 = 100.0,
        0x0C tint_current_green: f64 = 100.0,
        0x0D tint_current_blue: f64 = 100.0,
        0x0E tint_current_sat: f64 = 100.0,
        0x0F tint_time_left: i32 = 0,
        0x14 flash_continuous: bool = false,
        0x15 flash_red: i32 = 0,
        0x16 flash_green: i32 = 0,
        0x17 flash_blue: i32 = 0,
        0x18 flash_current_level: f64 = 0.0,
        0x19 flash_time_left: i32 = 0,
        0x1E shake_continuous: bool = false,
        0x1F shake_strength: i32 = 0,
        0x20 shake_speed: i32 = 0,
        0x21 shake_position: i32 = 0,
        0x22 shake_position_y: i32 = 0,
        0x23 shake_time_left: i32 = 0,
        0x29 pan_x: i32 = 0,
        0x2A pan_y: i32 = 0,
        0x2B battleanim_id: i32 = 0,
        0x2C battleanim_target: i32 = 0,
        0x2D battleanim_frame: i32 = 0,
        0x2E battleanim_active: bool = false,
        0x2F battleanim_global: bool = false,
        0x30 weather: i32 = 0,
        0x31 weather_strength: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SavePicture<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 start_x: f64 = 0.0,
        0x03 start_y: f64 = 0.0,
        0x04 current_x: f64 = 0.0,
        0x05 current_y: f64 = 0.0,
        0x06 fixed_to_map: bool = false,
        0x07 current_magnify: f64 = 100.0,
        0x08 current_top_trans: f64 = 0.0,
        0x09 use_transparent_color: bool = false,
        0x0B current_red: f64 = 100.0,
        0x0C current_green: f64 = 100.0,
        0x0D current_blue: f64 = 100.0,
        0x0E current_sat: f64 = 100.0,
        0x0F effect_mode: SavePictureEffect = SavePictureEffect(0),
        0x10 current_effect_power: f64 = 0.0,
        0x12 current_bot_trans: f64 = 0.0,
        0x13 spritesheet_cols: i32 = 1, is2k3,
        0x14 spritesheet_rows: i32 = 1, is2k3,
        0x15 spritesheet_frame: i32 = 0, is2k3,
        0x16 spritesheet_speed: i32 = 0, is2k3,
        0x17 frames: i32 = 0, is2k3,
        0x18 spritesheet_play_once: bool = false, is2k3,
        0x19 map_layer: SavePictureMapLayer = SavePictureMapLayer(7),
        0x1A battle_layer: SavePictureBattleLayer = SavePictureBattleLayer(0),
        0x1B flags: SavePictureFlags = SavePictureFlags { erase_on_map_change: true, affected_by_flash: true, affected_by_shake: true, ..SavePictureFlags::DEFAULT },
        0x1F finish_x: f64 = 0.0,
        0x20 finish_y: f64 = 0.0,
        0x21 finish_magnify: i32 = 100,
        0x22 finish_top_trans: i32 = 0,
        0x23 finish_bot_trans: i32 = 0,
        0x29 finish_red: i32 = 100,
        0x2A finish_green: i32 = 100,
        0x2B finish_blue: i32 = 100,
        0x2C finish_sat: i32 = 100,
        0x2E finish_effect_power: i32 = 0,
        0x33 time_left: i32 = 0,
        0x34 current_rotation: f64 = 0.0,
        0x35 current_waver: i32 = 0,
        0xC8 easyrpg_flip: SavePictureEasyRpgFlip = SavePictureEasyRpgFlip(0), is2k3,
        0xC9 easyrpg_blend_mode: i32 = 0, is2k3,
        0xCA easyrpg_type: i32 = 0, is2k3,
        0x0A maniac_current_magnify_height: f64 = 100.0,
        0x1C maniac_image_data: Packed<u8>,
        0x24 maniac_finish_magnify_height: i32 = 100,
    }
}

lcf_strings!(SavePicture<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SavePartyLocation<'a> {
        0x01 active: bool = true,
        0x0B map_id: i32 = 0, persist,
        0x0C position_x: i32 = 0, persist,
        0x0D position_y: i32 = 0, persist,
        0x15 direction: i32 = 2, persist,
        0x16 facing: i32 = 2, persist,
        0x17 anim_frame: i32 = 1,
        0x18 transparency: i32 = 0,
        0x1F remaining_step: i32 = 0,
        0x20 move_frequency: i32 = 2,
        0x21 layer: i32 = 1, persist,
        0x22 overlap_forbidden: bool = false,
        0x23 animation_type: EventPageAnimType = EventPageAnimType(0), persist,
        0x24 lock_facing: bool = false,
        0x25 move_speed: i32 = 4, persist,
        0x29 move_route: MoveRoute<'a>, persist,
        0x2A move_route_overwrite: bool = false,
        0x2B move_route_index: i32 = 0,
        0x2C move_route_finished: bool = false,
        0x2E sprite_hidden: bool = false,
        0x2F move_route_through: bool = false,
        0x30 anim_paused: i32 = 0,
        0x33 through: bool = false,
        0x34 stop_count: i32 = 0,
        0x35 anim_count: i32 = 0,
        0x36 max_stop_count: i32 = 0,
        0x3D jumping: bool = false,
        0x3E begin_jump_x: i32 = 0,
        0x3F begin_jump_y: i32 = 0,
        0x47 pause: bool = false,
        0x48 flying: bool = false,
        0x49 sprite_name: DbStr<'a>,
        0x4A sprite_id: i32 = 0,
        0x4B processed: bool = false,
        0x51 flash_red: i32 = -1,
        0x52 flash_green: i32 = -1,
        0x53 flash_blue: i32 = -1,
        0x54 flash_current_level: f64 = 0.0,
        0x55 flash_time_left: i32 = 0,
        0xC9 easyrpg_move_failure_count: i32 = 0,
        0xCA easyrpg_clone_map_id: u32 = 0,
        0xCB easyrpg_clone_event_id: u32 = 0,
        0xCC easyrpg_runtime_flags: EasyRpgEventRuntimeFlags,
        0x65 boarding: bool = false,
        0x66 aboard: bool = false,
        0x67 vehicle: SavePartyLocationVehicleType = SavePartyLocationVehicleType(0),
        0x68 unboarding: bool = false,
        0x69 preboard_move_speed: i32 = 4,
        0x6C menu_calling: bool = false,
        0x6F pan_state: SavePartyLocationPanState = SavePartyLocationPanState(1),
        0x70 pan_current_x: i32 = 2304,
        0x71 pan_current_y: i32 = 1792,
        0x72 pan_finish_x: i32 = 2304,
        0x73 pan_finish_y: i32 = 1792,
        0x79 pan_speed: i32 = 16,
        0x7C total_encounter_rate: i32 = 0,
        0x7D encounter_calling: bool = false,
        0x83 map_save_count: i32 = 0,
        0x84 database_save_count: i32 = 0,
        0x8D maniac_horizontal_pan_speed: f64 = 0.0,
        0x8E maniac_vertical_pan_speed: f64 = 0.0,
    }
}

lcf_strings!(SavePartyLocation<'a> { sprite_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveVehicleLocation<'a> {
        0x01 active: bool = true,
        0x0B map_id: i32 = 0, persist,
        0x0C position_x: i32 = 0, persist,
        0x0D position_y: i32 = 0, persist,
        0x15 direction: i32 = 2, persist,
        0x16 facing: i32 = 2, persist,
        0x17 anim_frame: i32 = 1,
        0x18 transparency: i32 = 0,
        0x1F remaining_step: i32 = 0,
        0x20 move_frequency: i32 = 2,
        0x21 layer: i32 = 1, persist,
        0x22 overlap_forbidden: bool = false,
        0x23 animation_type: EventPageAnimType = EventPageAnimType(0), persist,
        0x24 lock_facing: bool = false,
        0x25 move_speed: i32 = 4, persist,
        0x29 move_route: MoveRoute<'a>, persist,
        0x2A move_route_overwrite: bool = false,
        0x2B move_route_index: i32 = 0,
        0x2C move_route_finished: bool = false,
        0x2E sprite_hidden: bool = false,
        0x2F move_route_through: bool = false,
        0x30 anim_paused: i32 = 0,
        0x33 through: bool = false,
        0x34 stop_count: i32 = 0,
        0x35 anim_count: i32 = 0,
        0x36 max_stop_count: i32 = 0,
        0x3D jumping: bool = false,
        0x3E begin_jump_x: i32 = 0,
        0x3F begin_jump_y: i32 = 0,
        0x47 pause: bool = false,
        0x48 flying: bool = false,
        0x49 sprite_name: DbStr<'a>,
        0x4A sprite_id: i32 = 0,
        0x4B processed: bool = false,
        0x51 flash_red: i32 = -1,
        0x52 flash_green: i32 = -1,
        0x53 flash_blue: i32 = -1,
        0x54 flash_current_level: f64 = 0.0,
        0x55 flash_time_left: i32 = 0,
        0xC9 easyrpg_move_failure_count: i32 = 0,
        0xCA easyrpg_clone_map_id: u32 = 0,
        0xCB easyrpg_clone_event_id: u32 = 0,
        0xCC easyrpg_runtime_flags: EasyRpgEventRuntimeFlags,
        0x65 vehicle: SaveVehicleLocationVehicleType = SaveVehicleLocationVehicleType(0),
        0x6A remaining_ascent: i32 = 0,
        0x6B remaining_descent: i32 = 0,
        0x6F orig_sprite_name: DbStr<'a>,
        0x70 orig_sprite_id: i32 = 0,
    }
}

lcf_strings!(SaveVehicleLocation<'a> { sprite_name, orig_sprite_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveActor<'a> indexed {
        0x01 name: DbStr<'a> = DbStr::from_bytes(b"\x01"), persist,
        0x02 title: DbStr<'a> = DbStr::from_bytes(b"\x01"), persist,
        0x0B sprite_name: DbStr<'a>,
        0x0C sprite_id: i32 = 0,
        0x0D transparency: i32 = 0,
        0x15 face_name: DbStr<'a>,
        0x16 face_id: i32 = 0,
        0x1F level: i32 = -1,
        0x20 exp: i32 = -1,
        0x21 hp_mod: i32 = -1,
        0x22 sp_mod: i32 = -1,
        0x29 attack_mod: i32 = 0,
        0x2A defense_mod: i32 = 0,
        0x2B spirit_mod: i32 = 0,
        0x2C agility_mod: i32 = 0,
        0x34 skills: Packed<i16>, count_persist = 0x33, persist,
        0x3D equipped: Packed<i16> = Packed(alloc::vec![0i16; 5]), persist,
        0x47 current_hp: i32 = -1,
        0x48 current_sp: i32 = -1,
        0x50 battle_commands: Packed<i32> = Packed(alloc::vec![-1i32; 7]), persist, is2k3,
        0x52 status: Packed<i16>, count = 0x51, persist,
        0x53 changed_battle_commands: bool = false,
        0x5A class_id: i32 = -1,
        0x5B row: SaveActorRowType = SaveActorRowType(0), is2k3,
        0x5C two_weapon: bool = false, is2k3,
        0x5D lock_equipment: bool = false, is2k3,
        0x5E auto_battle: bool = false, is2k3,
        0x5F super_guard: bool = false, is2k3,
        0x60 battler_animation: i32 = 0, is2k3,
    }
}

lcf_strings!(SaveActor<'a> { name, title, sprite_name, face_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveInventory {
        0x02 party: Packed<i16>, count_persist = 0x01, persist,
        0x0C item_ids: Packed<i16>, count_persist = 0x0B, persist,
        0x0D item_counts: Packed<u8>, persist,
        0x0E item_usage: Packed<u8>, persist,
        0x15 gold: i32 = 0,
        0x17 timer1_frames: i32 = 0,
        0x18 timer1_active: bool = false,
        0x19 timer1_visible: bool = false,
        0x1A timer1_battle: bool = false,
        0x1B timer2_frames: i32 = 0,
        0x1C timer2_active: bool = false,
        0x1D timer2_visible: bool = false,
        0x1E timer2_battle: bool = false,
        0x20 battles: i32 = 0,
        0x21 defeats: i32 = 0,
        0x22 escapes: i32 = 0,
        0x23 victories: i32 = 0,
        0x29 turns: i32 = 0,
        0x2A steps: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveTarget indexed {
        0x01 map_id: i32 = 0,
        0x02 map_x: i32 = 0,
        0x03 map_y: i32 = 0,
        0x04 switch_on: bool = false,
        0x05 switch_id: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveEventExecFrame<'a> indexed {
        0x02 commands: EventCommandList<'a>, size_persist = 0x01, persist,
        0x0B current_command: i32 = 0,
        0x0C event_id: i32 = 0,
        0x0D triggered_by_decision_key: bool = false,
        0x16 subcommand_path: Packed<u8>, size = 0x15, persist,
        0x0E maniac_event_info: i32 = 0,
        0x0F maniac_event_id: i32 = 0,
        0x10 maniac_event_page_id: i32 = 0,
        0x11 maniac_loop_info_size: i32 = 0,
        0x12 maniac_loop_info: Packed<i32>,
        0xCC easyrpg_runtime_flags: EasyRpgFrameRuntimeFlags,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveEventExecState<'a> {
        0x01 stack: List<SaveEventExecFrame<'a>>, persist,
        0x04 show_message: bool = false,
        0x0B abort_on_escape: bool = false,
        0x0D wait_movement: bool = false,
        0x15 keyinput_wait: bool = false,
        0x16 keyinput_variable: u8 = 0,
        0x17 keyinput_all_directions: bool = false,
        0x18 keyinput_decision: i32 = 0,
        0x19 keyinput_cancel: i32 = 0,
        0x1A keyinput_2kshift_2k3numbers: i32 = 0,
        0x1B keyinput_2kdown_2k3operators: i32 = 0,
        0x1C keyinput_2kleft_2k3shift: i32 = 0,
        0x1D keyinput_2kright: i32 = 0,
        0x1E keyinput_2kup: i32 = 0,
        0x1F wait_time: i32 = 0,
        0x20 keyinput_time_variable: i32 = 0,
        0x23 keyinput_2k3down: i32 = 0,
        0x24 keyinput_2k3left: i32 = 0,
        0x25 keyinput_2k3right: i32 = 0,
        0x26 keyinput_2k3up: i32 = 0,
        0x29 keyinput_timed: bool = false,
        0x2A wait_key_enter: bool = false,
        0xC9 easyrpg_active: bool = false,
        0xCA easyrpg_string: DbStr<'a>,
        0xCB easyrpg_parameters: Packed<i32>,
        0xCC easyrpg_runtime_flags: EasyRpgStateRuntimeFlags,
    }
}

lcf_strings!(SaveEventExecState<'a> { easyrpg_string });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveMapEvent<'a> indexed {
        0x01 active: bool = true,
        0x0B map_id: i32 = 0, persist,
        0x0C position_x: i32 = 0, persist,
        0x0D position_y: i32 = 0, persist,
        0x15 direction: i32 = 2, persist,
        0x16 facing: i32 = 2, persist,
        0x17 anim_frame: i32 = 1,
        0x18 transparency: i32 = 0,
        0x1F remaining_step: i32 = 0,
        0x20 move_frequency: i32 = 2,
        0x21 layer: i32 = 1, persist,
        0x22 overlap_forbidden: bool = false,
        0x23 animation_type: EventPageAnimType = EventPageAnimType(0), persist,
        0x24 lock_facing: bool = false,
        0x25 move_speed: i32 = 4, persist,
        0x29 move_route: MoveRoute<'a>, persist,
        0x2A move_route_overwrite: bool = false,
        0x2B move_route_index: i32 = 0,
        0x2C move_route_finished: bool = false,
        0x2E sprite_hidden: bool = false,
        0x2F move_route_through: bool = false,
        0x30 anim_paused: i32 = 0,
        0x33 through: bool = false,
        0x34 stop_count: i32 = 0,
        0x35 anim_count: i32 = 0,
        0x36 max_stop_count: i32 = 0,
        0x3D jumping: bool = false,
        0x3E begin_jump_x: i32 = 0,
        0x3F begin_jump_y: i32 = 0,
        0x47 pause: bool = false,
        0x48 flying: bool = false,
        0x49 sprite_name: DbStr<'a>,
        0x4A sprite_id: i32 = 0,
        0x4B processed: bool = false,
        0x51 flash_red: i32 = -1,
        0x52 flash_green: i32 = -1,
        0x53 flash_blue: i32 = -1,
        0x54 flash_current_level: f64 = 0.0,
        0x55 flash_time_left: i32 = 0,
        0xC9 easyrpg_move_failure_count: i32 = 0,
        0xCA easyrpg_clone_map_id: u32 = 0,
        0xCB easyrpg_clone_event_id: u32 = 0,
        0xCC easyrpg_runtime_flags: EasyRpgEventRuntimeFlags,
        0x65 waiting_execution: bool = false,
        0x66 original_move_route_index: i32 = 0,
        0x67 triggered_by_decision_key: bool = false,
        0x6C parallel_event_execstate: SaveEventExecState<'a>, persist,
    }
}

lcf_strings!(SaveMapEvent<'a> { sprite_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveMapInfo<'a> {
        0x01 position_x: i32 = 0,
        0x02 position_y: i32 = 0,
        0x03 encounter_steps: i32 = -1,
        0x05 chipset_id: i32 = 0,
        0x0B events: List<SaveMapEvent<'a>>, persist,
        0x15 lower_tiles: Packed<u8> = Packed(alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143]), persist,
        0x16 upper_tiles: Packed<u8> = Packed(alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143]), persist,
        0x20 parallax_name: DbStr<'a>,
        0x21 parallax_horz: bool = false,
        0x22 parallax_vert: bool = false,
        0x23 parallax_horz_auto: bool = false,
        0x24 parallax_horz_speed: i32 = 0,
        0x25 parallax_vert_auto: bool = false,
        0x26 parallax_vert_speed: i32 = 0,
    }
}

lcf_strings!(SaveMapInfo<'a> { parallax_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveCommonEvent<'a> indexed {
        0x01 parallel_event_execstate: SaveEventExecState<'a>, persist,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePanorama {
        0x01 pan_x: i32 = 0,
        0x02 pan_y: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Save<'a> {
        0x64 title: SaveTitle<'a>,
        0x65 system: SaveSystem<'a>, persist,
        0x66 screen: SaveScreen, persist,
        0x67 pictures: List<SavePicture<'a>>, persist,
        0x68 party_location: SavePartyLocation<'a>, persist,
        0x69 boat_location: SaveVehicleLocation<'a>, persist,
        0x6A ship_location: SaveVehicleLocation<'a>, persist,
        0x6B airship_location: SaveVehicleLocation<'a>, persist,
        0x6C actors: List<SaveActor<'a>>, persist,
        0x6D inventory: SaveInventory, persist,
        0x6E targets: List<SaveTarget>, persist,
        0x6F map_info: SaveMapInfo<'a>, persist,
        0x70 panorama: SavePanorama, persist,
        0x71 foreground_event_execstate: SaveEventExecState<'a>, persist,
        0x72 common_events: List<SaveCommonEvent<'a>>, persist,
        0xC8 easyrpg_data: SaveEasyRpgData<'a>,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveEasyRpgData<'a> {
        0x01 version: i32 = 0,
        0x02 codepage: i32 = 0,
        0x64 windows: List<SaveEasyRpgWindow<'a>>,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveEasyRpgWindow<'a> indexed {
        0x01 texts: UnindexedList<SaveEasyRpgText<'a>>,
        0x02 width: i32 = 0,
        0x03 height: i32 = 0,
        0x04 system_name: DbStr<'a>,
        0x05 message_stretch: SystemStretch = SystemStretch(0),
        0x06 flags: SaveEasyRpgWindowFlags = SaveEasyRpgWindowFlags { draw_frame: true, border_margin: true },
    }
}

lcf_strings!(SaveEasyRpgWindow<'a> { system_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct SaveEasyRpgText<'a> {
        0x01 text: DbStr<'a>,
        0x02 position_x: i32 = 0,
        0x03 position_y: i32 = 0,
        0x04 font_name: DbStr<'a>,
        0x05 font_size: i32 = 12,
        0x06 letter_spacing: i32 = 0,
        0x07 line_spacing: i32 = 4,
        0x08 flags: SaveEasyRpgTextFlags = SaveEasyRpgTextFlags { draw_gradient: true, draw_shadow: true, ..SaveEasyRpgTextFlags::DEFAULT },
    }
}

lcf_strings!(SaveEasyRpgText<'a> { text, font_name });
