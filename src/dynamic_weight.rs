use crate::energy::EnergyCalc;
use crate::user::{Gender, UserProfile};
use serde::{Deserialize, Serialize};

/// 美国国立卫生研究院（NIH）Kevin Hall 动态能量平衡模型（Lancet 2011）生理学常数
pub mod constants {
    /// 脂肪组织（Adipose Tissue / FM）能量密度：约 9400 kcal/kg
    pub const RHO_FM: f64 = 9400.0;
    /// 瘦体重组织（Fat-Free Mass / FFM）能量密度：约 1800 kcal/kg
    pub const RHO_FFM: f64 = 1800.0;
    /// Forbes 脂肪与瘦体重分配参数常数：约 10.4 kg
    pub const FORBES_C: f64 = 10.4;
    /// 瘦体重基础代谢系数（Resting Metabolic Rate）：约 22.0 kcal/kg/天
    pub const GAMMA_FFM: f64 = 22.0;
    /// 脂肪组织基础代谢系数：约 3.2 kcal/kg/天
    pub const GAMMA_FM: f64 = 3.2;
    /// 食物热效应（TEF）系数：约占摄入能量变化的 10%
    pub const BETA_TEF: f64 = 0.10;
    /// 适应性产热（Adaptive Thermogenesis）减慢系数：持续能量缺口的约 14%
    pub const XI_AT: f64 = 0.14;
    /// 适应性产热响应半衰期时间常数：约 14 天
    pub const TAU_AT: f64 = 14.0;
    /// 传统 Wishnofsky 线性 3500kcal/lb 经验转换常数：7700 kcal/kg（用于对照对比）
    pub const WISHNOFSKY_KCAL_PER_KG: f64 = 7700.0;
}

/// 每日动态代谢轨迹采样点
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrajectoryDay {
    /// 模拟天数索引（第 0, 1, 2... 天）
    pub day: usize,
    /// 当日总体重（kg）
    pub weight_kg: f64,
    /// 当日脂肪质量（kg）
    pub fat_mass_kg: f64,
    /// 当日去脂瘦体重（kg）
    pub fat_free_mass_kg: f64,
    /// 当日体脂率（%）
    pub body_fat_pct: f64,
    /// 当日总能量消耗 TDEE（随体重与适应性产热动态下降，kcal）
    pub expenditure_kcal: f64,
    /// 当日设定能量摄入（kcal）
    pub intake_kcal: f64,
    /// 当日实际能量缺口（kcal）
    pub deficit_kcal: f64,
    /// 传统静态 Wishnofsky 线性模型预测体重（用于直观展现“平台期减速”与线性模型的偏差）
    pub wishnofsky_weight_kg: f64,
}

/// Kevin Hall 动态能量平衡仿真全面输出结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicSimulationResult {
    /// 初始起始体重（kg）
    pub initial_weight_kg: f64,
    /// 目标体重（kg，可选）
    pub target_weight_kg: Option<f64>,
    /// 每日热量摄入设定（kcal/天）
    pub daily_intake_kcal: f64,
    /// 起始平衡维持热量（kcal/天）
    pub initial_maintenance_kcal: f64,
    /// 模拟周期结束时的最终体重（kg）
    pub final_weight_kg: f64,
    /// 【核心临床概念】代谢平台期渐近极限体重（若长期维持该摄入量，最终平衡达到的极限平台体重，kg）
    pub plateau_weight_kg: f64,
    /// 达到目标体重所需实际天数（若当前热量无法减至该目标则为 None）
    pub days_to_target: Option<usize>,
    /// 累计体重变化总量（kg）
    pub total_weight_change_kg: f64,
    /// 累计纯脂肪质量变化量（kg）
    pub fat_mass_change_kg: f64,
    /// 累计瘦体重质量变化量（kg）
    pub fat_free_mass_change_kg: f64,
    /// 适应性代谢减慢总量（初始消耗 - 最终消耗，体现机体节能反应，kcal/天）
    pub metabolic_adaptation_kcal: f64,
    /// 传统 Wishnofsky 线性公式预测的终点体重（kg）
    pub wishnofsky_final_weight_kg: f64,
    /// 传统线性公式过度高估的减重公斤数（展现非线性动态模型的临床必要性）
    pub wishnofsky_overestimate_kg: f64,
    /// 逐日演变全景时间轴
    pub timeline: Vec<TrajectoryDay>,
}

