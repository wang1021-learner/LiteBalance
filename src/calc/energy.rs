use crate::models::{ActivityLevel, Gender, HormoneProfile, UserProfile};

/// 能量计算参考标准规范
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnergyStandard {
    /// 最新的 NASEM 2023 官方回归方程矩阵（Dietary Reference Intakes for Energy，推荐默认）
    #[default]
    Nasem2023,
    /// 经典的 IOM 2005 乘法 PA 模型（向后兼容旧版标准）
    Iom2005,
}

/// NASEM 2023 与 IOM 2005 代谢计算对比明细
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyComparison {
    /// NASEM 2023 计算值（kcal/天）
    pub nasem_2023_kcal: f64,
    /// IOM 2005 计算值（kcal/天）
    pub iom_2005_kcal: f64,
    /// 差值绝对值（NASEM - IOM，kcal）
    pub difference_kcal: f64,
    /// 差异百分比（%）
    pub difference_pct: f64,
}

/// 每日总能量消耗（TDEE）与预估能量需求（EER）计算器
pub struct EnergyCalc;

impl EnergyCalc {
    /// 根据指定的能量标准计算每日能量需求（默认采用 NASEM 2023）
    pub fn calculate(profile: &UserProfile, standard: EnergyStandard) -> f64 {
        match standard {
            EnergyStandard::Nasem2023 => Self::nasem_2023(profile),
            EnergyStandard::Iom2005 => Self::iom_2005(profile),
        }
    }

