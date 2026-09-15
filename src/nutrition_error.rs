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

    #[error("计算错误: {0}")]
    CalculationError(String),
}
