use serde::{Deserialize, Serialize};
use crate::models::UserProfile;

/// 饮食流派哲学 / 三大宏量营养素分配方案。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DietProtocol {
    /// 高蛋白均衡饮食（循证运动营养学 / 增肌与保肌减脂）：
    /// - 锚定去脂体重（FFM）：2.0 - 2.6 g/kg FFM（或 1.8 - 2.2 g/kg 体重）。
    /// - 脂肪：占总能量 25-30%（且不低于 0.7 g/kg 体重以保障内分泌平衡）。
    /// - 碳水化合物：填充剩余热量，为肌糖原储备和高强度训练供能。
    HighProteinBalanced,

    /// 生酮饮食 / 极低碳水化合物方案（Keto）：
    /// - 碳水化合物：严格限制上限（25 - 30g 净碳水/天）。
    /// - 蛋白质：适度偏高（1.6 - 2.0 g/kg FFM，兼顾保肌与防止过量糖异生）。
    /// - 脂肪：占总热量 70-75%（作为机体主要生酮与供能底物）。
    Ketogenic,

    /// 地中海饮食 / 长寿心血管健康方案：
    /// - 强调优质不饱和脂肪酸：占总热量 35-40%。
    /// - 适度蛋白质：1.4 - 1.8 g/kg 体重。
    /// - 复合碳水化合物与高膳食纤维：占总热量 45-50%。
    Mediterranean,

    /// 低碳高脂饮食（LCHF - 非生酮温和低碳）：
    /// - 碳水化合物：50 - 100g/天。
    /// - 蛋白质：1.8 - 2.2 g/kg FFM。
    /// - 脂肪：填充剩余热量（40-50%）。
    LowCarbHighFat,

    /// 自定义宏量百分比分配：(碳水比例, 脂肪比例, 蛋白质比例)，总和为 1.0（如 0.50, 0.25, 0.25）。
    CustomPercentage { carbs_pct: f64, fat_pct: f64, protein_pct: f64 },
}

/// 碳水循环（Carb-Cycling）日程周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarbCyclingDayType {
    /// 高碳日：安排大肌群重训（深蹲/硬拉/大腿/背部）；约 50% 碳水，约 18% 脂肪，蛋白质锚定。
    HighCarb,
    /// 中碳日：常规推拉训练日（上肢力量、肩臂增肌）；均衡碳水分配。
    ModerateCarb,
    /// 低碳日 / 休息日：休息或轻度主动恢复；约 15% 极低碳水，较高健康脂肪（~40%），蛋白质充足。
    LowCarb,
}

/// 宏量营养素详细目标明细。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroTarget {
    pub total_kcal: f64,

    // 蛋白质
    pub protein_g: f64,
    pub protein_kcal: f64,
    pub protein_pct: f64,
    pub protein_per_kg_bw: f64,
    pub protein_per_kg_ffm: f64,

    // 脂肪
    pub fat_g: f64,
    pub fat_kcal: f64,
    pub fat_pct: f64,
    pub fat_per_kg_bw: f64,

    // 碳水化合物
    pub carbs_g: f64,
    pub carbs_kcal: f64,
    pub carbs_pct: f64,

    // 膳食纤维推荐底线（USDA标准：每 1000 kcal 约 14g）
    pub fiber_g_min: f64,

    // 临床与生理安全警示
    pub warnings: Vec<String>,
    pub protocol_notes: String,
}

/// 碳水循环周计划目标表。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CarbCyclingSchedule {
    pub high_carb_target: MacroTarget,
    pub moderate_carb_target: MacroTarget,
    pub low_carb_target: MacroTarget,
    pub weekly_average_kcal: f64,
}

pub struct MacroEngine;

impl MacroEngine {
    pub const KCAL_PER_G_PROTEIN: f64 = 4.0;
    pub const KCAL_PER_G_CARBS: f64 = 4.0;
    pub const KCAL_PER_G_FAT: f64 = 9.0;

    /// 营养学脂肪摄入生理安全红线（基于 Helms, Aragon, Fitschen 2014, JISSN）。
    /// Helms 等人建议自然健美备赛期间膳食脂肪应占总热量 15% 到 30%，
    /// 长期摄入低于总热量 15-20% 或低于 0.5-0.6 g/kg 体重与循环睾酮水平显著下降及脂溶性维生素吸收障碍相关。
    ///
    /// 此处将 0.60 g/kg 体重设为循证防线（非医疗处方，仅作为营养建议护栏）。
    pub const MIN_FAT_G_PER_KG_BW: f64 = 0.60;

