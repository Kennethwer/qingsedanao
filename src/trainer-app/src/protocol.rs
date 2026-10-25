//! 与游戏内插件的通信协议，和 src/Shared/Protocol.cs 保持一致。
//!
//! 本机 TCP，按行收发 UTF-8：
//! - `GET` → `STATE key=value;key=value;...`
//! - `SET <key> <value>` → `OK` | `ERR <原因>`
//! - `DO <action> [arg]` → `OK` | `ERR <原因>`
//!
//! 连接建立后插件先发一行 `HELLO <版本>`。

pub const DEFAULT_PORT: u16 = 37421;

pub const HELLO: &str = "HELLO";
pub const GET: &str = "GET";
pub const SET: &str = "SET";
pub const DO: &str = "DO";
pub const STATE: &str = "STATE";
pub const ERR: &str = "ERR";

pub enum Kind {
    Toggle,
    Slider { min: f32, max: f32, step: f32, default: f32, unit: &'static str },
}

pub struct Setting {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: Kind,
}

pub struct Section {
    pub title: &'static str,
    pub settings: &'static [Setting],
}

const fn toggle(key: &'static str, label: &'static str, hint: &'static str) -> Setting {
    Setting { key, label, hint, kind: Kind::Toggle }
}

const fn slider(key: &'static str, label: &'static str, min: f32, max: f32, step: f32, unit: &'static str) -> Setting {
    let default = if min > 1.0 { min } else { 1.0 };
    Setting { key, label, hint: "", kind: Kind::Slider { min, max, step, default, unit } }
}

/// 左栏
pub const LEFT_SECTIONS: &[Section] = &[
    Section {
        title: "生存",
        settings: &[
            toggle("godMode", "无敌", "不掉血，也不会被陷阱、地雷、怪物处决"),
            toggle(
                "noInstantKill",
                "防秒杀",
                "陷阱和地雷不再触发，掉进深坑会被拉回刚才站的位置，追击型怪物碰到也不会直接处决，没有摔落伤害。普通伤害照常。",
            ),
            toggle("lockHunger", "锁定饱食度", "饱食度保持为满，药剂也会按最快速度恢复"),
            toggle("infiniteEnergy", "无限能量", "能量（翻滚消耗）始终为满"),
            toggle("infiniteFlashlight", "无限手电", "手电电量始终为满"),
        ],
    },
    Section {
        title: "战斗",
        settings: &[
            toggle("oneHitKill", "一击必杀", "对怪物造成的伤害直接致死"),
            slider("damageMultiplier", "伤害倍率", 1.0, 20.0, 0.1, "x"),
            toggle("infiniteAmmo", "无限子弹", "开枪不消耗子弹，弹夹不减少，不用换弹"),
        ],
    },
];

/// 右栏
pub const RIGHT_SECTIONS: &[Section] = &[
    Section {
        title: "移动",
        settings: &[
            toggle("multiJump", "连跳", "在空中可以再次起跳"),
            slider("extraJumps", "空中跳跃", 1.0, 10.0, 1.0, "次"),
            slider("jumpHeight", "跳跃高度", 0.5, 3.0, 0.1, "x"),
            slider("moveSpeed", "移动速度", 0.5, 3.0, 0.1, "x"),
            slider("gameSpeed", "游戏速度", 0.1, 3.0, 0.1, "x"),
        ],
    },
    Section {
        title: "经验",
        settings: &[slider("expMultiplier", "经验倍率", 1.0, 10.0, 0.1, "x")],
    },
];

pub struct Action {
    pub label: &'static str,
    pub command: &'static str,
    /// 只能在关卡里用
    pub needs_player: bool,
}

pub const ACTIONS: &[Action] = &[
    Action { label: "回满血", command: "heal", needs_player: true },
    Action { label: "补满药剂", command: "refillMedicine", needs_player: true },
    Action { label: "补满弹药和地雷", command: "refillAmmo", needs_player: true },
    Action { label: "经验 +1000", command: "addExp 1000", needs_player: true },
    Action { label: "经验 +10000", command: "addExp 10000", needs_player: true },
    Action { label: "恢复默认设置", command: "resetAll", needs_player: false },
];

/// STATE 里除设置项以外的只读字段
pub mod state {
    pub const IN_GAME: &str = "inGame";
    pub const SCENE: &str = "scene";
    pub const HP: &str = "hp";
    pub const MAX_HP: &str = "maxHp";
    pub const EXP: &str = "exp";
    pub const MEDICINE: &str = "medicine";
    pub const MAX_MEDICINE: &str = "maxMedicine";
}
