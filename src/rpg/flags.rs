//! The schema's bit-packed boolean bundles.

use crate::lcf_flags;

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TroopPageConditionFlags {
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

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TerrainFlags {
        back_party is2k3,
        back_enemies is2k3,
        lateral_party is2k3,
        lateral_enemies is2k3,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageConditionFlags {
        switch_a,
        switch_b,
        variable,
        item,
        actor,
        timer,
        timer2 is2k3,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureFlags {
        erase_on_map_change is2k3,
        erase_on_battle_end is2k3,
        unused_bit is2k3,
        unused_bit2 is2k3,
        affected_by_tint is2k3,
        affected_by_flash is2k3,
        affected_by_shake is2k3,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveEasyRpgWindowFlags {
        draw_frame,
        border_margin,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveEasyRpgTextFlags {
        draw_gradient,
        draw_shadow,
        bold,
        italic,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EasyRpgFrameRuntimeFlags {
        reserved_1,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EasyRpgEventRuntimeFlags {
        reserved_1,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EasyRpgStateRuntimeFlags {
        conf_override_active,
        reserved_1,
        reserved_2,
        reserved_3,
        patch_destiny_on,
        patch_destiny_off,
        patch_dynrpg_on,
        patch_dynrpg_off,
        patch_maniac_on,
        patch_maniac_off,
        patch_common_this_event_on,
        patch_common_this_event_off,
        patch_unlock_pics_on,
        patch_unlock_pics_off,
        patch_keypatch_on,
        patch_keypatch_off,
        patch_r2k3_cmds_on,
        patch_r2k3_cmds_off,
        use_r2k_battle_system_on,
        use_r2k_battle_system_off,
    }
}

lcf_flags! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct ManiacMessageHookFlags {
        user_event,
        create_window,
        destroy_window,
        text_rendering,
    }
}
