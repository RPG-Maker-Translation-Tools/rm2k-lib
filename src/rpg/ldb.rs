//! `LDB` structures.

#![allow(clippy::struct_excessive_bools)]

use super::{enums::*, *};
use crate::{
    field::{BitArray, DbStr, List, Packed},
    lcf_strings, lcf_struct,
    raw::{DatabaseVersion, EmptyBlock, Equipment, EventCommandList, Parameters},
};

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct Learning indexed {
        0x01 level: i32 = 1,
        0x02 skill_id: i32 = 1,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Actor<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 title: DbStr<'a>,
        0x03 character_name: DbStr<'a>,
        0x04 character_index: i32 = 0,
        0x05 transparent: bool = false,
        0x07 initial_level: i32 = 1,
        0x08 final_level: i32 = -1,
        0x09 critical_hit: bool = true,
        0x0A critical_hit_chance: i32 = 30,
        0x0F face_name: DbStr<'a>,
        0x10 face_index: i32 = 0,
        0x15 two_weapon: bool = false,
        0x16 lock_equipment: bool = false,
        0x17 auto_battle: bool = false,
        0x18 super_guard: bool = false,
        0x1F parameters: Parameters, persist,
        0x29 exp_base: i32 = -1,
        0x2A exp_inflation: i32 = -1,
        0x2B exp_correction: i32 = 0,
        0x33 initial_equipment: Equipment, persist,
        0x38 unarmed_animation: i32 = 1,
        0x39 class_id: i32 = 0, is2k3,
        0x3B battle_x: i32 = 220, is2k3,
        0x3C battle_y: i32 = 120, is2k3,
        0x3E battler_animation: i32 = 1, is2k3,
        0x3F skills: List<Learning>, persist,
        0x42 rename_skill: bool = false,
        0x43 skill_name: DbStr<'a>,
        0x48 state_ranks: Packed<u8>, size = 0x47, persist,
        0x4A attribute_ranks: Packed<u8>, size = 0x49, persist,
        0x50 battle_commands: Packed<i32>, persist, is2k3,
        0xC9 easyrpg_actorai: i32 = -1,
        0xCA easyrpg_prevent_critical: bool = false,
        0xCB easyrpg_raise_evasion: bool = false,
        0xCC easyrpg_immune_to_attribute_downshifts: bool = false,
        0xCD easyrpg_ignore_evasion: bool = false,
        0xCE easyrpg_unarmed_hit: i32 = -1,
        0xD0 easyrpg_unarmed_state_set: BitArray<'a>, size = 0xCF,
        0xD1 easyrpg_unarmed_state_chance: i32 = 0,
        0xD3 easyrpg_unarmed_attribute_set: BitArray<'a>, size = 0xD2,
        0xD4 easyrpg_dual_attack: bool = false,
        0xD5 easyrpg_attack_all: bool = false,
    }
}

lcf_strings!(Actor<'a> { name, title, character_name, face_name, skill_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Sound<'a> {
        0x01 name: DbStr<'a> = DbStr::from_bytes(b"(OFF)"), persist,
        0x03 volume: i32 = 100,
        0x04 tempo: i32 = 100,
        0x05 balance: i32 = 50,
    }
}

lcf_strings!(Sound<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct AnimationTiming<'a> indexed {
        0x01 frame: i32 = 0,
        0x02 se: Sound<'a>, persist,
        0x03 flash_scope: AnimationTimingFlashScope = AnimationTimingFlashScope(0), persist,
        0x04 flash_red: i32 = 31,
        0x05 flash_green: i32 = 31,
        0x06 flash_blue: i32 = 31,
        0x07 flash_power: i32 = 31,
        0x08 screen_shake: AnimationTimingScreenShake = AnimationTimingScreenShake(0),
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationCellData indexed {
        0x01 valid: i32 = 1,
        0x02 cell_id: i32 = 0,
        0x03 x: i32 = 0,
        0x04 y: i32 = 0,
        0x05 zoom: i32 = 100,
        0x06 tone_red: i32 = 100,
        0x07 tone_green: i32 = 100,
        0x08 tone_blue: i32 = 100,
        0x09 tone_gray: i32 = 100,
        0x0A transparency: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationFrame indexed {
        0x01 cells: List<AnimationCellData>, persist,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Animation<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 animation_name: DbStr<'a>,
        0x03 large: bool = false,
        0x06 timings: List<AnimationTiming<'a>>, persist,
        0x09 scope: AnimationScope = AnimationScope(0), persist,
        0x0A position: AnimationPosition = AnimationPosition(2), persist,
        0x0C frames: List<AnimationFrame>, persist,
    }
}

lcf_strings!(Animation<'a> { name, animation_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Attribute<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 r#type: AttributeType = AttributeType(0), persist,
        0x0B a_rate: i32 = 300,
        0x0C b_rate: i32 = 200,
        0x0D c_rate: i32 = 100,
        0x0E d_rate: i32 = 50,
        0x0F e_rate: i32 = 0,
    }
}

lcf_strings!(Attribute<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct BattleCommand<'a> indexed {
        0x01 name: DbStr<'a>, persist,
        0x02 r#type: BattleCommandType = BattleCommandType(0),
    }
}

