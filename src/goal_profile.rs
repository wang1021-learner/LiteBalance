//! 动态卡路里预算与自适应目标调节器 (goal_profile.rs)
//!
//! 提供基于临床代谢动力学与生理反馈控制的卡路里目标调节引擎：
//! 1. 目标周速率换算（基于 7700 kcal/kg 体脂守恒常数：1100 kcal/天/kg/周）；
//! 2. 渐进平稳着陆缓冲（Tapering）：在距目标体重 5.0kg ~ 1.0kg 区间内线性平滑收敛至维持热量；
//! 3. 实测体重 OLS 回归斜率自适应微调（Adaptive Feedback Controller）：
//!    针对平台期或过速减重进行带阻尼（0.5 系数）与安全阈值（±250 kcal）的闭环调节；
//! 4. 临床级内分泌安全红线（Safety Floor）：强制男性 ≥1500 kcal，女性 ≥1200 kcal，
//!    防止下丘脑-垂体-性腺轴与甲状腺内分泌损伤。

use crate::energy::EnergyCalc;
use crate::macro_engine::{DietProtocol, MacroEngine};
use crate::nutrition_error::NutritionError;
use crate::user::{Gender, HormoneProfile, UserProfile};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 目标类型分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalKind {
    /// 减脂减重
    LoseWeight,
    /// 维持现有体重
    MaintainWeight,
    /// 增肌增重
    GainWeight,
}

impl GoalKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalKind::LoseWeight => "lose_weight",
            GoalKind::MaintainWeight => "maintain_weight",
            GoalKind::GainWeight => "gain_weight",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lose" | "lose_weight" | "cut" => GoalKind::LoseWeight,
            "gain" | "gain_weight" | "bulk" => GoalKind::GainWeight,
            _ => GoalKind::MaintainWeight,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GoalKind::LoseWeight => "减脂减重",
            GoalKind::MaintainWeight => "维持体重",
            GoalKind::GainWeight => "增肌增重",
        }
    }
}

/// 用户体态与体重目标配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoalConfig {
    /// 目标分类（减重/维持/增重）
    pub kind: GoalKind,
    /// 目标体重（kg，若维持可为 None）
    pub target_weight_kg: Option<f64>,
    /// 预期每周体重变化速率（kg/周，减重为负值如 -0.5，增重为正值如 +0.25）
    pub weekly_rate_kg: f64,
    /// 是否开启距目标临近时的平稳着陆缓冲
    pub taper_enabled: bool,
    /// 是否开启基于近期体重 OLS 回归斜率的闭环自适应微调
    pub adaptive_enabled: bool,
    /// 用户手动微调卡路里偏移（kcal/天，可正可负）
    pub manual_calorie_offset: f64,
}

impl Default for GoalConfig {
    fn default() -> Self {
        Self {
            kind: GoalKind::MaintainWeight,
            target_weight_kg: None,
            weekly_rate_kg: 0.0,
            taper_enabled: true,
            adaptive_enabled: true,
            manual_calorie_offset: 0.0,
        }
    }
}

/// 自适应卡路里预算与宏量素规划结算结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdaptiveBudgetResult {
    /// 用户基准维持消耗 TDEE（NASEM 2023 DRI，kcal/天）
    pub base_tdee_kcal: f64,
    /// 目标周速率对应的原始理论热量调整值（kcal/天）
    pub raw_rate_adjustment_kcal: f64,
    /// 目标渐进着陆衰减系数（0.0 ~ 1.0）
    pub taper_factor: f64,
    /// 经过平稳着陆衰减后的理论热量调整值（kcal/天）
    pub tapered_adjustment_kcal: f64,
    /// 过去历史时间窗口（通常 14~30 天）实测 OLS 体重回归斜率换算的周速率（kg/周）
    pub actual_ols_weekly_rate_kg: Option<f64>,
    /// 实测变化速率与目标周速率的偏离差值（kg/周，实际减速慢于目标则为正）
    pub rate_discrepancy_kg: Option<f64>,
    /// 经阻尼处理后的闭环自适应修正量（kcal/天，受 ±250 kcal 约束）
    pub adaptive_correction_kcal: f64,
    /// 用户手动偏移值（kcal/天）
    pub manual_offset_kcal: f64,
    /// 未施加安全红线前的原始预算（kcal/天）
    pub unconstrained_budget_kcal: f64,
    /// 最终推荐每日摄入卡路里预算（受内分泌安全红线钳制，kcal/天）
    pub daily_budget_kcal: f64,
    /// 该用户生理表型所对应的最低安全摄入底线（kcal/天）
    pub safety_floor_kcal: f64,
    /// 是否触发了生理安全红线钳制
    pub safety_floor_triggered: bool,
    /// 推荐蛋白质目标（g/天，基于去脂体重或体重锚定）
    pub recommended_protein_g: f64,
    /// 推荐脂肪目标（g/天，保障内分泌底线）
    pub recommended_fat_g: f64,
    /// 推荐碳水化合物目标（g/天）
    pub recommended_carbs_g: f64,
    /// 临床诊断与安全指导提示列表
    pub clinical_notes: Vec<String>,
}

