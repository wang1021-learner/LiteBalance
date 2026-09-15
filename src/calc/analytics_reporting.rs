use serde::{Deserialize, Serialize};

/// 周期内能量平衡与体重秤轨迹分析统计
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyAnalytics {
    /// 统计窗口总天数（例如 7 天、14 天、30 天）
    pub total_days: usize,
    /// 实际记录了饮食日志的有效天数
    pub logged_days: usize,
    /// 记录依从率（已记录天数 / 总天数 * 100%）
    pub logging_adherence_pct: f64,
    /// 平均每日热量摄入（kcal/天）
    pub avg_daily_intake_kcal: f64,
    /// 设定的每日摄入目标（kcal/天）
    pub avg_daily_target_kcal: f64,
    /// 用户的每日总能量消耗预估（TDEE，kcal/天）
    pub avg_daily_tdee_kcal: f64,
    /// 窗口内累计热量缺口 / 盈余（累计 TDEE - 累计摄入，kcal）
    pub cumulative_caloric_deficit: f64,
    /// 理论预期体重变化（千克，按 7700 kcal/kg 人体脂肪/瘦体重等价积分换算）
    pub theoretical_weight_change_kg: f64,
    /// 体重秤实测变化（千克，通过体重测量记录的最小二乘法 OLS 线性回归斜率计算）
    pub actual_weight_change_kg: Option<f64>,
    /// 【核心临床指标】代谢偏离度（千克，实际变化 - 理论预期变化）
    /// - 若为正数（如预期减 1.0kg，实减 0.2kg）：提示代谢适应、肌糖原水肿阻滞或有未计入的隐形油脂摄入；
    /// - 若为负数（如预期减 1.0kg，实减 1.8kg）：提示早期糖原水分大量流失或非运动日常消耗 NEAT 增高。
    pub metabolic_divergence_kg: Option<f64>,
}

/// 宏量营养素三大供能比与蛋白质达标分析
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroAnalytics {
    /// 平均每日蛋白质实际摄入量（g）
    pub avg_daily_protein_g: f64,
    /// 平均每日碳水化合物实际摄入量（g）
    pub avg_daily_carbs_g: f64,
    /// 平均每日脂肪实际摄入量（g）
    pub avg_daily_fat_g: f64,
    /// 实际蛋白质能量供能比（%）
    pub actual_protein_energy_pct: f64,
    /// 实际碳水化合物能量供能比（%）
    pub actual_carbs_energy_pct: f64,
    /// 实际脂肪能量供能比（%）
    pub actual_fat_energy_pct: f64,
    /// 锚定瘦体重（FFM）的目标蛋白质摄入量（g）
    pub target_protein_g: f64,
    /// 达到目标蛋白质 90% 以上的天数
    pub protein_compliance_days: usize,
    /// 蛋白质摄入充分性达标率（%）
    pub protein_compliance_rate_pct: f64,
}

/// 间歇性断食与水分摄入等行为习惯依从率分析
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitAnalytics {
    /// 窗口内尝试断食的总次数
    pub fasting_sessions_count: usize,
    /// 达成预定目标时长的断食完成次数
    pub fasting_completed_count: usize,
    /// 断食完成依从率（%）
    pub fasting_compliance_rate_pct: f64,
    /// 平均单次断食实际持续时长（小时）
    pub avg_fasting_duration_hours: f64,
    /// 平均每日饮水量（毫升/天）
    pub avg_daily_water_ml: f64,
    /// 临床推荐饮水目标（毫升/天）
    pub target_water_ml: u32,
    /// 饮水达标天数
    pub water_compliance_days: usize,
    /// 饮水达标依从率（%）
    pub water_compliance_rate_pct: f64,
}

/// 周期性代谢与营养综合评估总报告
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriodicReport {
    /// 周期起始日期
    pub start_date: String,
    /// 周期截止日期
    pub end_date: String,
    /// 统计窗口天数
    pub period_days: usize,
    /// 能量平衡与体重轨迹维度
    pub energy: EnergyAnalytics,
    /// 宏量营养素分配与蛋白质保护维度
    pub macros: MacroAnalytics,
    /// 断食与饮水习惯维度
    pub habits: HabitAnalytics,
    /// 临床观察与可执行调优建议
    pub insights: Vec<String>,
}

/// 单日饮食摄入数据输入结构体
#[derive(Debug, Clone)]
pub struct DailyIntakeData {
    pub date: String,
    pub intake_kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
}