lcf_strings!(BattleCommand<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct BattleCommands<'a> {
        0x02 placement: BattleCommandsPlacement = BattleCommandsPlacement(0),
        0x04 death_handler_unused: bool = false, is2k3,
        0x06 row: BattleCommandsRowShown = BattleCommandsRowShown(0),
        0x07 battle_type: BattleCommandsBattleType = BattleCommandsBattleType(0),
        0x09 unused_display_normal_parameters: bool = true,
        0x0A commands: List<BattleCommand<'a>>, persist,
        0x0F death_handler: bool = false, is2k3,
        0x10 death_event: i32 = 1,
        0x14 window_size: BattleCommandsWindowSize = BattleCommandsWindowSize(0),
        0x18 transparency: BattleCommandsTransparency = BattleCommandsTransparency(0),
        0x19 death_teleport: bool = false,
        0x1A death_teleport_id: i32 = 1,
        0x1B death_teleport_x: i32 = 0,
        0x1C death_teleport_y: i32 = 0,
        0x1D death_teleport_face: BattleCommandsFacing = BattleCommandsFacing(0),
        0xC8 easyrpg_default_atb_mode: SaveSystemAtbMode = SaveSystemAtbMode(0), is2k3,
        0xC9 easyrpg_enable_battle_row_command: bool = true, is2k3,
        0xCA easyrpg_sequential_order: bool = false, is2k3,
        0xCB easyrpg_disable_row_feature: bool = false, is2k3,
        0xCC easyrpg_fixed_actor_facing_direction: i32 = 0, is2k3,
        0xCD easyrpg_fixed_enemy_facing_direction: i32 = 0, is2k3,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct BattlerAnimation<'a> indexed {
        0x01 name: DbStr<'a>, is2k3,
        0x02 speed: BattlerAnimationSpeed = BattlerAnimationSpeed(20), is2k3,
        0x0A poses: List<BattlerAnimationPose<'a>>, persist, is2k3,
        0x0B weapons: List<BattlerAnimationWeapon<'a>>, persist, is2k3,
    }
}

lcf_strings!(BattlerAnimation<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationItemSkill indexed {
        0x02 unknown02: i32 = 0, is2k3,
        0x03 r#type: BattlerAnimationItemSkillAnimType = BattlerAnimationItemSkillAnimType(0), is2k3,
        0x04 weapon_animation_id: i32 = 0, is2k3,
        0x05 movement: BattlerAnimationItemSkillMovement = BattlerAnimationItemSkillMovement(0), is2k3,
        0x06 after_image: BattlerAnimationItemSkillAfterimage = BattlerAnimationItemSkillAfterimage(0), is2k3,
        0x07 attacks: i32 = 0, is2k3,
        0x08 ranged: bool = false, is2k3,
        0x09 ranged_animation_id: i32 = 0, is2k3,
        0x0C ranged_speed: BattlerAnimationItemSkillSpeed = BattlerAnimationItemSkillSpeed(0), is2k3,
        0x0D battle_animation_id: i32 = 1, is2k3,
        0x0E pose: i32 = 3, is2k3,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct BattlerAnimationPose<'a> indexed {
        0x01 name: DbStr<'a>, is2k3,
        0x02 battler_name: DbStr<'a>, is2k3,
        0x03 battler_index: i32 = 0, is2k3,
        0x04 animation_type: BattlerAnimationPoseAnimType = BattlerAnimationPoseAnimType(0), is2k3,
        0x05 battle_animation_id: i32 = 1, is2k3,
    }
}

lcf_strings!(BattlerAnimationPose<'a> { name, battler_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct BattlerAnimationWeapon<'a> indexed {
        0x01 name: DbStr<'a>, is2k3,
        0x02 weapon_name: DbStr<'a>, is2k3,
        0x03 weapon_index: i32 = 0, is2k3,
    }
}

lcf_strings!(BattlerAnimationWeapon<'a> { name, weapon_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Chipset<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 chipset_name: DbStr<'a>,
        0x03 terrain_data: Packed<i16> = Packed(alloc::vec![1i16; 162]),
        0x04 passable_data_lower: Packed<u8> = Packed(alloc::vec![15u8; 162]),
        0x05 passable_data_upper: Packed<u8> = Packed(alloc::vec![31, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15]),
        0x0B animation_type: ChipsetAnimType = ChipsetAnimType(0),
        0x0C animation_speed: i32 = 0,
    }
}

lcf_strings!(Chipset<'a> { name, chipset_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Class<'a> indexed {
        0x01 name: DbStr<'a>,
        0x15 two_weapon: bool = false,
        0x16 lock_equipment: bool = false,
        0x17 auto_battle: bool = false,
        0x18 super_guard: bool = false,
        0x1F parameters: Parameters,
        0x29 exp_base: i32 = 300,
        0x2A exp_inflation: i32 = 300,
        0x2B exp_correction: i32 = 0,
        0x3E battler_animation: i32 = 0,
        0x3F skills: List<Learning>, persist,
        0x48 state_ranks: Packed<u8>, size = 0x47, persist,
        0x4A attribute_ranks: Packed<u8>, size = 0x49, persist,
        0x50 battle_commands: Packed<i32>, persist,
    }
}

lcf_strings!(Class<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct CommonEvent<'a> indexed {
        0x01 name: DbStr<'a>,
        0x0B trigger: CommonEventTrigger = CommonEventTrigger(5), persist,
        0x0C switch_flag: bool = false,
        0x0D switch_id: i32 = 1,
        0x16 event_commands: EventCommandList<'a>, size_persist = 0x15, persist,
    }
}

