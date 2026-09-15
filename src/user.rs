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

/// 身体活动水平等级（PAL，基于 NASEM 2023 成人分界定义）
///
/// 成人（≥19 岁）PAL 区间来源：NASEM 2023 *Dietary Reference Intakes for Energy*
/// Highlights / Health Canada DRI 表：Inactive <1.53，Low active <1.68，Active <1.85，Very active <2.50。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityLevel {
    /// 久坐 / 极少活动：1.00 <= PAL < 1.53
    Inactive,
    /// 低度活动：1.53 <= PAL < 1.68
    LowActive,
    /// 积极活动：1.68 <= PAL < 1.85
    Active,
    /// 极度活跃：1.85 <= PAL < 2.50
    VeryActive,
}

impl ActivityLevel {
    /// 返回该等级对应的代表性 PAL 中点（便于展示；成人 EER 本身按分类回归，不乘此系数）
    pub fn representative_pal(&self) -> f64 {
        match self {
            ActivityLevel::Inactive => 1.40,
            ActivityLevel::LowActive => 1.60,
            ActivityLevel::Active => 1.75,
            ActivityLevel::VeryActive => 2.05,
        }
    }

    /// 返回 NASEM 2023 成人 PAL 物理活动范围区间字符串
    pub fn pal_range_str(&self) -> &'static str {
        match self {
            ActivityLevel::Inactive => "1.00 <= PAL < 1.53",
            ActivityLevel::LowActive => "1.53 <= PAL < 1.68",
            ActivityLevel::Active => "1.68 <= PAL < 1.85",
            ActivityLevel::VeryActive => "1.85 <= PAL < 2.50",
        }
    }
}

/// 生殖/泌乳生命阶段（影响能量方程适用性）
///
/// 当前核心仅实现**非孕非哺成人** EER。妊娠与哺乳在 NASEM 2023 中有独立修正项，
/// 在实现专用方程前必须显式拒绝，避免把非孕成人公式当成孕哺预算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ReproductiveStatus {
    /// 非妊娠、非哺乳（默认）
    #[default]
    None,
    /// 妊娠（trimester：1/2/3，仅用于拒绝提示；专用 EER 尚未接入）
    Pregnant {
        trimester: u8,
    },
    /// 哺乳期
    Lactating,
}

impl ReproductiveStatus {
    /// 供错误提示使用的中文状态名
    pub fn display_name(self) -> &'static str {
        match self {
            ReproductiveStatus::None => "非孕非哺",
            ReproductiveStatus::Pregnant { .. } => "妊娠",
            ReproductiveStatus::Lactating => "哺乳",
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
    /// 生殖/泌乳状态（默认非孕非哺；旧备份 JSON 缺省字段时按 None 反序列化）
    #[serde(default)]
    pub reproductive_status: ReproductiveStatus,
}

impl UserProfile {
    /// 构造并校验用户档案（默认非孕非哺）
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
            reproductive_status: ReproductiveStatus::None,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// 设置生殖/泌乳状态（用于显式声明妊娠或哺乳，从而触发能量方程拒绝）
    pub fn with_reproductive_status(mut self, status: ReproductiveStatus) -> Self {
        self.reproductive_status = status;
        self
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