/// 基于 Kevin Hall 模型的动态体重规划器
pub struct DynamicWeightPlanner;

impl DynamicWeightPlanner {
    /// 基于 BMI、年龄和性别估算初始体脂率（Deurenberg/Gallagher 公式，若用户未测量体脂秤时使用）
    pub fn estimate_body_fat_pct(bmi: f64, age: u8, gender: Gender) -> f64 {
        let sex_factor = match gender {
            Gender::Male => 1.0,
            Gender::Female => 0.0,
            Gender::NonBinary(_) => 0.5,
        };
        // 1.20 * BMI + 0.23 * Age - 10.8 * sex - 5.4
        let bf = 1.20 * bmi + 0.23 * (age as f64) - 10.8 * sex_factor - 5.4;
        bf.clamp(5.0, 60.0)
    }

    /// 执行动态体重与代谢微分方程仿真模拟
    ///
    /// # 参数说明
    /// * `profile` - 用户生理属性档案
    /// * `initial_body_fat_pct` - 用户测量体脂率（可选）
    /// * `daily_intake_kcal` - 规划每日摄入卡路里（如 2000.0 kcal）
    /// * `simulation_days` - 模拟时长天数（如 90 天、180 天）
    /// * `target_weight_kg` - 期望达成的目标体重（可选）
    pub fn simulate(
        profile: &UserProfile,
        initial_body_fat_pct: Option<f64>,
        daily_intake_kcal: f64,
        simulation_days: usize,
        target_weight_kg: Option<f64>,
    ) -> DynamicSimulationResult {
        use constants::*;

        let initial_weight = profile.weight_kg;
        let bf_pct = initial_body_fat_pct
            .unwrap_or_else(|| Self::estimate_body_fat_pct(profile.bmi(), profile.age, profile.gender));

        let initial_fm = initial_weight * (bf_pct / 100.0);
        let initial_ffm = initial_weight - initial_fm;

        // 稳态初始维持总代谢消耗（基于 NASEM 2023）
        let ee_0 = EnergyCalc::nasem_2023(profile);
        let delta_ei = daily_intake_kcal - ee_0;

        // 拆解静息代谢率 RMR 与活动产热 PAE 基础组分
        let rmr_0 = GAMMA_FFM * initial_ffm + GAMMA_FM * initial_fm;
        let tef_0 = BETA_TEF * ee_0;
        let pae_0 = (ee_0 - rmr_0 - tef_0).max(0.0);
        let delta_activity_factor = if initial_weight > 0.0 {
            pae_0 / initial_weight
        } else {
            0.0
        };

        let mut current_fm = initial_fm;
        let mut current_ffm = initial_ffm;
        let mut adaptive_thermogenesis = 0.0;
        let mut days_to_target: Option<usize> = None;

        let mut timeline = Vec::with_capacity(simulation_days + 1);

        // 第 0 天基线
        timeline.push(TrajectoryDay {
            day: 0,
            weight_kg: initial_weight,
            fat_mass_kg: initial_fm,
            fat_free_mass_kg: initial_ffm,
            body_fat_pct: bf_pct,
            expenditure_kcal: ee_0,
            intake_kcal: daily_intake_kcal,
            deficit_kcal: ee_0 - daily_intake_kcal,
            wishnofsky_weight_kg: initial_weight,
        });

        let dt = 1.0; // 积分步长：1 天

        for day in 1..=simulation_days {
            let current_weight = current_fm + current_ffm;

            // 1. 计算动态能量消耗分量（静息代谢、活动消耗、食物热效应）
            let delta_rmr = GAMMA_FFM * (current_ffm - initial_ffm) + GAMMA_FM * (current_fm - initial_fm);
            let delta_pae = delta_activity_factor * (current_weight - initial_weight);
            let delta_tef = BETA_TEF * delta_ei;

            // 更新适应性产热（微分方程：d(AT)/dt = (XI * delta_ei - AT) / TAU）
            adaptive_thermogenesis += (dt / TAU_AT) * (XI_AT * delta_ei - adaptive_thermogenesis);

            let current_ee = (ee_0 + delta_rmr + delta_pae + delta_tef + adaptive_thermogenesis).max(800.0);
            let energy_balance = daily_intake_kcal - current_ee;

            // 2. Forbes 脂肪与瘦组织分配比例积分
            let effective_rho = RHO_FM + RHO_FFM * (FORBES_C / current_fm.max(1.0));

            // 脂肪与瘦体重变化速率
            let d_fm = (energy_balance / effective_rho) * dt;
            let d_ffm = (FORBES_C / current_fm.max(1.0)) * d_fm;

            current_fm += d_fm;
            current_ffm += d_ffm;
            let next_weight = current_fm + current_ffm;

            // 传统 Wishnofsky 线性模型对照
            let wishnofsky_weight = initial_weight + (delta_ei * (day as f64)) / WISHNOFSKY_KCAL_PER_KG;

            // 检测目标达成节点
            if let Some(target) = target_weight_kg
                && days_to_target.is_none()
                && ((initial_weight >= target && next_weight <= target)
                    || (initial_weight <= target && next_weight >= target))
            {
                days_to_target = Some(day);
            }

            timeline.push(TrajectoryDay {
                day,
                weight_kg: (next_weight * 100.0).round() / 100.0,
                fat_mass_kg: (current_fm * 100.0).round() / 100.0,
                fat_free_mass_kg: (current_ffm * 100.0).round() / 100.0,
                body_fat_pct: ((current_fm / next_weight.max(1.0)) * 1000.0).round() / 10.0,
                expenditure_kcal: (current_ee * 10.0).round() / 10.0,
                intake_kcal: daily_intake_kcal,
                deficit_kcal: ((current_ee - daily_intake_kcal) * 10.0).round() / 10.0,
                wishnofsky_weight_kg: (wishnofsky_weight * 100.0).round() / 100.0,
            });
        }

        let final_day = timeline.last().unwrap();
        let wishnofsky_final = initial_weight + (delta_ei * (simulation_days as f64)) / WISHNOFSKY_KCAL_PER_KG;

        // 理论稳态极限平台期：
        // 在极限平台期，代谢下降刚好抵消热量赤字：~22 kcal/天/kg 稳态斜率
        let plateau_delta_weight = delta_ei / 22.0;
        let plateau_weight = (initial_weight + plateau_delta_weight).max(30.0);

        let wishnofsky_overestimate = if delta_ei < 0.0 {
            (final_day.weight_kg - wishnofsky_final).max(0.0)
        } else {
            (wishnofsky_final - final_day.weight_kg).max(0.0)
        };

        DynamicSimulationResult {
            initial_weight_kg: initial_weight,
            target_weight_kg,
            daily_intake_kcal,
            initial_maintenance_kcal: (ee_0 * 10.0).round() / 10.0,
            final_weight_kg: final_day.weight_kg,
            plateau_weight_kg: (plateau_weight * 10.0).round() / 10.0,
            days_to_target,
            total_weight_change_kg: ((final_day.weight_kg - initial_weight) * 100.0).round() / 100.0,
            fat_mass_change_kg: ((final_day.fat_mass_kg - initial_fm) * 100.0).round() / 100.0,
            fat_free_mass_change_kg: ((final_day.fat_free_mass_kg - initial_ffm) * 100.0).round() / 100.0,
            metabolic_adaptation_kcal: ((ee_0 - final_day.expenditure_kcal) * 10.0).round() / 10.0,
            wishnofsky_final_weight_kg: (wishnofsky_final * 100.0).round() / 100.0,
            wishnofsky_overestimate_kg: (wishnofsky_overestimate * 100.0).round() / 100.0,
            timeline,
        }
    }