lcf_strings!(CommonEvent<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Skill<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 description: DbStr<'a>,
        0x03 using_message1: DbStr<'a>,
        0x04 using_message2: DbStr<'a>,
        0x07 failure_message: i32 = 0,
        0x08 r#type: SkillType = SkillType(0), persist,
        0x09 sp_type: SkillSpType = SkillSpType(0), is2k3,
        0x0A sp_percent: i32 = 0, is2k3,
        0x0B sp_cost: i32 = 0,
        0x0C scope: SkillScope = SkillScope(0), persist,
        0x0D switch_id: i32 = 1,
        0x0E animation_id: i32 = 1,
        0x10 sound_effect: Sound<'a>, persist,
        0x12 occasion_field: bool = true,
        0x13 occasion_battle: bool = false,
        0x14 reverse_state_effect: bool = false, is2k3,
        0x15 physical_rate: i32 = 0,
        0x16 magical_rate: i32 = 3,
        0x17 variance: i32 = 4,
        0x18 power: i32 = 0,
        0x19 hit: i32 = 100,
        0x1F affect_hp: bool = false,
        0x20 affect_sp: bool = false,
        0x21 affect_attack: bool = false,
        0x22 affect_defense: bool = false,
        0x23 affect_spirit: bool = false,
        0x24 affect_agility: bool = false,
        0x25 absorb_damage: bool = false,
        0x26 ignore_defense: bool = false,
        0x2A state_effects: BitArray<'a>, size = 0x29, persist,
        0x2C attribute_effects: BitArray<'a>, size = 0x2B, persist,
        0x2D affect_attr_defence: bool = false,
        0x31 battler_animation: i32 = -1, is2k3,
        0x32 battler_animation_data: List<BattlerAnimationItemSkill>, persist, is2k3,
        0xC9 easyrpg_battle2k3_message: DbStr<'a> = DbStr::from_bytes(b"default_message"), is2k3,
        0xCA easyrpg_ignore_reflect: bool = false, is2k3,
        0xCB easyrpg_state_hit: i32 = -1,
        0xCC easyrpg_attribute_hit: i32 = -1,
        0xCD easyrpg_ignore_restrict_skill: bool = false,
        0xCE easyrpg_ignore_restrict_magic: bool = false,
        0xCF easyrpg_enable_stat_absorbing: bool = false,
        0xD0 easyrpg_affected_by_evade_all_physical_attacks: bool = false, is2k3,
        0xD1 easyrpg_critical_hit_chance: i32 = 0,
        0xD2 easyrpg_affected_by_row_modifiers: bool = false, is2k3,
        0xD3 easyrpg_hp_type: SkillSpType = SkillSpType(0), is2k3,
        0xD4 easyrpg_hp_percent: i32 = 0, is2k3,
        0xD5 easyrpg_hp_cost: i32 = 0,
    }
}

lcf_strings!(Skill<'a> { name, description, using_message1, using_message2, easyrpg_battle2k3_message });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Item<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 description: DbStr<'a>,
        0x03 r#type: ItemType = ItemType(0), persist,
        0x05 price: i32 = 0,
        0x06 uses: i32 = 1,
        0x0B atk_points1: i32 = 0,
        0x0C def_points1: i32 = 0,
        0x0D spi_points1: i32 = 0,
        0x0E agi_points1: i32 = 0,
        0x0F two_handed: bool = false, persist,
        0x10 sp_cost: i32 = 0,
        0x11 hit: i32 = 90,
        0x12 critical_hit: i32 = 0,
        0x14 animation_id: i32 = 1,
        0x15 preemptive: bool = false,
        0x16 dual_attack: bool = false,
        0x17 attack_all: bool = false,
        0x18 ignore_evasion: bool = false,
        0x19 prevent_critical: bool = false,
        0x1A raise_evasion: bool = false,
        0x1B half_sp_cost: bool = false,
        0x1C no_terrain_damage: bool = false,
        0x1D cursed: bool = false, is2k3,
        0x1F entire_party: bool = false, persist,
        0x20 recover_hp_rate: i32 = 0,
        0x21 recover_hp: i32 = 0,
        0x22 recover_sp_rate: i32 = 0,
        0x23 recover_sp: i32 = 0,
        0x25 occasion_field1: bool = false,
        0x26 ko_only: bool = false,
        0x29 max_hp_points: i32 = 0,
        0x2A max_sp_points: i32 = 0,
        0x2B atk_points2: i32 = 0,
        0x2C def_points2: i32 = 0,
        0x2D spi_points2: i32 = 0,
        0x2E agi_points2: i32 = 0,
        0x33 using_message: i32 = 0, persist,
        0x35 skill_id: i32 = 1,
        0x37 switch_id: i32 = 1,
        0x39 occasion_field2: bool = true,
        0x3A occasion_battle: bool = false,
        0x3E actor_set: BitArray<'a>, size = 0x3D, persist,
        0x40 state_set: BitArray<'a>, size = 0x3F, persist,
        0x42 attribute_set: BitArray<'a>, size = 0x41, persist,
        0x43 state_chance: i32 = 0,
        0x44 reverse_state_effect: bool = false,
        0x45 weapon_animation: i32 = -1, is2k3,
        0x46 animation_data: List<BattlerAnimationItemSkill>, persist, is2k3,
        0x47 use_skill: bool = false, is2k3,
        0x49 class_set: BitArray<'a>, size = 0x48, persist, is2k3,
        0x4B ranged_trajectory: ItemTrajectory = ItemTrajectory(0),
        0x4C ranged_target: ItemTarget = ItemTarget(0),
        0xC9 easyrpg_using_message: DbStr<'a> = DbStr::from_bytes(b"default_message"),
        0xCA easyrpg_max_count: i32 = -1,
    }
}