/// 动态自适应目标计算器核心引擎
pub struct GoalProfileEngine;

impl GoalProfileEngine {
    /// 体脂组织能量当量密度（约 7700 kcal/kg，分摊至 7 天约 1100 kcal/天/kg/周）
    pub const KCAL_PER_KG_WEEK_DAILY: f64 = 1100.0;
    /// 渐进平稳着陆起始触发距离（距离目标 5.0 kg 开始减速缓冲）
    pub const TAPER_START_DISTANCE_KG: f64 = 5.0;
    /// 渐进平稳着陆完全收敛距离（距离目标 1.0 kg 内完全转为维持热量）
    pub const TAPER_END_DISTANCE_KG: f64 = 1.0;
    /// 自适应反馈调节阻尼系数（避免短期水分扰动导致预算过度震荡）
    pub const ADAPTIVE_DAMPING_FACTOR: f64 = 0.5;
    /// 单周期自适应最大允许修正绝对值（kcal/天）
    pub const MAX_ADAPTIVE_CORRECTION_KCAL: f64 = 250.0;

    /// 计算指定周速率对应的理论每日能量缺口/盈余
    pub fn calculate_rate_adjustment(weekly_rate_kg: f64) -> f64 {
        weekly_rate_kg * Self::KCAL_PER_KG_WEEK_DAILY
    }

    /// 计算距目标体重的平稳着陆缓冲衰减因子 (0.0 ~ 1.0)
    pub fn calculate_taper_factor(
        current_weight_kg: f64,
        target_weight_kg: Option<f64>,
        kind: GoalKind,
        enabled: bool,
    ) -> f64 {
        if !enabled || kind == GoalKind::MaintainWeight {
            return 1.0;
        }
        let target = match target_weight_kg {
            Some(t) => t,
            None => return 1.0,
        };

        let signed_distance = target - current_weight_kg;
        // 减重：target < current，若 current <= target 则超额或达标 (signed_distance >= 0 表示还需减，<= 0 表示完成)
        // 增重：target > current，若 current >= target 则超额完成
        let reached_or_overshot = match kind {
            GoalKind::LoseWeight => signed_distance >= 0.0,
            GoalKind::GainWeight => signed_distance <= 0.0,
            GoalKind::MaintainWeight => true,
        };

        if reached_or_overshot {
            return 0.0;
        }

        let distance = signed_distance.abs();
        if distance <= Self::TAPER_END_DISTANCE_KG {
            0.0
        } else if distance >= Self::TAPER_START_DISTANCE_KG {
            1.0
        } else {
            let span = Self::TAPER_START_DISTANCE_KG - Self::TAPER_END_DISTANCE_KG;
            (distance - Self::TAPER_END_DISTANCE_KG) / span
        }
    }

    /// 获取受试者内分泌与代谢生理安全最低热量摄入底线
    /// - 男性 / 雄激素表型：≥ 1500 kcal
    /// - 女性 / 雌激素表型 / 均值表型：≥ 1200 kcal
    pub fn recommended_safety_floor(gender: &Gender) -> f64 {
        match gender {
            Gender::Male => 1500.0,
            Gender::NonBinary(HormoneProfile::TestosteroneTypical) => 1500.0,
            Gender::Female => 1200.0,
            Gender::NonBinary(HormoneProfile::EstrogenTypical) => 1200.0,
            Gender::NonBinary(HormoneProfile::Averaged) => 1200.0,
        }
    }

