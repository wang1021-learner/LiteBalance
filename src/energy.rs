use crate::nutrition_error::NutritionError;
use crate::user::{ActivityLevel, Gender, HormoneProfile, ReproductiveStatus, UserProfile};

/// NASEM 2023 成人（≥19 岁）能量方程适用的最低年龄。
pub const ADULT_ENERGY_MIN_AGE: u8 = 19;

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
    /// 校验档案是否适用于当前已实现的成人非孕非哺能量方程。
    ///
    /// 未成年人与妊娠/哺乳必须走 NASEM 专用生命阶段方程；在尚未接入前显式拒绝，
    /// 避免把成人回归式误用于不可适用人群。
    pub fn validate_energy_eligibility(profile: &UserProfile) -> Result<(), NutritionError> {
        if profile.age < ADULT_ENERGY_MIN_AGE {
            return Err(NutritionError::UnsupportedAgeForEnergy {
                age: profile.age,
                min_adult_age: ADULT_ENERGY_MIN_AGE,
            });
        }
        match profile.reproductive_status {
            ReproductiveStatus::None => Ok(()),
            ReproductiveStatus::Pregnant { .. } => {
                Err(NutritionError::UnsupportedReproductiveStatus("妊娠"))
            }
            ReproductiveStatus::Lactating => {
                Err(NutritionError::UnsupportedReproductiveStatus("哺乳"))
            }
        }
    }

    /// 根据指定的能量标准计算每日能量需求（默认采用 NASEM 2023）
    pub fn calculate(profile: &UserProfile, standard: EnergyStandard) -> Result<f64, NutritionError> {
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
    ///
    /// # 适用范围
    /// 仅接受年龄 ≥19 且非孕非哺的档案。系数表与 Health Canada / NASEM Table 5-16 成人式一致。
    pub fn nasem_2023(profile: &UserProfile) -> Result<f64, NutritionError> {
        Self::validate_energy_eligibility(profile)?;
        Ok(match profile.gender {
            Gender::Male => Self::nasem_2023_male(
                profile.age,
                profile.height_cm,
                profile.weight_kg,
                profile.activity_level,
            ),
            Gender::Female => Self::nasem_2023_female(
                profile.age,
                profile.height_cm,
                profile.weight_kg,
                profile.activity_level,
            ),
            Gender::NonBinary(hormone) => match hormone {
                HormoneProfile::Averaged => {
                    let male = Self::nasem_2023_male(
                        profile.age,
                        profile.height_cm,
                        profile.weight_kg,
                        profile.activity_level,
                    );
                    let female = Self::nasem_2023_female(
                        profile.age,
                        profile.height_cm,
                        profile.weight_kg,
                        profile.activity_level,
                    );
                    (male + female) / 2.0
                }
                HormoneProfile::EstrogenTypical => Self::nasem_2023_female(
                    profile.age,
                    profile.height_cm,
                    profile.weight_kg,
                    profile.activity_level,
                ),
                HormoneProfile::TestosteroneTypical => Self::nasem_2023_male(
                    profile.age,
                    profile.height_cm,
                    profile.weight_kg,
                    profile.activity_level,
                ),
            },
        })
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

    /// IOM 2005 TDEE 方程（向后兼容旧版计算标准；同样仅适用于非孕非哺成人）
    ///
    /// - 男性: TEE = 864 - 9.72 * A + PA * (14.2 * W + 503 * H_m)
    /// - 女性: TEE = 387 - 7.31 * A + PA * (10.9 * W + 660.7 * H_m)
    pub fn iom_2005(profile: &UserProfile) -> Result<f64, NutritionError> {
        Self::validate_energy_eligibility(profile)?;
        Ok(match profile.gender {
            Gender::Male => Self::iom_2005_male(
                profile.age,
                profile.height_cm,
                profile.weight_kg,
                profile.activity_level,
            ),
            Gender::Female => Self::iom_2005_female(
                profile.age,
                profile.height_cm,
                profile.weight_kg,
                profile.activity_level,
            ),
            Gender::NonBinary(hormone) => match hormone {
                HormoneProfile::Averaged => {
                    let male = Self::iom_2005_male(
                        profile.age,
                        profile.height_cm,
                        profile.weight_kg,
                        profile.activity_level,
                    );
                    let female = Self::iom_2005_female(
                        profile.age,
                        profile.height_cm,
                        profile.weight_kg,
                        profile.activity_level,
                    );
                    (male + female) / 2.0
                }
                HormoneProfile::EstrogenTypical => Self::iom_2005_female(
                    profile.age,
                    profile.height_cm,
                    profile.weight_kg,
                    profile.activity_level,
                ),
                HormoneProfile::TestosteroneTypical => Self::iom_2005_male(
                    profile.age,
                    profile.height_cm,
                    profile.weight_kg,
                    profile.activity_level,
                ),
            },
        })
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
    pub fn compare(profile: &UserProfile) -> Result<EnergyComparison, NutritionError> {
        let nasem = Self::nasem_2023(profile)?;
        let iom = Self::iom_2005(profile)?;
        let diff = nasem - iom;
        let pct = if iom.abs() > f64::EPSILON {
            (diff / iom) * 100.0
        } else {
            0.0
        };

        Ok(EnergyComparison {
            nasem_2023_kcal: nasem,
            iom_2005_kcal: iom,
            difference_kcal: diff,
            difference_pct: pct,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nutrition_error::NutritionError;

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

    /// 黄金测试：NASEM 2023 Highlights 官方演算例题。
    ///
    /// 来源：National Academies, *DRIs for Energy Highlights* (2023)：
    /// 22 岁非孕非哺女性，165 cm，63 kg，Low Active → EER = 2,275 kcal/天。
    #[test]
    fn test_nasem_2023_official_highlights_worked_example() {
        let profile = UserProfile::new(22, 165.0, 63.0, Gender::Female, ActivityLevel::LowActive).unwrap();
        let eer = EnergyCalc::nasem_2023(&profile).unwrap();
        // 官方逐步演算：575.77 - 154.22 + 1089.0 + 764.82 = 2275.37 → 报告为 2,275 kcal/天
        assert!((eer - 2275.37).abs() < 1e-2);
        assert!((eer.round() - 2275.0).abs() < 1e-9);
    }

    /// 黄金测试：成人男性 Inactive 系数与 Health Canada / NASEM Table 5-16 一致。
    #[test]
    fn test_nasem_2023_male_coefficients_match_table_5_16() {
        // EER = 753.07 - 10.83*A + 6.50*H + 14.10*W
        let eer = EnergyCalc::nasem_2023_male(40, 178.0, 82.0, ActivityLevel::Inactive);
        let expected = 753.07 - 10.83 * 40.0 + 6.50 * 178.0 + 14.10 * 82.0;
        assert!((eer - expected).abs() < 1e-9);
    }

    #[test]
    fn test_non_binary_averaged() {
        let profile = UserProfile::new(
            30,
            170.0,
            70.0,
            Gender::NonBinary(HormoneProfile::Averaged),
            ActivityLevel::Active,
        )
        .unwrap();

        let male = EnergyCalc::nasem_2023_male(30, 170.0, 70.0, ActivityLevel::Active);
        let female = EnergyCalc::nasem_2023_female(30, 170.0, 70.0, ActivityLevel::Active);
        let expected = (male + female) / 2.0;

        let result = EnergyCalc::nasem_2023(&profile).unwrap();
        assert!((result - expected).abs() < 1e-4);
    }

    #[test]
    fn test_comparison_shows_refinement() {
        let profile = UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::LowActive).unwrap();

        let comp = EnergyCalc::compare(&profile).unwrap();
        assert!(comp.nasem_2023_kcal > 2000.0);
        assert!(comp.iom_2005_kcal > 2000.0);
        assert_eq!(comp.difference_kcal, comp.nasem_2023_kcal - comp.iom_2005_kcal);
    }

    #[test]
    fn test_rejects_minor_for_adult_eer() {
        let teen = UserProfile::new(16, 170.0, 60.0, Gender::Female, ActivityLevel::Active).unwrap();
        let err = EnergyCalc::nasem_2023(&teen).unwrap_err();
        assert_eq!(
            err,
            NutritionError::UnsupportedAgeForEnergy {
                age: 16,
                min_adult_age: ADULT_ENERGY_MIN_AGE
            }
        );
    }

    #[test]
    fn test_rejects_pregnancy_and_lactation() {
        let pregnant = UserProfile::new(28, 165.0, 68.0, Gender::Female, ActivityLevel::LowActive)
            .unwrap()
            .with_reproductive_status(ReproductiveStatus::Pregnant { trimester: 2 });
        assert_eq!(
            EnergyCalc::nasem_2023(&pregnant).unwrap_err(),
            NutritionError::UnsupportedReproductiveStatus("妊娠")
        );

        let lactating = UserProfile::new(30, 165.0, 65.0, Gender::Female, ActivityLevel::LowActive)
            .unwrap()
            .with_reproductive_status(ReproductiveStatus::Lactating);
        assert_eq!(
            EnergyCalc::iom_2005(&lactating).unwrap_err(),
            NutritionError::UnsupportedReproductiveStatus("哺乳")
        );
    }

    #[test]
    fn test_adult_pal_ranges_match_nasem_2023() {
        assert_eq!(ActivityLevel::Inactive.pal_range_str(), "1.00 <= PAL < 1.53");
        assert_eq!(ActivityLevel::LowActive.pal_range_str(), "1.53 <= PAL < 1.68");
        assert_eq!(ActivityLevel::Active.pal_range_str(), "1.68 <= PAL < 1.85");
        assert_eq!(ActivityLevel::VeryActive.pal_range_str(), "1.85 <= PAL < 2.50");
    }
}