lcf_strings!(Item<'a> { name, description, easyrpg_using_message });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EnemyAction indexed {
        0x01 kind: EnemyActionKind = EnemyActionKind(0), persist,
        0x02 basic: EnemyActionBasic = EnemyActionBasic(1), persist,
        0x03 skill_id: i32 = 1,
        0x04 enemy_id: i32 = 1,
        0x05 condition_type: EnemyActionConditionType = EnemyActionConditionType(0), persist,
        0x06 condition_param1: i32 = 0,
        0x07 condition_param2: i32 = 0,
        0x08 switch_id: i32 = 1,
        0x09 switch_on: bool = false,
        0x0A switch_on_id: i32 = 1,
        0x0B switch_off: bool = false,
        0x0C switch_off_id: i32 = 1,
        0x0D rating: i32 = 50,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Enemy<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 battler_name: DbStr<'a>,
        0x03 battler_hue: i32 = 0,
        0x04 max_hp: i32 = 10,
        0x05 max_sp: i32 = 10,
        0x06 attack: i32 = 10,
        0x07 defense: i32 = 10,
        0x08 spirit: i32 = 10,
        0x09 agility: i32 = 10,
        0x0A transparent: bool = false,
        0x0B exp: i32 = 0,
        0x0C gold: i32 = 0,
        0x0D drop_id: i32 = 0,
        0x0E drop_prob: i32 = 100,
        0x15 critical_hit: bool = false,
        0x16 critical_hit_chance: i32 = 30,
        0x1A miss: bool = false,
        0x1C levitate: bool = false,
        0x20 state_ranks: Packed<u8>, size = 0x1F, persist,
        0x22 attribute_ranks: Packed<u8>, size = 0x21, persist,
        0x2A actions: List<EnemyAction>, persist,
        0x0F maniac_unarmed_animation: i32 = 1,
        0xC9 easyrpg_enemyai: i32 = -1,
        0xCA easyrpg_prevent_critical: bool = false,
        0xCB easyrpg_raise_evasion: bool = false,
        0xCC easyrpg_immune_to_attribute_downshifts: bool = false,
        0xCD easyrpg_ignore_evasion: bool = false,
        0xCE easyrpg_hit: i32 = -1,
        0xD0 easyrpg_state_set: BitArray<'a>, size = 0xCF,
        0xD1 easyrpg_state_chance: i32 = 0,
        0xD3 easyrpg_attribute_set: BitArray<'a>, size = 0xD2,
        0xD4 easyrpg_super_guard: bool = false,
        0xD5 easyrpg_attack_all: bool = false,
    }
}

lcf_strings!(Enemy<'a> { name, battler_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TroopMember indexed {
        0x01 enemy_id: i32 = 1,
        0x02 x: i32 = 0,
        0x03 y: i32 = 0,
        0x04 invisible: bool = false,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TroopPageCondition {
        0x01 flags: TroopPageConditionFlags, persist,
        0x02 switch_a_id: i32 = 1,
        0x03 switch_b_id: i32 = 1,
        0x04 variable_id: i32 = 1,
        0x05 variable_value: i32 = 0,
        0x06 turn_a: i32 = 0,
        0x07 turn_b: i32 = 0,
        0x08 fatigue_min: i32 = 0,
        0x09 fatigue_max: i32 = 100,
        0x0A enemy_id: i32 = 0,
        0x0B enemy_hp_min: i32 = 0,
        0x0C enemy_hp_max: i32 = 100,
        0x0D actor_id: i32 = 1,
        0x0E actor_hp_min: i32 = 0,
        0x0F actor_hp_max: i32 = 100,
        0x10 turn_enemy_id: i32 = 0, is2k3,
        0x11 turn_enemy_a: i32 = 0, is2k3,
        0x12 turn_enemy_b: i32 = 0, is2k3,
        0x13 turn_actor_id: i32 = 1, is2k3,
        0x14 turn_actor_a: i32 = 0, is2k3,
        0x15 turn_actor_b: i32 = 0, is2k3,
        0x16 command_actor_id: i32 = 1, is2k3,
        0x17 command_id: i32 = 1, is2k3,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct TroopPage<'a> indexed {
        0x02 condition: TroopPageCondition, persist,
        0x0C event_commands: EventCommandList<'a>, size_persist = 0x0B, persist,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Troop<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 members: List<TroopMember>, persist,
        0x03 auto_alignment: bool = false, is2k3,
        0x05 terrain_set: BitArray<'a>, size = 0x04, persist,
        0x06 appear_randomly: bool = false, is2k3,
        0x0B pages: List<TroopPage<'a>>, persist,
    }
}

lcf_strings!(Troop<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Terrain<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 damage: i32 = 0,
        0x03 encounter_rate: i32 = 100,
        0x04 background_name: DbStr<'a>,
        0x05 boat_pass: bool = false,
        0x06 ship_pass: bool = false,
        0x07 airship_pass: bool = true,
        0x09 airship_land: bool = true,
        0x0B bush_depth: TerrainBushDepth = TerrainBushDepth(0), persist,
        0x0F footstep: Sound<'a>, persist, is2k3,
        0x10 on_damage_se: bool = false, is2k3,
        0x11 background_type: TerrainBGAssociation = TerrainBGAssociation(0), is2k3,
        0x15 background_a_name: DbStr<'a>, is2k3,
        0x16 background_a_scrollh: bool = false, is2k3,
        0x17 background_a_scrollv: bool = false, is2k3,
        0x18 background_a_scrollh_speed: i32 = 0, is2k3,
        0x19 background_a_scrollv_speed: i32 = 0, is2k3,
        0x1E background_b: bool = false, is2k3,
        0x1F background_b_name: DbStr<'a>, is2k3,
        0x20 background_b_scrollh: bool = false, is2k3,
        0x21 background_b_scrollv: bool = false, is2k3,
        0x22 background_b_scrollh_speed: i32 = 0, is2k3,
        0x23 background_b_scrollv_speed: i32 = 0, is2k3,
        0x28 special_flags: TerrainFlags, is2k3,
        0x29 special_back_party: i32 = 15, is2k3,
        0x2A special_back_enemies: i32 = 10, is2k3,
        0x2B special_lateral_party: i32 = 10, is2k3,
        0x2C special_lateral_enemies: i32 = 5, is2k3,
        0x2D grid_location: i32 = 0, is2k3,
        0x2E grid_top_y: i32 = 120, is2k3,
        0x2F grid_elongation: i32 = 392, is2k3,
        0x30 grid_inclination: i32 = 16000, is2k3,
        0xC8 easyrpg_damage_in_percent: bool = false,
        0xC9 easyrpg_damage_can_kill: bool = false,
    }
}

