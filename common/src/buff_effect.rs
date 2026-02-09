#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExceptionEffect {
    /// 正常
    #[default]
    Normal,
    /// 击昏
    Stun,
    /// 疲劳
    Fatigue,
    /// 麻痹
    Numbness,
    /// 冰冻
    Freeze,
    /// 中毒
    Poisoning,
    /// 流血
    Bleeding,
    /// 窒息
    Sinking,
    /// 灼伤
    Burn,
    /// 剧毒
    HighlyToxic,
}
