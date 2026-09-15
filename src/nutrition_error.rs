use thiserror::Error;

/// 营养与生理计算领域错误
#[derive(Error, Debug, PartialEq)]
pub enum NutritionError {
    #[error("年龄必须在 1 到 120 岁之间，当前输入: {0}")]
    InvalidAge(u8),

    #[error("身高必须在人类正常生理范围内（50 到 280 厘米），当前输入: {0} cm")]
    InvalidHeight(f64),

    #[error("体重必须在合理生理范围内（20 到 500 千克），当前输入: {0} kg")]
    InvalidWeight(f64),

    /// 当前能量方程仅覆盖非孕非哺的成年人（≥19 岁）。
    /// 儿童/青少年需使用 NASEM 2023 带生长耗能（ECG）的专用方程，不可套用成人回归式。
    #[error(
        "能量需求方程暂不支持该生命阶段：年龄 {age} 岁（需 ≥ {min_adult_age} 岁成人）。请勿将成人 EER 用于未成年人。"
    )]
    UnsupportedAgeForEnergy {
        age: u8,
        min_adult_age: u8,
    },

    /// 妊娠期与哺乳期有独立的 NASEM EER 修正（孕周沉积能、泌乳额外需求），当前核心未实现，显式拒绝以免误用。
    #[error("能量需求方程暂不支持{0}状态。妊娠/哺乳请咨询临床营养方案，勿直接套用非孕成人 EER。")]
    UnsupportedReproductiveStatus(&'static str),

    #[error("计算错误: {0}")]
    CalculationError(String),
}