lcf_strings!(Terrain<'a> { name, background_name, background_a_name, background_b_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct State<'a> indexed {
        0x01 name: DbStr<'a>,
        0x02 r#type: StatePersistence = StatePersistence(0), persist,
        0x03 color: i32 = 6,
        0x04 priority: i32 = 50,
        0x05 restriction: StateRestriction = StateRestriction(0), persist,
        0x0B a_rate: i32 = 100,
        0x0C b_rate: i32 = 80,
        0x0D c_rate: i32 = 60,
        0x0E d_rate: i32 = 30,
        0x0F e_rate: i32 = 0,
        0x15 hold_turn: i32 = 0,
        0x16 auto_release_prob: i32 = 0,
        0x17 release_by_damage: i32 = 0,
        0x1E affect_type: StateAffectType = StateAffectType(0), is2k3,
        0x1F affect_attack: bool = false,
        0x20 affect_defense: bool = false,
        0x21 affect_spirit: bool = false,
        0x22 affect_agility: bool = false,
        0x23 reduce_hit_ratio: i32 = 100,
        0x24 avoid_attacks: bool = false, is2k3,
        0x25 reflect_magic: bool = false, is2k3,
        0x26 cursed: bool = false, is2k3,
        0x27 battler_animation_id: i32 = 100, is2k3,
        0x29 restrict_skill: bool = false,
        0x2A restrict_skill_level: i32 = 0,
        0x2B restrict_magic: bool = false,
        0x2C restrict_magic_level: i32 = 0,
        0x2D hp_change_type: StateChangeType = StateChangeType(0),
        0x2E sp_change_type: StateChangeType = StateChangeType(0),
        0x33 message_actor: DbStr<'a>,
        0x34 message_enemy: DbStr<'a>,
        0x35 message_already: DbStr<'a>,
        0x36 message_affected: DbStr<'a>,
        0x37 message_recovery: DbStr<'a>,
        0x3D hp_change_max: i32 = 0,
        0x3E hp_change_val: i32 = 0,
        0x3F hp_change_map_steps: i32 = 0,
        0x40 hp_change_map_val: i32 = 0,
        0x41 sp_change_max: i32 = 0,
        0x42 sp_change_val: i32 = 0,
        0x43 sp_change_map_steps: i32 = 0,
        0x44 sp_change_map_val: i32 = 0,
        0xC9 easyrpg_immune_states: BitArray<'a>, size = 0xC8,
    }
}

