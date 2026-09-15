//! 核心生理数据模型模块
//!
//! 包含用户生理档案、性别生理学细分（非二元性别与激素表型支持）、
//! 身体活动等级、以及核心计算异常定义。

pub mod error;
pub mod user;

pub use error::NutritionError;
pub use user::{ActivityLevel, Gender, HormoneProfile, UserProfile};