    /// 标准生理基础蛋白质摄入底线（WHO / DRI 指南：0.80 g/kg 体重）。
    pub const MIN_PROTEIN_G_PER_KG_BW: f64 = 0.80;

    /// 根据每日总热量预算、用户体征及饮食流派计算宏量营养素目标。
    ///
    /// # 参数
    /// * `total_kcal` - 目标每日热量摄入 (kcal)
    /// * `user` - 用户生理档案
    /// * `body_fat_pct` - 可选体脂率（若为 None 则基于 BMI 与性别年龄估算）
    /// * `protocol` - 选定的饮食流派/分配协议
    pub fn calculate(
        total_kcal: f64,
        user: &UserProfile,
        body_fat_pct: Option<f64>,
        protocol: DietProtocol,
    ) -> MacroTarget {
        let bw = user.weight_kg;
        let bf = body_fat_pct.unwrap_or_else(|| {
            crate::calc::dynamic_weight::DynamicWeightPlanner::estimate_body_fat_pct(
                user.bmi(),
                user.age,
                user.gender,
            )
        });
        let ffm = bw * (1.0 - (bf / 100.0)).max(0.1);

        let mut warnings = Vec::new();

        let (protein_g, fat_g, carbs_g, notes) = match protocol {
            DietProtocol::HighProteinBalanced => {
                // 瘦体重保肌：2.2 g/kg FFM（且不低于 1.8 g/kg 体重）
                let p_g = (2.2 * ffm).max(1.8 * bw);
                let p_kcal = p_g * Self::KCAL_PER_G_PROTEIN;

                // 脂肪：占总热量 28%，保证不低于 0.70 g/kg 体重
                let target_fat_kcal = total_kcal * 0.28;
                let f_g = (target_fat_kcal / Self::KCAL_PER_G_FAT).max(0.70 * bw);
                let f_kcal = f_g * Self::KCAL_PER_G_FAT;

                // 碳水：填充剩余全部能量
                let remaining_kcal = (total_kcal - p_kcal - f_kcal).max(0.0);
                let c_g = remaining_kcal / Self::KCAL_PER_G_CARBS;

                (
                    p_g,
                    f_g,
                    c_g,
                    "高蛋白均衡饮食：蛋白质锚定去脂体重（FFM）以在减脂期最大化保肌；中度脂肪保障内分泌，碳水化合物维持肌糖原储备与训练强度。",
                )
            }

            DietProtocol::Ketogenic => {
                // 严格碳水上限：30g
                let c_g = 30.0;
                let c_kcal = c_g * Self::KCAL_PER_G_CARBS;

                // 适度偏高蛋白质以防肌肉流失但避免过量糖异生：1.8 g/kg FFM
                let p_g = (1.8 * ffm).max(1.2 * bw);
                let p_kcal = p_g * Self::KCAL_PER_G_PROTEIN;

                // 脂肪：供应剩余大部分能量（>70% 总能量）
                let remaining_kcal = (total_kcal - p_kcal - c_kcal).max(0.0);
                let f_g = remaining_kcal / Self::KCAL_PER_G_FAT;

                (
                    p_g,
                    f_g,
                    c_g,
                    "生酮饮食方案：严格 30g 净碳水上限；优质脂肪作为主要生酮底物供能；适量高蛋白保护瘦体重。",
                )
            }

            DietProtocol::Mediterranean => {
                // 不饱和脂肪酸优先：占总热量 38%
                let f_kcal = total_kcal * 0.38;
                let f_g = f_kcal / Self::KCAL_PER_G_FAT;

                // 适中蛋白质：1.6 g/kg 体重
                let p_g = 1.6 * bw;
                let p_kcal = p_g * Self::KCAL_PER_G_PROTEIN;

                // 复合碳水化合物：供应剩余热量（约 45%）
                let remaining_kcal = (total_kcal - p_kcal - f_kcal).max(0.0);
                let c_g = remaining_kcal / Self::KCAL_PER_G_CARBS;

                (
                    p_g,
                    f_g,
                    c_g,
                    "地中海饮食方案：高比例单不饱和与多不饱和脂肪（38%）；适量优质蛋白；植物源复合慢碳为主。",
                )
            }

            DietProtocol::LowCarbHighFat => {
                // 温和低碳水：固定 80g
                let c_g = 80.0;
                let c_kcal = c_g * Self::KCAL_PER_G_CARBS;

                // 较高蛋白质：2.0 g/kg FFM
                let p_g = (2.0 * ffm).max(1.6 * bw);
                let p_kcal = p_g * Self::KCAL_PER_G_PROTEIN;

                // 脂肪：填充剩余热量
                let remaining_kcal = (total_kcal - p_kcal - c_kcal).max(0.0);
                let f_g = remaining_kcal / Self::KCAL_PER_G_FAT;

                (
                    p_g,
                    f_g,
                    c_g,
                    "低碳高脂（LCHF）：80g 温和碳水上限；提高健康脂肪比例，充足蛋白质防止瘦组织分解。",
                )
            }

            DietProtocol::CustomPercentage { carbs_pct, fat_pct, protein_pct } => {
                let sum = carbs_pct + fat_pct + protein_pct;
                let (c_p, f_p, p_p) = if (sum - 1.0).abs() > 1e-4 {
                    (carbs_pct / sum, fat_pct / sum, protein_pct / sum)
                } else {
                    (carbs_pct, fat_pct, protein_pct)
                };

                let p_g = (total_kcal * p_p) / Self::KCAL_PER_G_PROTEIN;
                let f_g = (total_kcal * f_p) / Self::KCAL_PER_G_FAT;
                let c_g = (total_kcal * c_p) / Self::KCAL_PER_G_CARBS;

                (
                    p_g,
                    f_g,
                    c_g,
                    "自定义宏量百分比：用户自定义配置的三大营养素热量占比。",
                )
            }
        };

        // 校验生理安全底线
        let fat_per_kg_bw = fat_g / bw;
        if fat_per_kg_bw < Self::MIN_FAT_G_PER_KG_BW {
            warnings.push(format!(
                "脂肪摄入量 ({:.1} g, {:.2} g/kg 体重) 低于运动营养学推荐安全底线 ({:.2} g/kg 体重；Helms et al. 2014 JISSN 建议占总能量 15-30%)。长期极低脂可能影响类固醇激素合成与脂溶性维生素吸收（非医疗诊断）。",
                fat_g, fat_per_kg_bw, Self::MIN_FAT_G_PER_KG_BW
            ));
        }

        let protein_per_kg_bw = protein_g / bw;
        if protein_per_kg_bw < Self::MIN_PROTEIN_G_PER_KG_BW {
            warnings.push(format!(
                "蛋白质摄入量 ({:.1} g, {:.2} g/kg 体重) 低于生理基准最低需求 ({:.2} g/kg 体重)，存在肌肉分解代谢流失风险。",
                protein_g, protein_per_kg_bw, Self::MIN_PROTEIN_G_PER_KG_BW
            ));
        }

        let protein_kcal = (protein_g * Self::KCAL_PER_G_PROTEIN * 10.0).round() / 10.0;
        let fat_kcal = (fat_g * Self::KCAL_PER_G_FAT * 10.0).round() / 10.0;
        let carbs_kcal = (carbs_g * Self::KCAL_PER_G_CARBS * 10.0).round() / 10.0;
        let calculated_total = protein_kcal + fat_kcal + carbs_kcal;

        let protein_pct = ((protein_kcal / calculated_total.max(1.0)) * 1000.0).round() / 10.0;
        let fat_pct = ((fat_kcal / calculated_total.max(1.0)) * 1000.0).round() / 10.0;
        let carbs_pct = ((carbs_kcal / calculated_total.max(1.0)) * 1000.0).round() / 10.0;

        // USDA 膳食指南：每 1000 kcal 推荐 14g 膳食纤维
        let fiber_g_min = ((total_kcal / 1000.0) * 14.0 * 10.0).round() / 10.0;

        MacroTarget {
            total_kcal: (total_kcal * 10.0).round() / 10.0,
            protein_g: (protein_g * 10.0).round() / 10.0,
            protein_kcal,
            protein_pct,
            protein_per_kg_bw: (protein_per_kg_bw * 100.0).round() / 100.0,
            protein_per_kg_ffm: ((protein_g / ffm) * 100.0).round() / 100.0,
            fat_g: (fat_g * 10.0).round() / 10.0,
            fat_kcal,
            fat_pct,
            fat_per_kg_bw: (fat_per_kg_bw * 100.0).round() / 100.0,
            carbs_g: (carbs_g * 10.0).round() / 10.0,
            carbs_kcal,
            carbs_pct,
            fiber_g_min,
            warnings,
            protocol_notes: notes.to_string(),
        }
    }