/// 单日体重测量输入（用于最小二乘法线性回归消除短期体液波动）
#[derive(Debug, Clone)]
pub struct DailyWeightData {
    pub day_offset: f64, // 距起始日的天数偏移量（如 0.0, 1.0, 2.5）
    pub weight_kg: f64,
}

/// 断食记录输入结构体
#[derive(Debug, Clone)]
pub struct FastingSessionData {
    pub duration_hours: f64,
    pub target_hours: f64,
    pub is_completed: bool,
}

/// 周期性营养分析与代谢偏离度计算引擎
pub struct PeriodicAnalyticsEngine;

impl PeriodicAnalyticsEngine {
    /// 权威热量-人体组织转换常数：7700 kcal 对应 1kg 人体体重变化
    pub const KCAL_PER_KG_BODY_TISSUE: f64 = 7700.0;

    /// 聚合指定时间窗口内的全部日志，生成周期性代谢报告
    #[allow(clippy::too_many_arguments)]
    pub fn generate_report(
        start_date: &str,
        end_date: &str,
        period_days: usize,
        target_tdee_kcal: f64,
        target_intake_kcal: f64,
        target_protein_g: f64,
        target_water_ml: u32,
        daily_intakes: &[DailyIntakeData],
        weight_logs: &[DailyWeightData],
        daily_water_logs: &[(String, u32)], // (日期, 累计饮水量ml)
        fasting_sessions: &[FastingSessionData],
    ) -> PeriodicReport {
        let logged_days = daily_intakes.len();
        let total_days = period_days.max(1);
        let logging_adherence_pct = (logged_days as f64 / total_days as f64 * 100.0).min(100.0);

        // 1. 能量与赤字计算
        let total_intake_kcal: f64 = daily_intakes.iter().map(|d| d.intake_kcal).sum();
        let avg_daily_intake_kcal = if logged_days > 0 {
            total_intake_kcal / logged_days as f64
        } else {
            0.0
        };

        let cumulative_tdee = target_tdee_kcal * total_days as f64;
        let cumulative_intake = total_intake_kcal;
        let cumulative_caloric_deficit = cumulative_tdee - cumulative_intake;
        // 正赤字对应减重：Delta kg = -deficit / 7700
        let theoretical_weight_change_kg = -(cumulative_caloric_deficit / Self::KCAL_PER_KG_BODY_TISSUE);

        // 基于最小二乘法（OLS）线性回归计算体重秤的实际变动趋势（过滤每日水钠波动）
        let actual_weight_change_kg = if weight_logs.len() >= 2 {
            let n = weight_logs.len() as f64;
            let sum_x: f64 = weight_logs.iter().map(|w| w.day_offset).sum();
            let sum_y: f64 = weight_logs.iter().map(|w| w.weight_kg).sum();
            let sum_xy: f64 = weight_logs.iter().map(|w| w.day_offset * w.weight_kg).sum();
            let sum_xx: f64 = weight_logs.iter().map(|w| w.day_offset * w.day_offset).sum();

            let denom = n * sum_xx - sum_x * sum_x;
            if denom.abs() > 1e-6 {
                let slope = (n * sum_xy - sum_x * sum_y) / denom;
                Some(slope * (total_days as f64 - 1.0))
            } else {
                None
            }
        } else {
            None
        };

        // 代谢偏离度 = 实际体重变动 - 理论预期变动
        let metabolic_divergence_kg = actual_weight_change_kg.map(|actual| actual - theoretical_weight_change_kg);

        let energy = EnergyAnalytics {
            total_days,
            logged_days,
            logging_adherence_pct,
            avg_daily_intake_kcal,
            avg_daily_target_kcal: target_intake_kcal,
            avg_daily_tdee_kcal: target_tdee_kcal,
            cumulative_caloric_deficit,
            theoretical_weight_change_kg,
            actual_weight_change_kg,
            metabolic_divergence_kg,
        };

        // 2. 宏量营养素统计
        let total_protein: f64 = daily_intakes.iter().map(|d| d.protein_g).sum();
        let total_carbs: f64 = daily_intakes.iter().map(|d| d.carbs_g).sum();
        let total_fat: f64 = daily_intakes.iter().map(|d| d.fat_g).sum();

        let avg_daily_protein_g = if logged_days > 0 { total_protein / logged_days as f64 } else { 0.0 };
        let avg_daily_carbs_g = if logged_days > 0 { total_carbs / logged_days as f64 } else { 0.0 };
        let avg_daily_fat_g = if logged_days > 0 { total_fat / logged_days as f64 } else { 0.0 };

        let total_macro_kcal = (total_protein * 4.0) + (total_carbs * 4.0) + (total_fat * 9.0);
        let (p_pct, c_pct, f_pct) = if total_macro_kcal > 0.0 {
            (
                (total_protein * 4.0) / total_macro_kcal * 100.0,
                (total_carbs * 4.0) / total_macro_kcal * 100.0,
                (total_fat * 9.0) / total_macro_kcal * 100.0,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        let protein_compliance_days = daily_intakes
            .iter()
            .filter(|d| d.protein_g >= target_protein_g * 0.90)
            .count();

        let protein_compliance_rate_pct = if logged_days > 0 {
            (protein_compliance_days as f64 / logged_days as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let macros = MacroAnalytics {
            avg_daily_protein_g,
            avg_daily_carbs_g,
            avg_daily_fat_g,
            actual_protein_energy_pct: p_pct,
            actual_carbs_energy_pct: c_pct,
            actual_fat_energy_pct: f_pct,
            target_protein_g,
            protein_compliance_days,
            protein_compliance_rate_pct,
        };

        // 3. 断食与饮水习惯统计
        let fasting_sessions_count = fasting_sessions.len();
        let fasting_completed_count = fasting_sessions.iter().filter(|f| f.is_completed).count();
        let fasting_compliance_rate_pct = if fasting_sessions_count > 0 {
            (fasting_completed_count as f64 / fasting_sessions_count as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let total_fasting_hours: f64 = fasting_sessions.iter().map(|f| f.duration_hours).sum();
        let avg_fasting_duration_hours = if fasting_sessions_count > 0 {
            total_fasting_hours / fasting_sessions_count as f64
        } else {
            0.0
        };

        let total_water_ml: u32 = daily_water_logs.iter().map(|(_, w)| *w).sum();
        let avg_daily_water_ml = if !daily_water_logs.is_empty() {
            total_water_ml as f64 / daily_water_logs.len() as f64
        } else {
            0.0
        };

        let water_compliance_days = daily_water_logs
            .iter()
            .filter(|(_, w)| *w >= target_water_ml)
            .count();

        let water_compliance_rate_pct = if !daily_water_logs.is_empty() {
            (water_compliance_days as f64 / daily_water_logs.len() as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let habits = HabitAnalytics {
            fasting_sessions_count,
            fasting_completed_count,
            fasting_compliance_rate_pct,
            avg_fasting_duration_hours,
            avg_daily_water_ml,
            target_water_ml,
            water_compliance_days,
            water_compliance_rate_pct,
        };

        // 4. 临床智能观察与结论生成
        let mut insights = Vec::new();

        // 记录连续性建议
        if logging_adherence_pct >= 85.0 {
            insights.push(format!(
                "记录依从性极高：累计记录 {}/{} 天（{:.1}%），高置信度的数据为精准代谢建模提供了可靠基础。",
                logged_days, total_days, logging_adherence_pct
            ));
        } else {
            insights.push(format!(
                "记录依从性为 {:.1}%（{}/{} 天）。部分未记录的天数会降低能量平衡预测的准确性。",
                logging_adherence_pct, logged_days, total_days
            ));
        }

        // 体重偏离度分析
        if let Some(actual_change) = actual_weight_change_kg {
            let div = actual_change - theoretical_weight_change_kg;
            if div.abs() <= 0.3 {
                insights.push(format!(
                    "体重秤与理论模型高度收敛：实际变化 ({:+.2} kg) 与能量缺口预测 ({:+.2} kg) 高度吻合。",
                    actual_change, theoretical_weight_change_kg
                ));
            } else if div > 0.3 {
                insights.push(format!(
                    "体重下降慢于理论预期 +{:.2} kg（实际 {:+.2} kg vs 理论 {:+.2} kg）。常见诱因：高钠摄入引起体液潴留、新力量训练后肌肉微损伤炎症存水、或烹饪食用油存在未记录误差。",
                    div, actual_change, theoretical_weight_change_kg
                ));
            } else {
                insights.push(format!(
                    "减重速率快于理论预期 {:.2} kg（实际 {:+.2} kg vs 理论 {:+.2} kg）。多见于减脂初期糖原结合水的大幅释放，或非运动日常活动量 (NEAT) 的自发提升。",
                    div.abs(), actual_change, theoretical_weight_change_kg
                ));
            }
        }

        // 蛋白质摄入与瘦体重保护
        if protein_compliance_rate_pct >= 80.0 {
            insights.push(format!(
                "蛋白质达标表现优秀：{}/{} 天达标（{:.1}%）。日均摄入 {:.1}g 蛋白质，在热量赤字期为瘦体重（FFM）提供了坚固防护。",
                protein_compliance_days, logged_days, protein_compliance_rate_pct, avg_daily_protein_g
            ));
        } else {
            insights.push(format!(
                "蛋白质达标天数仅为 {}/{} 天（{:.1}%）。建议将日均蛋白质提升至目标 {:.1}g，以减少肌肉流失并提升饱腹感。",
                protein_compliance_days, logged_days, protein_compliance_rate_pct, target_protein_g
            ));
        }

        // 行为习惯观察
        if habits.fasting_sessions_count > 0 {
            insights.push(format!(
                "间歇性断食执行情况：完成 {}/{} 次断食（依从率 {:.1}%），单次平均维持 {:.1} 小时。",
                habits.fasting_completed_count, habits.fasting_sessions_count, habits.fasting_compliance_rate_pct, habits.avg_fasting_duration_hours
            ));
        }

        if habits.avg_daily_water_ml >= target_water_ml as f64 {
            insights.push(format!(
                "饮水达标：日均饮水 {:.0} ml（目标 {} ml），有效保障了基础代谢运转。",
                habits.avg_daily_water_ml, target_water_ml
            ));
        }

        PeriodicReport {
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            period_days,
            energy,
            macros,
            habits,
            insights,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metabolic_divergence_calculation() {
        // 14 天热量赤字情景验证：
        // TDEE = 2500 kcal/天，摄入 = 2000 kcal/天（每日缺口 500 kcal）
        // 14 天累计缺口 = 7000 kcal
        // 理论减重 = -7000 / 7700 = -0.909 kg
        let mut daily_intakes = Vec::new();
        for i in 0..14 {
            daily_intakes.push(DailyIntakeData {
                date: format!("2026-05-{:02}", i + 1),
                intake_kcal: 2000.0,
                protein_g: 150.0,
                carbs_g: 200.0,
                fat_g: 66.6,
            });
        }

        // 体重记录：第 0 天为 80.0kg，第 13 天为 79.1kg（实际减重 -0.9kg）
        let weight_logs = vec![
            DailyWeightData { day_offset: 0.0, weight_kg: 80.0 },
            DailyWeightData { day_offset: 13.0, weight_kg: 79.1 },
        ];

        let report = PeriodicAnalyticsEngine::generate_report(
            "2026-05-01",
            "2026-05-14",
            14,
            2500.0,
            2000.0,
            150.0,
            2500,
            &daily_intakes,
            &weight_logs,
            &[],
            &[],
        );

        assert_eq!(report.energy.logged_days, 14);
        assert!((report.energy.cumulative_caloric_deficit - 7000.0).abs() < 1e-2);
        assert!((report.energy.theoretical_weight_change_kg - (-0.909)).abs() < 1e-2);

        let actual = report.energy.actual_weight_change_kg.unwrap();
        assert!((actual - (-0.9)).abs() < 1e-2);

        let divergence = report.energy.metabolic_divergence_kg.unwrap();
        assert!(divergence.abs() < 0.05); // 理论与实测紧密收敛

        assert!(report.insights.iter().any(|s| s.contains("高度收敛")));
    }

    #[test]
    fn test_macro_split_and_protein_compliance() {
        let daily_intakes = vec![
            DailyIntakeData {
                date: "2026-05-01".into(),
                intake_kcal: 2000.0,
                protein_g: 150.0, // 600 kcal (30%)
                carbs_g: 225.0,   // 900 kcal (45%)
                fat_g: 55.5,      // 500 kcal (25%)
            },
        ];

        let report = PeriodicAnalyticsEngine::generate_report(
            "2026-05-01",
            "2026-05-01",
            1,
            2000.0,
            2000.0,
            140.0,
            2000,
            &daily_intakes,
            &[],
            &[],
            &[],
        );

        assert!((report.macros.actual_protein_energy_pct - 30.0).abs() < 1.0);
        assert!((report.macros.actual_carbs_energy_pct - 45.0).abs() < 1.0);
        assert!((report.macros.actual_fat_energy_pct - 25.0).abs() < 1.0);
        assert_eq!(report.macros.protein_compliance_days, 1);
        assert_eq!(report.macros.protein_compliance_rate_pct, 100.0);
    }
}