    /// 反向目标求解器：使用二分根查找算法，计算在指定天数 `target_days` 内精准达到目标体重所需的每日摄入热量。
    ///
    /// # 生物学与数值精度说明：
    /// 数值收敛精度为 0.05kg（50克），而在真实生理中：
    /// 1. 人体自主饮食打卡通常存在 20%~40% 的低估漏记偏见；
    /// 2. 水钠潴留与肌糖原存储会造成每日 ±1~2kg 的自然生理波动；
    /// 3. 本数值模型提供高置信度的科学基准线，用于指导个性化饮食规划。
    pub fn solve_target_intake(
        profile: &UserProfile,
        initial_body_fat_pct: Option<f64>,
        target_weight_kg: f64,
        target_days: usize,
    ) -> Result<f64, String> {
        if target_days == 0 {
            return Err("目标天数必须大于 0".to_string());
        }

        let maintenance = EnergyCalc::nasem_2023(profile);
        let mut low = 500.0;
        let mut high = (maintenance * 2.0).max(5000.0);
        let tolerance = 0.05; // 50 克收敛容差

        for _ in 0..50 {
            let mid = (low + high) / 2.0;
            let sim = Self::simulate(profile, initial_body_fat_pct, mid, target_days, Some(target_weight_kg));

            let diff = sim.final_weight_kg - target_weight_kg;

            if diff.abs() <= tolerance {
                return Ok((mid * 10.0).round() / 10.0);
            }

            if target_weight_kg < profile.weight_kg {
                // 减重方向：若模拟体重偏高，说明摄入过多，需要降低上限
                if diff > 0.0 {
                    high = mid;
                } else {
                    low = mid;
                }
            } else {
                // 增重方向：若模拟体重偏高，需要降低摄入
                if diff > 0.0 {
                    high = mid;
                } else {
                    low = mid;
                }
            }
        }

        let final_guess = (low + high) / 2.0;
        Ok((final_guess * 10.0).round() / 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::{ActivityLevel, Gender};

    #[test]
    fn test_dynamic_weight_loss_plateau() {
        // 成年男性：30岁，175cm，90kg，久坐
        let profile = UserProfile::new(30, 175.0, 90.0, Gender::Male, ActivityLevel::Inactive).unwrap();

        // 初始基线平衡消耗约 2800 kcal
        let maintenance = EnergyCalc::nasem_2023(&profile);
        // 施加每日 500 kcal 赤字
        let daily_intake = maintenance - 500.0;

        let sim = DynamicWeightPlanner::simulate(&profile, Some(28.0), daily_intake, 365, Some(75.0));

        assert!(sim.final_weight_kg < 90.0);
        assert!(sim.metabolic_adaptation_kcal > 50.0);
        // 传统线性公式严重高估减重成果（因为忽视了基础代谢随体重下降而自发缩减）
        assert!(sim.wishnofsky_overestimate_kg > 2.0);
    }

    #[test]
    fn test_reverse_solver_calculates_realistic_intake() {
        // 30岁男性，175cm，85kg，积极活动，目标在 180 天内减至 78kg
        let profile = UserProfile::new(30, 175.0, 85.0, Gender::Male, ActivityLevel::Active).unwrap();

        let target_weight = 78.0;
        let days = 180;

        let intake = DynamicWeightPlanner::solve_target_intake(&profile, None, target_weight, days).unwrap();

        // 验证反解摄入量在正向模拟后能精准收敛至 78kg
        let sim = DynamicWeightPlanner::simulate(&profile, None, intake, days, Some(target_weight));
        assert!((sim.final_weight_kg - target_weight).abs() < 0.1);
    }
}
