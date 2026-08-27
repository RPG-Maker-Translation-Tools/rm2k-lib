//! Named constants for the schema's enumerated fields.

use crate::lcf_enum;

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationTimingFlashScope {
        NOTHING = 0,
        TARGET = 1,
        SCREEN = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationTimingScreenShake {
        NOTHING = 0,
        TARGET = 1,
        SCREEN = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationScope {
        TARGET = 0,
        SCREEN = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AnimationPosition {
        UP = 0,
        MIDDLE = 1,
        DOWN = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct AttributeType {
        PHYSICAL = 0,
        MAGICAL = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandType {
        ATTACK = 0,
        SKILL = 1,
        SUBSKILL = 2,
        DEFENSE = 3,
        ITEM = 4,
        ESCAPE = 5,
        SPECIAL = 6,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsPlacement {
        MANUAL = 0,
        AUTOMATIC = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsRowShown {
        FRONT = 0,
        BACK = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsBattleType {
        TRADITIONAL = 0,
        ALTERNATIVE = 1,
        GAUGE = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsWindowSize {
        LARGE = 0,
        SMALL = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsTransparency {
        OPAQUE = 0,
        TRANSPARENT = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattleCommandsFacing {
        RETAIN = 0,
        UP = 1,
        RIGHT = 2,
        DOWN = 3,
        LEFT = 4,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationSpeed {
        SLOW = 20,
        MEDIUM = 14,
        FAST = 8,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationPose {
        IDLE = 0,
        ATTACK_RIGHT = 1,
        ATTACK_LEFT = 2,
        SKILL = 3,
        DEAD = 4,
        DAMAGE = 5,
        DAZED = 6,
        DEFEND = 7,
        WALK_LEFT = 8,
        WALK_RIGHT = 9,
        VICTORY = 10,
        ITEM = 11,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationItemSkillSpeed {
        FAST = 0,
        MEDIUM = 1,
        SLOW = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationItemSkillAnimType {
        WEAPON = 0,
        BATTLE = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationItemSkillMovement {
        NONE = 0,
        STEP = 1,
        JUMP = 2,
        MOVE = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationItemSkillAfterimage {
        NONE = 0,
        ADD = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct BattlerAnimationPoseAnimType {
        CHARACTER = 0,
        BATTLE = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct ChipsetAnimType {
        RECIPROCATING = 0,
        CYCLIC = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct CommonEventTrigger {
        AUTOMATIC = 3,
        PARALLEL = 4,
        CALL = 5,
        MANIAC_BATTLE_START = 6,
        MANIAC_BATTLE_PARALLEL = 7,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SkillType {
        NORMAL = 0,
        TELEPORT = 1,
        ESCAPE = 2,
        SWITCH = 3,
        SUBSKILL = 4,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SkillSpType {
        COST = 0,
        PERCENT = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SkillScope {
        ENEMY = 0,
        ENEMIES = 1,
        SELF = 2,
        ALLY = 3,
        PARTY = 4,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct ItemType {
        NORMAL = 0,
        WEAPON = 1,
        SHIELD = 2,
        ARMOR = 3,
        HELMET = 4,
        ACCESSORY = 5,
        MEDICINE = 6,
        BOOK = 7,
        MATERIAL = 8,
        SPECIAL = 9,
        SWITCH = 10,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct ItemTrajectory {
        STRAIGHT = 0,
        RETURN = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct ItemTarget {
        SINGLE = 0,
        CENTER = 1,
        SIMULTANEOUS = 2,
        SEQUENTIAL = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EnemyActionKind {
        BASIC = 0,
        SKILL = 1,
        TRANSFORMATION = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EnemyActionBasic {
        ATTACK = 0,
        DUAL_ATTACK = 1,
        DEFENSE = 2,
        OBSERVE = 3,
        CHARGE = 4,
        AUTODESTRUCTION = 5,
        ESCAPE = 6,
        NOTHING = 7,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EnemyActionConditionType {
        ALWAYS = 0,
        SWITCH = 1,
        TURN = 2,
        ACTORS = 3,
        HP = 4,
        SP = 5,
        PARTY_LVL = 6,
        PARTY_FATIGUE = 7,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TerrainBushDepth {
        NORMAL = 0,
        THIRD = 1,
        HALF = 2,
        FULL = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TerrainBGAssociation {
        BACKGROUND = 0,
        FRAME = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveActorRowType {
        FRONT = 0,
        BACK = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePartyLocationVehicleType {
        NONE = 0,
        SKIFF = 1,
        SHIP = 2,
        AIRSHIP = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePartyLocationPanState {
        FIXED = 0,
        FOLLOW = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureEffect {
        NONE = 0,
        ROTATION = 1,
        WAVE = 2,
        MANIAC_FIXED_ANGLE = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureMapLayer {
        NONE = 0,
        PARALLAX = 1,
        TILEMAP_BELOW = 2,
        EVENTS_BELOW = 3,
        EVENTS_SAME_AS_PLAYER = 4,
        TILEMAP_ABOVE = 5,
        EVENTS_ABOVE = 6,
        WEATHER = 7,
        ANIMATIONS = 8,
        WINDOWS = 9,
        TIMERS = 10,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureBattleLayer {
        NONE = 0,
        BACKGROUND = 1,
        BATTLERS_AND_ANIMATIONS = 2,
        WEATHER = 3,
        WINDOWS_AND_STATUS = 4,
        TIMERS = 5,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveSystemScene {
        MAP = 0,
        MENU = 1,
        BATTLE = 2,
        SHOP = 3,
        NAME = 4,
        FILE = 5,
        TITLE = 6,
        GAME_OVER = 7,
        DEBUG = 8,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveSystemAtbMode {
        ATB_ACTIVE = 0,
        ATB_WAIT = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SaveVehicleLocationVehicleType {
        NONE = 0,
        SKIFF = 1,
        SHIP = 2,
        AIRSHIP = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct StatePersistence {
        ENDS = 0,
        PERSISTS = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct StateRestriction {
        NORMAL = 0,
        DO_NOTHING = 1,
        ATTACK_ENEMY = 2,
        ATTACK_ALLY = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct StateAffectType {
        HALF = 0,
        DOUBLE = 1,
        NOTHING = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct StateChangeType {
        LOSE = 0,
        GAIN = 1,
        NOTHING = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemFadeOut {
        DEFAULT = 0,
        FADE_OUT = 1,
        REMOVE_BLOCKS = 2,
        WIPE_DOWNWARD = 3,
        WIPE_UPWARD = 4,
        VENETIAN_BLINDS = 5,
        VERTICAL_BLINDS = 6,
        HORIZONTAL_BLINDS = 7,
        RECEDING_SQUARE = 8,
        EXPANDING_SQUARE = 9,
        SCREEN_MOVES_UP = 10,
        SCREEN_MOVES_DOWN = 11,
        SCREEN_MOVES_LEFT = 12,
        SCREEN_MOVES_RIGHT = 13,
        VERTICAL_DIV = 14,
        HORIZONTAL_DIV = 15,
        QUADRASECTION = 16,
        ZOOM_IN = 17,
        MOSAIC = 18,
        WAVER_SCREEN = 19,
        INSTANTANEOUS = 20,
        NONE = 21,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemFadeIn {
        DEFAULT = 0,
        FADE_IN = 1,
        RECONSTITUTE_BLOCKS = 2,
        UNWIPE_DOWNWARD = 3,
        UNWIPE_UPWARD = 4,
        VENETIAN_BLINDS = 5,
        VERTICAL_BLINDS = 6,
        HORIZONTAL_BLINDS = 7,
        RECEDING_SQUARE = 8,
        EXPANDING_SQUARE = 9,
        SCREEN_MOVES_DOWN = 10,
        SCREEN_MOVES_UP = 11,
        SCREEN_MOVES_RIGHT = 12,
        SCREEN_MOVES_LEFT = 13,
        VERTICAL_UNIFY = 14,
        HORIZONTAL_UNIFY = 15,
        UNIFY_QUADRANTS = 16,
        ZOOM_OUT = 17,
        MOSAIC = 18,
        WAVER_SCREEN = 19,
        INSTANTANEOUS = 20,
        NONE = 21,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemStretch {
        STRETCH = 0,
        TILED = 1,
        EASYRPG_NONE = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemFont {
        GOTHIC = 0,
        MINCHO = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemBattleFormation {
        TERRAIN = 0,
        LOOSE = 1,
        TIGHT = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemBattleCondition {
        NONE = 0,
        INITIATIVE = 1,
        BACK = 2,
        SURROUND = 3,
        PINCERS = 4,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SystemEquipmentSetting {
        ACTOR = 0,
        CLASS = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapInfoMusicType {
        PARENT = 0,
        EVENT = 1,
        SPECIFIC = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapInfoBGMType {
        PARENT = 0,
        TERRAIN = 1,
        SPECIFIC = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapInfoTriState {
        PARENT = 0,
        ALLOW = 1,
        FORBID = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageConditionComparison {
        EQUAL = 0,
        GREATER_EQUAL = 1,
        LESS_EQUAL = 2,
        GREATER = 3,
        LESS = 4,
        NOT_EQUAL = 5,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageDirection {
        UP = 0,
        RIGHT = 1,
        DOWN = 2,
        LEFT = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageFrame {
        LEFT = 0,
        MIDDLE = 1,
        RIGHT = 2,
        MIDDLE2 = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageMoveType {
        STATIONARY = 0,
        RANDOM = 1,
        VERTICAL = 2,
        HORIZONTAL = 3,
        TOWARD = 4,
        AWAY = 5,
        CUSTOM = 6,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageTrigger {
        ACTION = 0,
        TOUCHED = 1,
        COLLISION = 2,
        AUTO_START = 3,
        PARALLEL = 4,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageLayers {
        BELOW = 0,
        SAME = 1,
        ABOVE = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageAnimType {
        NON_CONTINUOUS = 0,
        CONTINUOUS = 1,
        FIXED_NON_CONTINUOUS = 2,
        FIXED_CONTINUOUS = 3,
        FIXED_GRAPHIC = 4,
        SPIN = 5,
        STEP_FRAME_FIX = 6,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageMoveSpeed {
        EIGHTH = 1,
        QUARTER = 2,
        HALF = 3,
        NORMAL = 4,
        DOUBLE = 5,
        FOURFOLD = 6,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapScrollType {
        NONE = 0,
        VERTICAL = 1,
        HORIZONTAL = 2,
        BOTH = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapGeneratorMode {
        SINGLE_PASSAGE = 0,
        LINKED_ROOMS = 1,
        MAZE_PASSAGE = 2,
        OPEN_ROOM = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MapGeneratorTiles {
        ONE = 0,
        TWO = 1,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct TreeMapMapType {
        ROOT = 0,
        MAP = 1,
        AREA = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct MoveCommandCode {
        MOVE_UP = 0,
        MOVE_RIGHT = 1,
        MOVE_DOWN = 2,
        MOVE_LEFT = 3,
        MOVE_UPRIGHT = 4,
        MOVE_DOWNRIGHT = 5,
        MOVE_DOWNLEFT = 6,
        MOVE_UPLEFT = 7,
        MOVE_RANDOM = 8,
        MOVE_TOWARDS_HERO = 9,
        MOVE_AWAY_FROM_HERO = 10,
        MOVE_FORWARD = 11,
        FACE_UP = 12,
        FACE_RIGHT = 13,
        FACE_DOWN = 14,
        FACE_LEFT = 15,
        TURN_90_DEGREE_RIGHT = 16,
        TURN_90_DEGREE_LEFT = 17,
        TURN_180_DEGREE = 18,
        TURN_90_DEGREE_RANDOM = 19,
        FACE_RANDOM_DIRECTION = 20,
        FACE_HERO = 21,
        FACE_AWAY_FROM_HERO = 22,
        WAIT = 23,
        BEGIN_JUMP = 24,
        END_JUMP = 25,
        LOCK_FACING = 26,
        UNLOCK_FACING = 27,
        INCREASE_MOVEMENT_SPEED = 28,
        DECREASE_MOVEMENT_SPEED = 29,
        INCREASE_MOVEMENT_FREQUENCE = 30,
        DECREASE_MOVEMENT_FREQUENCE = 31,
        SWITCH_ON = 32,
        SWITCH_OFF = 33,
        CHANGE_GRAPHIC = 34,
        PLAY_SOUND_EFFECT = 35,
        WALK_EVERYWHERE_ON = 36,
        WALK_EVERYWHERE_OFF = 37,
        STOP_ANIMATION = 38,
        START_ANIMATION = 39,
        INCREASE_TRANSP = 40,
        DECREASE_TRANSP = 41,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventCommandCode {
        END = 10,
        CALL_COMMON_EVENT = 1005,
        FORCE_FLEE = 1006,
        ENABLE_COMBO = 1007,
        CHANGE_CLASS = 1008,
        CHANGE_BATTLE_COMMANDS = 1009,
        OPEN_LOAD_MENU = 5001,
        EXIT_GAME = 5002,
        TOGGLE_ATB_MODE = 5003,
        TOGGLE_FULLSCREEN = 5004,
        OPEN_VIDEO_OPTIONS = 5005,
        SHOW_MESSAGE = 10110,
        MESSAGE_OPTIONS = 10120,
        CHANGE_FACE_GRAPHIC = 10130,
        SHOW_CHOICE = 10140,
        INPUT_NUMBER = 10150,
        CONTROL_SWITCHES = 10210,
        CONTROL_VARS = 10220,
        TIMER_OPERATION = 10230,
        CHANGE_GOLD = 10310,
        CHANGE_ITEMS = 10320,
        CHANGE_PARTY_MEMBERS = 10330,
        CHANGE_EXP = 10410,
        CHANGE_LEVEL = 10420,
        CHANGE_PARAMETERS = 10430,
        CHANGE_SKILLS = 10440,
        CHANGE_EQUIPMENT = 10450,
        CHANGE_HP = 10460,
        CHANGE_SP = 10470,
        CHANGE_CONDITION = 10480,
        FULL_HEAL = 10490,
        SIMULATED_ATTACK = 10500,
        CHANGE_HERO_NAME = 10610,
        CHANGE_HERO_TITLE = 10620,
        CHANGE_SPRITE_ASSOCIATION = 10630,
        CHANGE_ACTOR_FACE = 10640,
        CHANGE_VEHICLE_GRAPHIC = 10650,
        CHANGE_SYSTEM_BGM = 10660,
        CHANGE_SYSTEM_SFX = 10670,
        CHANGE_SYSTEM_GRAPHICS = 10680,
        CHANGE_SCREEN_TRANSITIONS = 10690,
        ENEMY_ENCOUNTER = 10710,
        OPEN_SHOP = 10720,
        SHOW_INN = 10730,
        ENTER_HERO_NAME = 10740,
        TELEPORT = 10810,
        MEMORIZE_LOCATION = 10820,
        RECALL_TO_LOCATION = 10830,
        ENTER_EXIT_VEHICLE = 10840,
        SET_VEHICLE_LOCATION = 10850,
        CHANGE_EVENT_LOCATION = 10860,
        TRADE_EVENT_LOCATIONS = 10870,
        STORE_TERRAIN_ID = 10910,
        STORE_EVENT_ID = 10920,
        ERASE_SCREEN = 11010,
        SHOW_SCREEN = 11020,
        TINT_SCREEN = 11030,
        FLASH_SCREEN = 11040,
        SHAKE_SCREEN = 11050,
        PAN_SCREEN = 11060,
        WEATHER_EFFECTS = 11070,
        SHOW_PICTURE = 11110,
        MOVE_PICTURE = 11120,
        ERASE_PICTURE = 11130,
        SHOW_BATTLE_ANIMATION = 11210,
        PLAYER_VISIBILITY = 11310,
        FLASH_SPRITE = 11320,
        MOVE_EVENT = 11330,
        PROCEED_WITH_MOVEMENT = 11340,
        HALT_ALL_MOVEMENT = 11350,
        WAIT = 11410,
        PLAY_BGM = 11510,
        FADE_OUT_BGM = 11520,
        MEMORIZE_BGM = 11530,
        PLAY_MEMORIZED_BGM = 11540,
        PLAY_SOUND = 11550,
        PLAY_MOVIE = 11560,
        KEY_INPUT_PROC = 11610,
        CHANGE_MAP_TILESET = 11710,
        CHANGE_PBG = 11720,
        CHANGE_ENCOUNTER_STEPS = 11740,
        TILE_SUBSTITUTION = 11750,
        TELEPORT_TARGETS = 11810,
        CHANGE_TELEPORT_ACCESS = 11820,
        ESCAPE_TARGET = 11830,
        CHANGE_ESCAPE_ACCESS = 11840,
        OPEN_SAVE_MENU = 11910,
        CHANGE_SAVE_ACCESS = 11930,
        OPEN_MAIN_MENU = 11950,
        CHANGE_MAIN_MENU_ACCESS = 11960,
        CONDITIONAL_BRANCH = 12010,
        LABEL = 12110,
        JUMP_TO_LABEL = 12120,
        LOOP = 12210,
        BREAK_LOOP = 12220,
        END_EVENT_PROCESSING = 12310,
        ERASE_EVENT = 12320,
        CALL_EVENT = 12330,
        COMMENT = 12410,
        GAME_OVER = 12420,
        RETURNTO_TITLE_SCREEN = 12510,
        CHANGE_MONSTER_HP = 13110,
        CHANGE_MONSTER_MP = 13120,
        CHANGE_MONSTER_CONDITION = 13130,
        SHOW_HIDDEN_MONSTER = 13150,
        CHANGE_BATTLE_BG = 13210,
        SHOW_BATTLE_ANIMATION_B = 13260,
        CONDITIONAL_BRANCH_B = 13310,
        TERMINATE_BATTLE = 13410,
        SHOW_MESSAGE_2 = 20110,
        SHOW_CHOICE_OPTION = 20140,
        SHOW_CHOICE_END = 20141,
        VICTORY_HANDLER = 20710,
        ESCAPE_HANDLER = 20711,
        DEFEAT_HANDLER = 20712,
        END_BATTLE = 20713,
        TRANSACTION = 20720,
        NO_TRANSACTION = 20721,
        END_SHOP = 20722,
        STAY = 20730,
        NO_STAY = 20731,
        END_INN = 20732,
        ELSE_BRANCH = 22010,
        END_BRANCH = 22011,
        END_LOOP = 22210,
        COMMENT_2 = 22410,
        ELSE_BRANCH_B = 23310,
        END_BRANCH_B = 23311,
        EASY_RPG_TRIGGER_EVENT_AT = 2002,
        EASY_RPG_PATHFINDER = 2003,
        EASY_RPG_CALL_MOVEMENT_ACTION = 2050,
        EASY_RPG_WAIT_FOR_SINGLE_MOVEMENT = 2051,
        EASY_RPG_ANIMATE_VARIABLE = 2052,
        EASY_RPG_SET_INTERPRETER_FLAG = 2053,
        EASY_RPG_PROCESS_JSON = 2055,
        EASY_RPG_CLONE_MAP_EVENT = 2056,
        EASY_RPG_DESTROY_MAP_EVENT = 2057,
        EASY_RPG_STRING_PICTURE_MENU = 2058,
        MANIAC_GET_SAVE_INFO = 3001,
        MANIAC_SAVE = 3002,
        MANIAC_LOAD = 3003,
        MANIAC_END_LOAD_PROCESS = 3004,
        MANIAC_GET_MOUSE_POSITION = 3005,
        MANIAC_SET_MOUSE_POSITION = 3006,
        MANIAC_SHOW_STRING_PICTURE = 3007,
        MANIAC_GET_PICTURE_INFO = 3008,
        MANIAC_CONTROL_BATTLE = 3009,
        MANIAC_CONTROL_ATB_GAUGE = 3010,
        MANIAC_CHANGE_BATTLE_COMMAND_EX = 3011,
        MANIAC_GET_BATTLE_INFO = 3012,
        MANIAC_CONTROL_VAR_ARRAY = 3013,
        MANIAC_KEY_INPUT_PROC_EX = 3014,
        MANIAC_REWRITE_MAP = 3015,
        MANIAC_CONTROL_GLOBAL_SAVE = 3016,
        MANIAC_CHANGE_PICTURE_ID = 3017,
        MANIAC_SET_GAME_OPTION = 3018,
        MANIAC_CALL_COMMAND = 3019,
        MANIAC_CONTROL_STRINGS = 3020,
        MANIAC_GET_GAME_INFO = 3021,
        MANIAC_EDIT_PICTURE = 3025,
        MANIAC_WRITE_PICTURE = 3026,
        MANIAC_ADD_MOVE_ROUTE = 3027,
        MANIAC_EDIT_TILE = 3028,
        MANIAC_CONTROL_TEXT_PROCESSING = 3029,
        MANIAC_ZOOM = 3032,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct EventPageManiacEventInfo {
        ACTION = 0,
        TOUCHED = 1,
        COLLISION = 2,
        AUTO_START = 3,
        PARALLEL = 4,
        CALLED = 5,
        BATTLE_START = 6,
        BATTLE_PARALLEL = 7,
        MAP_EVENT = 16,
        COMMON_EVENT = 32,
        BATTLE_EVENT = 64,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureEasyRpgFlip {
        NONE = 0,
        X = 1,
        Y = 2,
        BOTH = 3,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SavePictureEasyRpgType {
        DEFAULT = 0,
        WINDOW = 1,
        CANVAS = 2,
    }
}

lcf_enum! {
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct SkillHpType {
        COST = 0,
        PERCENT = 1,
    }
}
