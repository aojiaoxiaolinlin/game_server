pub mod attributes;
pub mod skills;

use serde::{Deserialize, Serialize};

use crate::sprites::skills::Skill;

/// 玩家配置的精灵数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    pub id: u64,
    /// 精灵等级
    pub level: u8,
    /// 精灵当前经验值
    pub exp: u32,
    /// 精灵最大经验值
    pub max_exp: u32,

    // 强度数据面板
    /// 精灵当前生命值
    pub hp: u16,
    /// 精灵最大生命值
    pub max_hp: u16,

    /// 物理攻击力
    pub phy_atk: u16,
    /// 物理防御力
    pub phy_def: u16,
    /// 法术攻击力——法力
    pub mag_atk: u16,
    /// 法术防御力——抗性
    pub mag_def: u16,
    /// 速度
    pub speed: u16,

    /// 携带的技能
    pub skills: Vec<Skill>,
}