    /// 基于历史实测体重序列 `(NaiveDate, weight_kg)`，通过 OLS 回归拟合实际周变化速率
    pub fn fit_actual_weekly_rate(weight_points: &[(NaiveDate, f64)]) -> Option<f64> {
        if weight_points.len() < 2 {
            return None;
        }
        let first_date = weight_points[0].0;
        let n = weight_points.len() as f64;

        let xs: Vec<f64> = weight_points
            .iter()
            .map(|(d, _)| (*d - first_date).num_days() as f64)
            .collect();
        let ys: Vec<f64> = weight_points.iter().map(|(_, w)| *w).collect();

        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = ys.iter().sum();
        let sum_xy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
        let sum_xx: f64 = xs.iter().map(|x| x * x).sum();

        let denominator = n * sum_xx - sum_x * sum_x;
        if denominator.abs() < 1e-9 {
            return None;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        Some(slope * 7.0) // 转换为 kg/周
    }

    /// 全流程闭环计算自适应每日热量预算与宏量营养素配比
    ///
    /// 未成年人或妊娠/哺乳档案会返回 [`NutritionError`]，避免误用成人非孕 EER。
    pub fn compute_adaptive_budget(
        user: &UserProfile,
        goal: &GoalConfig,
        recent_weight_history: &[(NaiveDate, f64)],
    ) -> Result<AdaptiveBudgetResult, NutritionError> {
        let mut clinical_notes = Vec::new();

        // 1. 基准 TDEE 计算（遵循 NASEM 2023 DRI 方程）
        let base_tdee_kcal = EnergyCalc::nasem_2023(user)?.round();

        // 2. 原始速率热量调整
        let raw_rate_adjustment_kcal = match goal.kind {
            GoalKind::MaintainWeight => 0.0,
            GoalKind::LoseWeight => {
                // 校验减重速率临床合理性：推荐 -0.25 ~ -1.0 kg/周
                let mut rate = goal.weekly_rate_kg;
                if rate > 0.0 {
                    rate = -rate; // 容错：若传入正数则纠正为负
                }
                if rate < -1.0 {
                    clinical_notes.push(format!(
                        "警告：每周减重速率 {:.2} kg/周超过临床推荐上限 (-1.0 kg/周)，极易导致去脂体重流失与胆结石风险。",
                        rate
                    ));
                }
                Self::calculate_rate_adjustment(rate)
            }
            GoalKind::GainWeight => {
                let mut rate = goal.weekly_rate_kg;
                if rate < 0.0 {
                    rate = -rate; // 容错：若传入负数纠正为正
                }
                if rate > 0.5 {
                    clinical_notes.push(format!(
                        "提示：每周增重速率 {:.2} kg/周偏高，超过肌肉合成生理上限可能导致体脂过量堆积。",
                        rate
                    ));
                }
                Self::calculate_rate_adjustment(rate)
            }
        };

        // 3. 目标平稳着陆（Tapering）
        let taper_factor =
            Self::calculate_taper_factor(user.weight_kg, goal.target_weight_kg, goal.kind, goal.taper_enabled);
        let tapered_adjustment_kcal = (raw_rate_adjustment_kcal * taper_factor).round();
        if goal.taper_enabled && taper_factor < 1.0 && goal.kind != GoalKind::MaintainWeight {
            clinical_notes.push(format!(
                "平稳着陆缓冲激活：当前体重距离目标已在 {:.1} kg 内，赤字/盈余强度已收敛至 {:.0}%，助力顺畅过渡至维持期。",
                Self::TAPER_START_DISTANCE_KG,
                taper_factor * 100.0
            ));
        }

        // 4. 实测体重 OLS 回归斜率自适应控制
        let actual_ols_weekly_rate_kg = Self::fit_actual_weekly_rate(recent_weight_history);
        let mut adaptive_correction_kcal = 0.0;
        let mut rate_discrepancy_kg = None;

        if goal.adaptive_enabled
            && goal.kind != GoalKind::MaintainWeight
            && let Some(actual_rate) = actual_ols_weekly_rate_kg
        {
            let target_rate = if goal.kind == GoalKind::LoseWeight {
                -goal.weekly_rate_kg.abs()
            } else {
                goal.weekly_rate_kg.abs()
            };

            // discrepancy = 实际速率 - 目标速率
            // 例如减重目标 -0.5，实际 0.0 (平台期)，diff = +0.5 > 0
            let discrepancy = actual_rate - target_rate;
            rate_discrepancy_kg = Some(discrepancy);

            // 修正量：实际偏高（减得慢）需加大缺口（负向调整）；实际偏低（暴瘦）需调高摄入（正向调整）
            let raw_correction = -discrepancy * Self::KCAL_PER_KG_WEEK_DAILY * Self::ADAPTIVE_DAMPING_FACTOR;
            // 钳制在 [-250, +250] kcal
            adaptive_correction_kcal = raw_correction
                .clamp(-Self::MAX_ADAPTIVE_CORRECTION_KCAL, Self::MAX_ADAPTIVE_CORRECTION_KCAL)
                .round();

            if adaptive_correction_kcal.abs() >= 30.0 {
                if adaptive_correction_kcal < 0.0 {
                    clinical_notes.push(format!(
                            "自适应反馈：近期体重减速慢于预期（实测 {:.2} vs 目标 {:.2} kg/周），已自适应下调预算 {:.0} kcal 以突破代谢适应平台期。",
                            actual_rate, target_rate, adaptive_correction_kcal.abs()
                        ));
                } else {
                    clinical_notes.push(format!(
                            "自适应反馈：近期体重下降过快（实测 {:.2} vs 目标 {:.2} kg/周），为防止瘦体重过度消耗，已保护性上调预算 +{:.0} kcal。",
                            actual_rate, target_rate, adaptive_correction_kcal
                        ));
                }
            }
        }

        // 5. 汇总计算未受限预算
        let unconstrained_budget_kcal =
            base_tdee_kcal + tapered_adjustment_kcal + adaptive_correction_kcal + goal.manual_calorie_offset;

        // 6. 临床生理安全红线审查与钳制
        let safety_floor_kcal = Self::recommended_safety_floor(&user.gender);
        let (daily_budget_kcal, safety_floor_triggered) = if unconstrained_budget_kcal < safety_floor_kcal {
            clinical_notes.push(format!(
                "【内分泌安全红线报警】计算预算 ({:.0} kcal) 低于该生理表型安全底线 ({:.0} kcal)，已自动强制钳制在安全底线！请切勿过度节食以保护下丘脑与甲状腺轴，建议通过适度运动增加有效能耗。",
                unconstrained_budget_kcal, safety_floor_kcal
            ));
            (safety_floor_kcal, true)
        } else {
            (unconstrained_budget_kcal.round(), false)
        };

        // 7. 宏量素推荐分配（采用高蛋白均衡临床协议）
        let macro_plan = MacroEngine::calculate(daily_budget_kcal, user, None, DietProtocol::HighProteinBalanced);

        Ok(AdaptiveBudgetResult {
            base_tdee_kcal,
            raw_rate_adjustment_kcal,
            taper_factor,
            tapered_adjustment_kcal,
            actual_ols_weekly_rate_kg,
            rate_discrepancy_kg,
            adaptive_correction_kcal,
            manual_offset_kcal: goal.manual_calorie_offset,
            unconstrained_budget_kcal,
            daily_budget_kcal,
            safety_floor_kcal,
            safety_floor_triggered,
            recommended_protein_g: macro_plan.protein_g,
            recommended_fat_g: macro_plan.fat_g,
            recommended_carbs_g: macro_plan.carbs_g,
            clinical_notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::{ActivityLevel, Gender, UserProfile};

    fn make_test_user(gender: Gender, weight: f64) -> UserProfile {
        UserProfile::new(28, 175.0, weight, gender, ActivityLevel::LowActive).unwrap()
    }

    #[test]
    fn test_rate_adjustment_calculation() {
        assert_eq!(GoalProfileEngine::calculate_rate_adjustment(-0.5), -550.0);
        assert_eq!(GoalProfileEngine::calculate_rate_adjustment(-0.25), -275.0);
        assert_eq!(GoalProfileEngine::calculate_rate_adjustment(0.25), 275.0);
        assert_eq!(GoalProfileEngine::calculate_rate_adjustment(0.0), 0.0);
    }

    #[test]
    fn test_taper_factor_progression() {
        // 目标 70kg，减重场景
        let kind = GoalKind::LoseWeight;
        // 距离 6kg (> 5kg) -> 1.0
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(76.0, Some(70.0), kind, true),
            1.0
        );
        // 距离 3kg (中点) -> (3 - 1) / 4 = 0.5
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(73.0, Some(70.0), kind, true),
            0.5
        );
        // 距离 1kg -> 0.0
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(71.0, Some(70.0), kind, true),
            0.0
        );
        // 距离 0.5kg -> 0.0
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(70.5, Some(70.0), kind, true),
            0.0
        );
        // 达到目标 70kg -> 0.0
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(70.0, Some(70.0), kind, true),
            0.0
        );
        // 超额达成 69kg -> 0.0
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(69.0, Some(70.0), kind, true),
            0.0
        );

        // 未启用 taper
        assert_eq!(
            GoalProfileEngine::calculate_taper_factor(72.0, Some(70.0), kind, false),
            1.0
        );
    }

    #[test]
    fn test_safety_floor_trigger_and_clamping() {
        // 女性用户，基准 TDEE 较低，设定极度过激减重
        let female_user = make_test_user(Gender::Female, 50.0);
        let goal = GoalConfig {
            kind: GoalKind::LoseWeight,
            target_weight_kg: Some(45.0),
            weekly_rate_kg: -1.0, // -1100 kcal
            taper_enabled: false,
            adaptive_enabled: false,
            manual_calorie_offset: 0.0,
        };

        let result = GoalProfileEngine::compute_adaptive_budget(&female_user, &goal, &[]).unwrap();
        assert!(result.unconstrained_budget_kcal < 1200.0);
        assert_eq!(result.daily_budget_kcal, 1200.0);
        assert!(result.safety_floor_triggered);
        assert!(result.clinical_notes.iter().any(|n| n.contains("内分泌安全红线报警")));
    }

    #[test]
    fn test_adaptive_feedback_on_plateau() {
        let male_user = make_test_user(Gender::Male, 85.0);
        let goal = GoalConfig {
            kind: GoalKind::LoseWeight,
            target_weight_kg: Some(75.0),
            weekly_rate_kg: -0.5,
            taper_enabled: false,
            adaptive_enabled: true,
            manual_calorie_offset: 0.0,
        };

        // 模拟过去 14 天体重完全无变化（体重停滞 85.0 kg，实测斜率 0.0 kg/周）
        let d0 = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let history = vec![
            (d0, 85.0),
            (d0 + chrono::Duration::days(7), 85.0),
            (d0 + chrono::Duration::days(14), 85.0),
        ];

        let result = GoalProfileEngine::compute_adaptive_budget(&male_user, &goal, &history).unwrap();
        assert_eq!(result.actual_ols_weekly_rate_kg, Some(0.0));
        // 目标 -0.5，实际 0.0，discrepancy = +0.5
        // raw_correction = -0.5 * 1100 * 0.5 = -275 -> clamped to -250
        assert_eq!(result.adaptive_correction_kcal, -250.0);
        assert!(result.clinical_notes.iter().any(|n| n.contains("自适应下调预算")));
    }

    #[test]
    fn test_gain_weight_scenario() {
        let male_user = make_test_user(Gender::Male, 70.0);
        let goal = GoalConfig {
            kind: GoalKind::GainWeight,
            target_weight_kg: Some(75.0),
            weekly_rate_kg: 0.25, // +275 kcal
            taper_enabled: true,
            adaptive_enabled: false,
            manual_calorie_offset: 50.0,
        };

        let result = GoalProfileEngine::compute_adaptive_budget(&male_user, &goal, &[]).unwrap();
        assert_eq!(result.raw_rate_adjustment_kcal, 275.0);
        assert_eq!(result.manual_offset_kcal, 50.0);
        assert!(result.daily_budget_kcal > result.base_tdee_kcal);
        assert!(!result.safety_floor_triggered);
    }
}