    /// 生成完整的三阶段碳水循环计划（高碳日、中碳日、低碳日），
    /// 确保整周总能量消耗与 `baseline_daily_kcal * 7` 完全守恒平衡。
    ///
    /// 经典周期配置：
    /// - 2 天高碳日（腿部/大肌群重训）：基准热量 +15%，高碳水优先。
    /// - 3 天中碳日（上肢力量/肌肥大训练）：基准热量，均衡分配。
    /// - 2 天低碳日（休息日/轻度活动）：基准热量 -15%，降低碳水，适度补充健康脂肪。
    pub fn generate_carb_cycling_schedule(
        baseline_daily_kcal: f64,
        user: &UserProfile,
        body_fat_pct: Option<f64>,
    ) -> CarbCyclingSchedule {
        let high_kcal = baseline_daily_kcal * 1.15;
        let moderate_kcal = baseline_daily_kcal;
        let low_kcal = baseline_daily_kcal * 0.85;

        // High carb target
        let high_target = Self::calculate(
            high_kcal,
            user,
            body_fat_pct,
            DietProtocol::CustomPercentage {
                carbs_pct: 0.55,
                fat_pct: 0.20,
                protein_pct: 0.25,
            },
        );

        // Moderate carb target
        let moderate_target = Self::calculate(
            moderate_kcal,
            user,
            body_fat_pct,
            DietProtocol::HighProteinBalanced,
        );

        // Low carb target
        let low_target = Self::calculate(
            low_kcal,
            user,
            body_fat_pct,
            DietProtocol::LowCarbHighFat,
        );

        // Weekly balance: (2 * High + 3 * Moderate + 2 * Low) / 7
        let weekly_avg = (2.0 * high_kcal + 3.0 * moderate_kcal + 2.0 * low_kcal) / 7.0;

        CarbCyclingSchedule {
            high_carb_target: high_target,
            moderate_carb_target: moderate_target,
            low_carb_target: low_target,
            weekly_average_kcal: (weekly_avg * 10.0).round() / 10.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ActivityLevel, Gender};

    #[test]
    fn test_high_protein_balanced_ffm_anchoring() {
        // 80 kg male, 20% body fat -> FFM = 64 kg
        let user = UserProfile::new(
            28,
            178.0,
            80.0,
            Gender::Male,
            ActivityLevel::Active,
        ).unwrap();

        let target = MacroEngine::calculate(
            2200.0,
            &user,
            Some(20.0),
            DietProtocol::HighProteinBalanced,
        );

        println!("\nHigh Protein Balanced Target: {:?}", target);
        // Protein should be at least 2.2 * 64 = 140.8g (or 1.8 * 80 = 144g)
        assert!(target.protein_g >= 144.0);
        assert!(target.fat_per_kg_bw >= MacroEngine::MIN_FAT_G_PER_KG_BW);
        assert!(target.warnings.is_empty());
        assert!(target.carbs_g > 100.0);
    }

    #[test]
    fn test_ketogenic_protocol_low_carbs() {
        let user = UserProfile::new(
            35,
            170.0,
            75.0,
            Gender::Female,
            ActivityLevel::LowActive,
        ).unwrap();

        let target = MacroEngine::calculate(
            1800.0,
            &user,
            Some(25.0),
            DietProtocol::Ketogenic,
        );

        println!("Keto Target: {:?}", target);
        assert_eq!(target.carbs_g, 30.0);
        assert!(target.fat_pct >= 65.0);
        assert!(target.protein_g > 90.0);
        assert!(target.warnings.is_empty());
    }

    #[test]
    fn test_fat_warning_on_extreme_low_fat() {
        let user = UserProfile::new(
            25,
            180.0,
            85.0,
            Gender::Male,
            ActivityLevel::Active,
        ).unwrap();

        // 刻意设置极低脂（5%）分配
        let target = MacroEngine::calculate(
            2000.0,
            &user,
            Some(15.0),
            DietProtocol::CustomPercentage {
                carbs_pct: 0.60,
                fat_pct: 0.05, // 85kg 成年男性仅 100 kcal / 11g 脂肪！(< 0.13 g/kg)
                protein_pct: 0.35,
            },
        );

        println!("极低脂生理预警: {:?}", target.warnings);
        assert!(!target.warnings.is_empty());
        assert!(target.warnings[0].contains("低于运动营养学推荐安全底线"));
    }

    #[test]
    fn test_carb_cycling_schedule() {
        let user = UserProfile::new(
            30,
            175.0,
            78.0,
            Gender::Male,
            ActivityLevel::Active,
        ).unwrap();

        let schedule = MacroEngine::generate_carb_cycling_schedule(2400.0, &user, Some(18.0));
        println!("Carb Cycling Schedule: {:?}", schedule);

        assert!((schedule.weekly_average_kcal - 2400.0).abs() < 1.0);
        assert!(schedule.high_carb_target.carbs_g > schedule.moderate_carb_target.carbs_g);
        assert!(schedule.moderate_carb_target.carbs_g > schedule.low_carb_target.carbs_g);
        assert!(schedule.low_carb_target.fat_g > schedule.high_carb_target.fat_g);
    }
}