lcf_strings!(State<'a> { name, message_actor, message_enemy, message_already, message_affected, message_recovery });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Terms<'a> {
        0x01 encounter: DbStr<'a>, persist2k,
        0x02 special_combat: DbStr<'a>, persist,
        0x03 escape_success: DbStr<'a>, persist2k,
        0x04 escape_failure: DbStr<'a>, persist,
        0x05 victory: DbStr<'a>, persist,
        0x06 defeat: DbStr<'a>, persist,
        0x07 exp_received: DbStr<'a>, persist,
        0x08 gold_recieved_a: DbStr<'a>, persist,
        0x09 gold_recieved_b: DbStr<'a>, persist,
        0x0A item_recieved: DbStr<'a>, persist,
        0x0B attacking: DbStr<'a>, persist,
        0x0C enemy_critical: DbStr<'a>, persist,
        0x0D actor_critical: DbStr<'a>, persist,
        0x0E defending: DbStr<'a>, persist,
        0x0F observing: DbStr<'a>, persist,
        0x10 focus: DbStr<'a>, persist,
        0x11 autodestruction: DbStr<'a>, persist,
        0x12 enemy_escape: DbStr<'a>, persist,
        0x13 enemy_transform: DbStr<'a>, persist,
        0x14 enemy_damaged: DbStr<'a>, persist,
        0x15 enemy_undamaged: DbStr<'a>, persist,
        0x16 actor_damaged: DbStr<'a>, persist,
        0x17 actor_undamaged: DbStr<'a>, persist,
        0x18 skill_failure_a: DbStr<'a>, persist,
        0x19 skill_failure_b: DbStr<'a>, persist,
        0x1A skill_failure_c: DbStr<'a>, persist,
        0x1B dodge: DbStr<'a>, persist,
        0x1C use_item: DbStr<'a>, persist,
        0x1D hp_recovery: DbStr<'a>, persist,
        0x1E parameter_increase: DbStr<'a>, persist,
        0x1F parameter_decrease: DbStr<'a>, persist,
        0x20 enemy_hp_absorbed: DbStr<'a>, persist,
        0x21 actor_hp_absorbed: DbStr<'a>, persist,
        0x22 resistance_increase: DbStr<'a>, persist,
        0x23 resistance_decrease: DbStr<'a>, persist,
        0x24 level_up: DbStr<'a>, persist,
        0x25 skill_learned: DbStr<'a>, persist,
        0x26 battle_start: DbStr<'a>, persist, is2k3,
        0x27 miss: DbStr<'a>, persist, is2k3,
        0x29 shop_greeting1: DbStr<'a>, persist,
        0x2A shop_regreeting1: DbStr<'a>, persist,
        0x2B shop_buy1: DbStr<'a>, persist,
        0x2C shop_sell1: DbStr<'a>, persist,
        0x2D shop_leave1: DbStr<'a>, persist,
        0x2E shop_buy_select1: DbStr<'a>, persist,
        0x2F shop_buy_number1: DbStr<'a>, persist,
        0x30 shop_purchased1: DbStr<'a>, persist,
        0x31 shop_sell_select1: DbStr<'a>, persist,
        0x32 shop_sell_number1: DbStr<'a>, persist,
        0x33 shop_sold1: DbStr<'a>, persist,
        0x36 shop_greeting2: DbStr<'a>, persist,
        0x37 shop_regreeting2: DbStr<'a>, persist,
        0x38 shop_buy2: DbStr<'a>, persist,
        0x39 shop_sell2: DbStr<'a>, persist,
        0x3A shop_leave2: DbStr<'a>, persist,
        0x3B shop_buy_select2: DbStr<'a>, persist,
        0x3C shop_buy_number2: DbStr<'a>, persist,
        0x3D shop_purchased2: DbStr<'a>, persist,
        0x3E shop_sell_select2: DbStr<'a>, persist,
        0x3F shop_sell_number2: DbStr<'a>, persist,
        0x40 shop_sold2: DbStr<'a>, persist,
        0x43 shop_greeting3: DbStr<'a>, persist,
        0x44 shop_regreeting3: DbStr<'a>, persist,
        0x45 shop_buy3: DbStr<'a>, persist,
        0x46 shop_sell3: DbStr<'a>, persist,
        0x47 shop_leave3: DbStr<'a>, persist,
        0x48 shop_buy_select3: DbStr<'a>, persist,
        0x49 shop_buy_number3: DbStr<'a>, persist,
        0x4A shop_purchased3: DbStr<'a>, persist,
        0x4B shop_sell_select3: DbStr<'a>, persist,
        0x4C shop_sell_number3: DbStr<'a>, persist,
        0x4D shop_sold3: DbStr<'a>, persist,
        0x50 inn_a_greeting_1: DbStr<'a>, persist,
        0x51 inn_a_greeting_2: DbStr<'a>, persist,
        0x52 inn_a_greeting_3: DbStr<'a>, persist,
        0x53 inn_a_accept: DbStr<'a>, persist,
        0x54 inn_a_cancel: DbStr<'a>, persist,
        0x55 inn_b_greeting_1: DbStr<'a>, persist,
        0x56 inn_b_greeting_2: DbStr<'a>, persist,
        0x57 inn_b_greeting_3: DbStr<'a>, persist,
        0x58 inn_b_accept: DbStr<'a>, persist,
        0x59 inn_b_cancel: DbStr<'a>, persist,
        0x5C possessed_items: DbStr<'a>, persist,
        0x5D equipped_items: DbStr<'a>, persist,
        0x5F gold: DbStr<'a>, persist,
        0x65 battle_fight: DbStr<'a>, persist,
        0x66 battle_auto: DbStr<'a>, persist,
        0x67 battle_escape: DbStr<'a>, persist,
        0x68 command_attack: DbStr<'a>, persist,
        0x69 command_defend: DbStr<'a>, persist,
        0x6A command_item: DbStr<'a>, persist,
        0x6B command_skill: DbStr<'a>, persist,
        0x6C menu_equipment: DbStr<'a>, persist,
        0x6E menu_save: DbStr<'a>, persist,
        0x70 menu_quit: DbStr<'a>, persist,
        0x72 new_game: DbStr<'a>, persist,
        0x73 load_game: DbStr<'a>, persist,
        0x75 exit_game: DbStr<'a>, persist,
        0x76 status: DbStr<'a>, persist, is2k3,
        0x77 row: DbStr<'a>, persist, is2k3,
        0x78 order: DbStr<'a>, persist, is2k3,
        0x79 wait_on: DbStr<'a>, persist, is2k3,
        0x7A wait_off: DbStr<'a>, persist, is2k3,
        0x7B level: DbStr<'a>, persist,
        0x7C health_points: DbStr<'a>, persist,
        0x7D spirit_points: DbStr<'a>, persist,
        0x7E normal_status: DbStr<'a>, persist,
        0x7F exp_short: DbStr<'a>, persist,
        0x80 lvl_short: DbStr<'a>, persist,
        0x81 hp_short: DbStr<'a>, persist,
        0x82 sp_short: DbStr<'a>, persist,
        0x83 sp_cost: DbStr<'a>, persist,
        0x84 attack: DbStr<'a>, persist,
        0x85 defense: DbStr<'a>, persist,
        0x86 spirit: DbStr<'a>, persist,
        0x87 agility: DbStr<'a>, persist,
        0x88 weapon: DbStr<'a>, persist,
        0x89 shield: DbStr<'a>, persist,
        0x8A armor: DbStr<'a>, persist,
        0x8B helmet: DbStr<'a>, persist,
        0x8C accessory: DbStr<'a>, persist,
        0x92 save_game_message: DbStr<'a>, persist,
        0x93 load_game_message: DbStr<'a>, persist,
        0x94 file: DbStr<'a>, persist,
        0x97 exit_game_message: DbStr<'a>, persist,
        0x98 yes: DbStr<'a>, persist,
        0x99 no: DbStr<'a>, persist,
        0xA1 maniac_item_received_a: DbStr<'a>,
        0xA2 maniac_level_up_a: DbStr<'a>,
        0xA3 maniac_level_up_b: DbStr<'a>,
        0xA4 maniac_level_up_c: DbStr<'a>,
        0xA5 maniac_exp_received_a: DbStr<'a>,
        0xA6 maniac_skill_learned_a: DbStr<'a>,
        0xC8 easyrpg_item_number_separator: DbStr<'a> = DbStr::from_bytes(b"default_term"),
        0xC9 easyrpg_skill_cost_separator: DbStr<'a> = DbStr::from_bytes(b"default_term"),
        0xCA easyrpg_equipment_arrow: DbStr<'a> = DbStr::from_bytes(b"default_term"),
        0xCB easyrpg_status_scene_name: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xCC easyrpg_status_scene_class: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xCD easyrpg_status_scene_title: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xCE easyrpg_status_scene_condition: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xCF easyrpg_status_scene_front: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD0 easyrpg_status_scene_back: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD1 easyrpg_order_scene_confirm: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD2 easyrpg_order_scene_redo: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD3 easyrpg_battle2k3_double_attack: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD4 easyrpg_battle2k3_defend: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD5 easyrpg_battle2k3_observe: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD6 easyrpg_battle2k3_charge: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD7 easyrpg_battle2k3_selfdestruct: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD8 easyrpg_battle2k3_escape: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xD9 easyrpg_battle2k3_special_combat_back: DbStr<'a>, is2k3,
        0xDA easyrpg_battle2k3_skill: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
        0xDB easyrpg_battle2k3_item: DbStr<'a> = DbStr::from_bytes(b"default_term"), is2k3,
    }
}

