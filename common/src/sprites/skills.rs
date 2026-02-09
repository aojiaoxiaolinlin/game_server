use serde::{Deserialize, Serialize};

use super::attributes::{Attribute, SkillType};

/// 技能数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: u64,
    /// 技能名称
    pub name: String,
    /// 技能描述
    pub description: String,
    /// 技能类型
    pub skill_type: SkillType,
    /// 技能属性
    pub attribute: Attribute,
    /// 技能PP值
    pub pp: u16,
    /// 技能最大PP值
    pub max_pp: u16,
    /// 技能威力
    pub power: u16,
    /// 是否是先手技能
    pub is_preemptive: bool,

    /// 技能特殊效果
    pub special_effect: Option<SkillSpecialEffect>,
}

/// 技能特殊效果
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SkillSpecialEffect {
    /// 提升属性
    BoostAttribute,
    /// 降低属性
    ReduceAttribute,
    /// 状态效果
    StatusEffect,
}