    /// 按照 NASEM 2023 官方最新方程计算预估能量需求（EER）
    ///
    /// 文献参考：
    /// 美国国家科学院、工程院和医学院 (NASEM). 2023.
    /// 《Dietary Reference Intakes for Energy》. 华盛顿: 国家学术出版社.
    /// 第 5 章：预估能量需求预测方程的开发（基于双标水真实代谢测定矩阵）。
    pub fn nasem_2023(profile: &UserProfile) -> f64 {
        match profile.gender {
            Gender::Male => Self::nasem_2023_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level),
            Gender::Female => Self::nasem_2023_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level),
            Gender::NonBinary(hormone) => match hormone {
                HormoneProfile::Averaged => {
                    let male = Self::nasem_2023_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level);
                    let female = Self::nasem_2023_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level);
                    (male + female) / 2.0
                }
                HormoneProfile::EstrogenTypical => {
                    Self::nasem_2023_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level)
                }
                HormoneProfile::TestosteroneTypical => {
                    Self::nasem_2023_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level)
                }
            },
        }
    }

    /// NASEM 2023 成年男性（>= 19 岁）四分类专用回归方程：
    ///
    /// - 久坐 (Inactive):    753.07 - 10.83 * A + 6.50 * H + 14.10 * W
    /// - 低活动 (Low active):  581.47 - 10.83 * A + 8.30 * H + 14.94 * W
    /// - 积极活动 (Active):     1004.82 - 10.83 * A + 6.52 * H + 15.91 * W
    /// - 极度活跃 (Very active):-517.88 - 10.83 * A + 15.61 * H + 19.11 * W
    pub fn nasem_2023_male(age: u8, height_cm: f64, weight_kg: f64, activity: ActivityLevel) -> f64 {
        let a = age as f64;
        let h = height_cm;
        let w = weight_kg;

        match activity {
            ActivityLevel::Inactive => 753.07 - 10.83 * a + 6.50 * h + 14.10 * w,
            ActivityLevel::LowActive => 581.47 - 10.83 * a + 8.30 * h + 14.94 * w,
            ActivityLevel::Active => 1004.82 - 10.83 * a + 6.52 * h + 15.91 * w,
            ActivityLevel::VeryActive => -517.88 - 10.83 * a + 15.61 * h + 19.11 * w,
        }
    }

    /// NASEM 2023 成年女性（>= 19 岁）四分类专用回归方程：
    ///
    /// - 久坐 (Inactive):   584.90 - 7.01 * A + 5.72 * H + 11.71 * W
    /// - 低活动 (Low active): 575.77 - 7.01 * A + 6.60 * H + 12.14 * W
    /// - 积极活动 (Active):     710.25 - 7.01 * A + 6.54 * H + 12.34 * W
    /// - 极度活跃 (Very active):511.83 - 7.01 * A + 9.07 * H + 12.56 * W
    pub fn nasem_2023_female(age: u8, height_cm: f64, weight_kg: f64, activity: ActivityLevel) -> f64 {
        let a = age as f64;
        let h = height_cm;
        let w = weight_kg;

        match activity {
            ActivityLevel::Inactive => 584.90 - 7.01 * a + 5.72 * h + 11.71 * w,
            ActivityLevel::LowActive => 575.77 - 7.01 * a + 6.60 * h + 12.14 * w,
            ActivityLevel::Active => 710.25 - 7.01 * a + 6.54 * h + 12.34 * w,
            ActivityLevel::VeryActive => 511.83 - 7.01 * a + 9.07 * h + 12.56 * w,
        }
    }

    /// IOM 2005 TDEE 方程（向后兼容旧版计算标准）
    ///
    /// - 男性: TEE = 864 - 9.72 * A + PA * (14.2 * W + 503 * H_m)
    /// - 女性: TEE = 387 - 7.31 * A + PA * (10.9 * W + 660.7 * H_m)
    pub fn iom_2005(profile: &UserProfile) -> f64 {
        match profile.gender {
            Gender::Male => Self::iom_2005_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level),
            Gender::Female => Self::iom_2005_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level),
            Gender::NonBinary(hormone) => match hormone {
                HormoneProfile::Averaged => {
                    let male = Self::iom_2005_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level);
                    let female = Self::iom_2005_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level);
                    (male + female) / 2.0
                }
                HormoneProfile::EstrogenTypical => {
                    Self::iom_2005_female(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level)
                }
                HormoneProfile::TestosteroneTypical => {
                    Self::iom_2005_male(profile.age, profile.height_cm, profile.weight_kg, profile.activity_level)
                }
            },
        }
    }

    pub fn iom_2005_male(age: u8, height_cm: f64, weight_kg: f64, activity: ActivityLevel) -> f64 {
        let pa = match activity {
            ActivityLevel::Inactive => 1.00,
            ActivityLevel::LowActive => 1.12,
            ActivityLevel::Active => 1.27,
            ActivityLevel::VeryActive => 1.54,
        };
        let h_m = height_cm / 100.0;
        864.0 - 9.72 * (age as f64) + pa * (14.2 * weight_kg + 503.0 * h_m)
    }

    pub fn iom_2005_female(age: u8, height_cm: f64, weight_kg: f64, activity: ActivityLevel) -> f64 {
        let pa = match activity {
            ActivityLevel::Inactive => 1.00,
            ActivityLevel::LowActive => 1.14,
            ActivityLevel::Active => 1.27,
            ActivityLevel::VeryActive => 1.45,
        };
        let h_m = height_cm / 100.0;
        387.0 - 7.31 * (age as f64) + pa * (10.9 * weight_kg + 660.7 * h_m)
    }

    /// 对比 NASEM 2023 与 IOM 2005 的预测差异，用于数据透明度与历史对照
    pub fn compare(profile: &UserProfile) -> EnergyComparison {
        let nasem = Self::nasem_2023(profile);
        let iom = Self::iom_2005(profile);
        let diff = nasem - iom;
        let pct = if iom.abs() > f64::EPSILON {
            (diff / iom) * 100.0
        } else {
            0.0
        };

        EnergyComparison {
            nasem_2023_kcal: nasem,
            iom_2005_kcal: iom,
            difference_kcal: diff,
            difference_pct: pct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nasem_2023_male_inactive() {
        // 30岁男性，175cm，75kg，久坐
        // 753.07 - 10.83 * 30 + 6.50 * 175 + 14.10 * 75 = 753.07 - 324.9 + 1137.5 + 1057.5 = 2623.17 kcal
        let res = EnergyCalc::nasem_2023_male(30, 175.0, 75.0, ActivityLevel::Inactive);
        let expected = 753.07 - 10.83 * 30.0 + 6.50 * 175.0 + 14.10 * 75.0;
        assert!((res - expected).abs() < 1e-4);
        assert!((res - 2623.17).abs() < 1e-2);
    }

    #[test]
    fn test_nasem_2023_male_very_active() {
        // 25岁男性，180cm，80kg，极度活跃
        // -517.88 - 10.83 * 25 + 15.61 * 180 + 19.11 * 80 = 3549.97 kcal
        let res = EnergyCalc::nasem_2023_male(25, 180.0, 80.0, ActivityLevel::VeryActive);
        let expected = -517.88 - 10.83 * 25.0 + 15.61 * 180.0 + 19.11 * 80.0;
        assert!((res - expected).abs() < 1e-4);
        assert!((res - 3549.97).abs() < 1e-2);
    }

    #[test]
    fn test_nasem_2023_female_low_active() {
        // 28岁女性，165cm，60kg，低活动
        // 575.77 - 7.01 * 28 + 6.60 * 165 + 12.14 * 60 = 575.77 - 196.28 + 1089.0 + 728.4 = 2196.89 kcal
        let res = EnergyCalc::nasem_2023_female(28, 165.0, 60.0, ActivityLevel::LowActive);
        let expected = 575.77 - 7.01 * 28.0 + 6.60 * 165.0 + 12.14 * 60.0;
        assert!((res - expected).abs() < 1e-4);
        assert!((res - 2196.89).abs() < 1e-2);
    }

    #[test]
    fn test_nasem_2023_female_active() {
        // 35岁女性，168cm，65kg，积极活动
        // 710.25 - 7.01 * 35 + 6.54 * 168 + 12.34 * 65 = 2365.72 kcal
        let res = EnergyCalc::nasem_2023_female(35, 168.0, 65.0, ActivityLevel::Active);
        let expected = 710.25 - 7.01 * 35.0 + 6.54 * 168.0 + 12.34 * 65.0;
        assert!((res - expected).abs() < 1e-4);
        assert!((res - 2365.72).abs() < 1e-2);
    }

    #[test]
    fn test_non_binary_averaged() {
        let profile = UserProfile::new(
            30,
            170.0,
            70.0,
            Gender::NonBinary(HormoneProfile::Averaged),
            ActivityLevel::Active,
        ).unwrap();

        let male = EnergyCalc::nasem_2023_male(30, 170.0, 70.0, ActivityLevel::Active);
        let female = EnergyCalc::nasem_2023_female(30, 170.0, 70.0, ActivityLevel::Active);
        let expected = (male + female) / 2.0;

        let result = EnergyCalc::nasem_2023(&profile);
        assert!((result - expected).abs() < 1e-4);
    }

    #[test]
    fn test_comparison_shows_refinement() {
        let profile = UserProfile::new(
            30,
            175.0,
            75.0,
            Gender::Male,
            ActivityLevel::LowActive,
        ).unwrap();

        let comp = EnergyCalc::compare(&profile);
        assert!(comp.nasem_2023_kcal > 2000.0);
        assert!(comp.iom_2005_kcal > 2000.0);
        assert_eq!(comp.difference_kcal, comp.nasem_2023_kcal - comp.iom_2005_kcal);
    }
}