lcf_strings!(Terms<'a> { encounter, special_combat, escape_success, escape_failure, victory, defeat, exp_received, gold_recieved_a, gold_recieved_b, item_recieved, attacking, enemy_critical, actor_critical, defending, observing, focus, autodestruction, enemy_escape, enemy_transform, enemy_damaged, enemy_undamaged, actor_damaged, actor_undamaged, skill_failure_a, skill_failure_b, skill_failure_c, dodge, use_item, hp_recovery, parameter_increase, parameter_decrease, enemy_hp_absorbed, actor_hp_absorbed, resistance_increase, resistance_decrease, level_up, skill_learned, battle_start, miss, shop_greeting1, shop_regreeting1, shop_buy1, shop_sell1, shop_leave1, shop_buy_select1, shop_buy_number1, shop_purchased1, shop_sell_select1, shop_sell_number1, shop_sold1, shop_greeting2, shop_regreeting2, shop_buy2, shop_sell2, shop_leave2, shop_buy_select2, shop_buy_number2, shop_purchased2, shop_sell_select2, shop_sell_number2, shop_sold2, shop_greeting3, shop_regreeting3, shop_buy3, shop_sell3, shop_leave3, shop_buy_select3, shop_buy_number3, shop_purchased3, shop_sell_select3, shop_sell_number3, shop_sold3, inn_a_greeting_1, inn_a_greeting_2, inn_a_greeting_3, inn_a_accept, inn_a_cancel, inn_b_greeting_1, inn_b_greeting_2, inn_b_greeting_3, inn_b_accept, inn_b_cancel, possessed_items, equipped_items, gold, battle_fight, battle_auto, battle_escape, command_attack, command_defend, command_item, command_skill, menu_equipment, menu_save, menu_quit, new_game, load_game, exit_game, status, row, order, wait_on, wait_off, level, health_points, spirit_points, normal_status, exp_short, lvl_short, hp_short, sp_short, sp_cost, attack, defense, spirit, agility, weapon, shield, armor, helmet, accessory, save_game_message, load_game_message, file, exit_game_message, yes, no, maniac_item_received_a, maniac_level_up_a, maniac_level_up_b, maniac_level_up_c, maniac_exp_received_a, maniac_skill_learned_a, easyrpg_item_number_separator, easyrpg_skill_cost_separator, easyrpg_equipment_arrow, easyrpg_status_scene_name, easyrpg_status_scene_class, easyrpg_status_scene_title, easyrpg_status_scene_condition, easyrpg_status_scene_front, easyrpg_status_scene_back, easyrpg_order_scene_confirm, easyrpg_order_scene_redo, easyrpg_battle2k3_double_attack, easyrpg_battle2k3_defend, easyrpg_battle2k3_observe, easyrpg_battle2k3_charge, easyrpg_battle2k3_selfdestruct, easyrpg_battle2k3_escape, easyrpg_battle2k3_special_combat_back, easyrpg_battle2k3_skill, easyrpg_battle2k3_item });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Music<'a> {
        0x01 name: DbStr<'a> = DbStr::from_bytes(b"(OFF)"), persist,
        0x02 fadein: i32 = 0,
        0x03 volume: i32 = 100,
        0x04 tempo: i32 = 100,
        0x05 balance: i32 = 50,
    }
}

