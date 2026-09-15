use crate::nutrition_error::NutritionError;
use serde::{Deserialize, Serialize};

/// 生理激素代谢表型（用于非二元性别或跨性别代谢基准计算）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HormoneProfile {
    /// 算术均值（取男性与女性参考模型的平均值作为中性基准）
    Averaged,
    /// 雌激素主导表型（采用女性代谢参考参数）
    EstrogenTypical,
    /// 雄激素主导表型（采用男性代谢参考参数）
    TestosteroneTypical,
}

/// 性别分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    /// 男性
    Male,
    /// 女性
    Female,
    /// 非二元性别（携带特定激素代谢表型）
    NonBinary(HormoneProfile),
}

/// 身体活动水平等级（PAL，基于 NASEM 2023 与 IOM 标准定义）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityLevel {
    /// 久坐 / 极少活动：1.00 <= PAL < 1.40（如典型的办公室案头工作，极少步行）
    Inactive,
    /// 低度活动：1.40 <= PAL < 1.60（如日常通勤走动，加上每天约 30~60 分钟轻松步行）
    LowActive,
    /// 积极活动：1.60 <= PAL < 1.90（如每天至少 60 分钟中高强度运动）
    Active,
    /// 极度活跃：1.90 <= PAL < 2.50（如重体力劳动者或每日高负荷耐力/竞技运动员）
    VeryActive,
}

impl ActivityLevel {
    /// 返回该等级对应的代表性 PAL 系数数值（常用于老版 IOM 2005 模型对比）
    pub fn representative_pal(&self) -> f64 {
        match self {
            ActivityLevel::Inactive => 1.25,
            ActivityLevel::LowActive => 1.50,
            ActivityLevel::Active => 1.75,
            ActivityLevel::VeryActive => 2.20,
        }
    }

    /// 返回标准 PAL 物理活动范围区间字符串
    pub fn pal_range_str(&self) -> &'static str {
        match self {
            ActivityLevel::Inactive => "1.00 <= PAL < 1.40",
            ActivityLevel::LowActive => "1.40 <= PAL < 1.60",
            ActivityLevel::Active => "1.60 <= PAL < 1.90",
            ActivityLevel::VeryActive => "1.90 <= PAL < 2.50",
        }
    }
}

/// 用户生理档案实体（用于代谢与营养计算）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// 年龄（整岁）
    pub age: u8,
    /// 身高（厘米，cm）
    pub height_cm: f64,
    /// 体重（千克，kg）
    pub weight_kg: f64,
    /// 性别及代谢表型
    pub gender: Gender,
    /// 身体日常活动等级
    pub activity_level: ActivityLevel,
}

impl UserProfile {
    /// 构造并校验用户档案
    pub fn new(
        age: u8,
        height_cm: f64,
        weight_kg: f64,
        gender: Gender,
        activity_level: ActivityLevel,
    ) -> Result<Self, NutritionError> {
        let profile = Self {
            age,
            height_cm,
            weight_kg,
            gender,
            activity_level,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// 校验各项生理指标是否在人类医学正常范围内
    pub fn validate(&self) -> Result<(), NutritionError> {
        if self.age < 1 || self.age > 120 {
            return Err(NutritionError::InvalidAge(self.age));
        }
        if self.height_cm < 50.0 || self.height_cm > 280.0 {
            return Err(NutritionError::InvalidHeight(self.height_cm));
        }
        if self.weight_kg < 20.0 || self.weight_kg > 500.0 {
            return Err(NutritionError::InvalidWeight(self.weight_kg));
        }
        Ok(())
    }

    /// 身高转换为米（m）
    #[inline]
    pub fn height_m(&self) -> f64 {
        self.height_cm / 100.0
    }

    /// 身体质量指数（BMI = 体重kg / 身高m²）
    #[inline]
    pub fn bmi(&self) -> f64 {
        let h_m = self.height_m();
        self.weight_kg / (h_m * h_m)
    }
}
