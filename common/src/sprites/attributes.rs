use serde::{Deserialize, Serialize};

/// 技能类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SkillType {
    /// 物理攻击
    Physical,
    /// 法术攻击
    Magical,
}

/// 技能属性
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Attribute {
    /// 金
    Jin,
    /// 木
    Mu,
    /// 水
    Shui,
    /// 火
    Huo,
    /// 土
    Tu,
    /// 翼
    Yi,
    /// 怪
    Guai,
    /// 魔
    Mo,
    /// 妖
    Yao,
    /// 凤
    Feng,
    /// 毒
    Du,
    /// 雷
    Lei,
    /// 幻
    Huan,
    /// 冰
    Bing,
    /// 灵
    Ling,
    /// 机械
    JiXie,
    /// 火风
    Huofeng,
    /// 木灵
    Wuling,
    /// 圣
    Seng,
    /// 土幻
    Tonghuan,
    /// 水妖
    ShuiYao,
    /// 音
    Yin,
    /// 特殊
    Special,
    /// 无属性
    #[default]
    None,
}