lcf_strings!(Music<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TestBattler indexed {
        0x01 actor_id: i32 = 1,
        0x02 level: i32 = 1,
        0x0B weapon_id: i32 = 0,
        0x0C shield_id: i32 = 0,
        0x0D armor_id: i32 = 0,
        0x0E helmet_id: i32 = 0,
        0x0F accessory_id: i32 = 0,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct System<'a> {
        0x0A ldb_id: i32 = 0,
        0x0B boat_name: DbStr<'a>,
        0x0C ship_name: DbStr<'a>,
        0x0D airship_name: DbStr<'a>,
        0x0E boat_index: i32 = 0,
        0x0F ship_index: i32 = 0,
        0x10 airship_index: i32 = 0,
        0x11 title_name: DbStr<'a>,
        0x12 gameover_name: DbStr<'a>,
        0x13 system_name: DbStr<'a>,
        0x14 system2_name: DbStr<'a>, is2k3,
        0x16 party: Packed<i16> = Packed(alloc::vec![1]), count = 0x15, persist,
        0x1B menu_commands: Packed<i16> = Packed(alloc::vec![1]), count = 0x1A, persist, is2k3,
        0x1F title_music: Music<'a>, persist,
        0x20 battle_music: Music<'a>, persist,
        0x21 battle_end_music: Music<'a>, persist,
        0x22 inn_music: Music<'a>, persist,
        0x23 boat_music: Music<'a>, persist,
        0x24 ship_music: Music<'a>, persist,
        0x25 airship_music: Music<'a>, persist,
        0x26 gameover_music: Music<'a>, persist,
        0x29 cursor_se: Sound<'a>, persist,
        0x2A decision_se: Sound<'a>, persist,
        0x2B cancel_se: Sound<'a>, persist,
        0x2C buzzer_se: Sound<'a>, persist,
        0x2D battle_se: Sound<'a>, persist,
        0x2E escape_se: Sound<'a>, persist,
        0x2F enemy_attack_se: Sound<'a>, persist,
        0x30 enemy_damaged_se: Sound<'a>, persist,
        0x31 actor_damaged_se: Sound<'a>, persist,
        0x32 dodge_se: Sound<'a>, persist,
        0x33 enemy_death_se: Sound<'a>, persist,
        0x34 item_se: Sound<'a>, persist,
        0x3D transition_out: SystemFadeOut = SystemFadeOut(0), persist,
        0x3E transition_in: SystemFadeIn = SystemFadeIn(0), persist,
        0x3F battle_start_fadeout: SystemFadeOut = SystemFadeOut(0), persist,
        0x40 battle_start_fadein: SystemFadeIn = SystemFadeIn(0), persist,
        0x41 battle_end_fadeout: SystemFadeOut = SystemFadeOut(0), persist,
        0x42 battle_end_fadein: SystemFadeIn = SystemFadeIn(0), persist,
        0x47 message_stretch: SystemStretch = SystemStretch(0),
        0x48 font_id: SystemFont = SystemFont(0),
        0x51 selected_condition: i32 = 0,
        0x52 selected_hero: i32 = 0, persist,
        0x54 battletest_background: DbStr<'a>,
        0x55 battletest_data: List<TestBattler>, persist,
        0x5B save_count: i32 = 0,
        0x5E battletest_terrain: i32 = 0,
        0x5F battletest_formation: SystemBattleFormation = SystemBattleFormation(0),
        0x60 battletest_condition: SystemBattleCondition = SystemBattleCondition(0),
        0x61 equipment_setting: SystemEquipmentSetting = SystemEquipmentSetting(0), is2k3,
        0x62 battletest_alt_terrain: i32 = -1, is2k3,
        0x63 show_frame: bool = false, is2k3,
        0x64 frame_name: DbStr<'a>, is2k3,
        0x65 invert_animations: bool = false, is2k3,
        0x6F show_title: bool = true, is2k3,
        0xC8 easyrpg_alternative_exp: i32 = 0,
        0xC9 easyrpg_battle_options: Packed<i16> = Packed(alloc::vec![0, 1, 2]),
        0xCA easyrpg_max_actor_hp: i32 = -1,
        0xCB easyrpg_max_enemy_hp: i32 = -1,
        0xCC easyrpg_max_damage: i32 = -1,
        0xCD easyrpg_max_exp: i32 = -1,
        0xCE easyrpg_max_level: i32 = -1,
        0xCF easyrpg_max_savefiles: i32 = 15,
        0xD0 easyrpg_max_item_count: i32 = -1,
        0xD1 easyrpg_variable_min_value: i32 = 0,
        0xD2 easyrpg_variable_max_value: i32 = 0,
        0xD3 easyrpg_max_actor_sp: i32 = -1,
        0xD4 easyrpg_max_enemy_sp: i32 = -1,
        0xD5 easyrpg_max_stat_base_value: i32 = -1,
        0xD6 easyrpg_max_stat_battle_value: i32 = -1,
        0xD7 easyrpg_use_r2k_battle_system: bool = false, is2k3,
        0xD8 easyrpg_battle_use_r2ke_strings: bool = false, is2k3,
        0xD9 easyrpg_use_r2k_battle_commands: bool = false, is2k3,
        0xDA easyrpg_default_actorai: i32 = -1,
        0xDB easyrpg_default_enemyai: i32 = -1,
    }
}

lcf_strings!(System<'a> { boat_name, ship_name, airship_name, title_name, gameover_name, system_name, system2_name, battletest_background, frame_name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Switch<'a> indexed {
        0x01 name: DbStr<'a>,
    }
}

lcf_strings!(Switch<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Variable<'a> indexed {
        0x01 name: DbStr<'a>,
    }
}

lcf_strings!(Variable<'a> { name });

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct Database<'a> {
        0x0B actors: List<Actor<'a>>, persist,
        0x0C skills: List<Skill<'a>>, persist,
        0x0D items: List<Item<'a>>, persist,
        0x0E enemies: List<Enemy<'a>>, persist,
        0x0F troops: List<Troop<'a>>, persist,
        0x10 terrains: List<Terrain<'a>>, persist,
        0x11 attributes: List<Attribute<'a>>, persist,
        0x12 states: List<State<'a>>, persist,
        0x13 animations: List<Animation<'a>>, persist,
        0x14 chipsets: List<Chipset<'a>>, persist,
        0x15 terms: Terms<'a>, persist,
        0x16 system: System<'a>, persist,
        0x17 switches: List<Switch<'a>>, persist,
        0x18 variables: List<Variable<'a>>, persist,
        0x19 commonevents: List<CommonEvent<'a>>, persist,
        0x1A version: DatabaseVersion, db_version,
        0x1B commonevent_d2: EmptyBlock, persist, is2k3,
        0x1C commonevent_d3: EmptyBlock, persist, is2k3,
        0x1D battlecommands: BattleCommands<'a>, persist, is2k3,
        0x1E classes: List<Class<'a>>, persist, is2k3,
        0x1F class_d1: EmptyBlock, persist, is2k3,
        0x20 battleranimations: List<BattlerAnimation<'a>>, persist, is2k3,
        0x21 maniac_string_variables: List<StringVariable<'a>>,
    }
}

lcf_struct! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub struct StringVariable<'a> indexed {
        0x01 name: DbStr<'a>,
    }
}

lcf_strings!(StringVariable<'a> { name });
