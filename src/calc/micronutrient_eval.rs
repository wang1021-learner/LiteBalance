use serde::{Deserialize, Serialize};

use crate::models::Gender;
use crate::storage::models::Nutriments100g;

/// 临床推荐摄入量标准分类（基于 NASEM DRI）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriStandard {
    /// 推荐膳食摄入量（Recommended Dietary Allowance，满足 97%~98% 健康人群每日需求）
    Rda,
    /// 适宜摄入量（Adequate Intake，当 RDA 科学证据不足时制定的适宜参考水平）
    Ai,
}

/// 单项微量营养素摄入的临床评估状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DriStatus {
    /// 数据覆盖度不足（当日本次记录食物中，该微量元素测试质量占比低于 50%）。
    /// 【核心防误判机制】：坚决不把外部数据库的未检测（Null）视为摄入为 0，防止引起用户的虚假缺乏焦虑。
    LowDataCoverage { coverage_pct: f64 },
    /// 摄入不足（在数据覆盖度充足的前提下，摄入量低于 RDA/AI 推荐值的 70%）。
    Deficient { percent_of_target: f64 },
    /// 摄入基本充足（达到 RDA/AI 推荐值的 70% ~ 99%）。
    Adequate { percent_of_target: f64 },
    /// 最佳摄入状态（达到或略超推荐值 100%，且未突破安全上限 UL）。
    Optimal { percent_of_target: f64 },
    /// 摄入过量超标（超过 NASEM 制定的可耐受最高摄入量 UL 或慢性病风险降低限量 CDRR，提示健康风险）。
    Excessive { upper_limit: f64, percent_of_target: f64 },
}

/// 单项微量营养素详细评估结果条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NutrientAssessment {
    /// 元素唯一英文标识（如 "calcium", "zinc"）
    pub nutrient_id: String,
    /// 元素中英文显示名称（如 "Zinc (锌)"）
    pub nutrient_name: String,
    /// 计量单位（如 "mg", "µg"）
    pub unit: String,
    /// 当日实际摄入总量
    pub intake_amount: f64,
    /// 基于年龄与性别的科学目标推荐量（RDA 或 AI）
    pub target_value: f64,
    /// 参考标准类型（RDA 或 AI）
    pub standard: DriStandard,
    /// 安全上限阈值（UL，若无特定食物上限则为 None）
    pub upper_limit: Option<f64>,
    /// 达成目标百分比（%）
    pub percentage_of_target: f64,
    /// 数据置信度：当天所吃食物中包含此营养素测量数据的质量覆盖率（%）
    pub data_coverage_pct: f64,
    /// 临床健康状态评估
    pub status: DriStatus,
}

/// 每日全景微量营养素 DRI 雷达评估总报表
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyDriReport {
    /// 评估日期（YYYY-MM-DD）
    pub date: String,
    /// 当天记录的全部食物质量总计（克）
    pub total_food_mass_g: f64,
    /// 包含的 14 项关键微量营养素详细评估列表
    pub assessments: Vec<NutrientAssessment>,
    /// 临床拦截警报（严重缺乏或过量超标警报提示）
    pub warnings: Vec<String>,
    /// 微量元素整体充足度评分（有效可测营养素中达标的百分比 0~100%）
    pub adequacy_score: f64,
}

/// 临床级 NASEM 膳食营养素参考摄入量（DRI）评估计算器
pub struct MicronutrientEvaluator;

impl MicronutrientEvaluator {
    /// 判定为“可靠数据覆盖”的最低食物质量覆盖率阈值（50%）
    pub const MIN_CONFIDENCE_COVERAGE_PCT: f64 = 50.0;

    /// 评估一日饮食中全部微量元素的摄入量、达标率与过量风险。
    ///
    /// 严格遵循权威 NASEM DRI 标准：
    /// - 区分性别与生理阶段（如绝经前育龄女性铁 18mg vs 男性/绝经后女性 8mg；50/70 岁钙需求提升至 1200mg）
    /// - 严守安全上限拦截（钠上限 2300mg、钙上限 2500mg、锌上限 40mg、铁上限 45mg、维生素 A/C/D/B3/B6 上限）
    /// - 数据完整性保护（未录入数据标记为 LowDataCoverage，避免误报“虚假缺乏”）
    pub fn evaluate_daily_intake(
        date: &str,
        gender: Gender,
        age: u32,
        consumed_items: &[(f64, Nutriments100g)],
    ) -> DailyDriReport {
        // 计算当天摄入的所有食物总克重
        let total_food_mass_g: f64 = consumed_items.iter().map(|(g, _)| *g).sum();

        // 生理学性别判定（用于铁、钙、镁等显著受性别影响的营养素标准）
        let is_female = match gender {
            Gender::Female => true,
            Gender::Male => false,
            Gender::NonBinary(h) => matches!(h, crate::models::HormoneProfile::EstrogenTypical),
        };

        let mut assessments = Vec::new();
        let mut warnings = Vec::new();

        // 内部辅助闭包：计算单项营养素摄入量、覆盖率与安全状态
        let mut eval_nutrient = |id: &'static str,
                                 name: &'static str,
                                 unit: &'static str,
                                 target: f64,
                                 standard: DriStandard,
                                 ul: Option<f64>,
                                 extractor: &dyn Fn(&Nutriments100g) -> Option<f64>| {
            let mut total_intake = 0.0;
            let mut measured_mass = 0.0;

            for (amount_g, nutriments) in consumed_items {
                if let Some(val_per_100g) = extractor(nutriments) {
                    total_intake += (*amount_g / 100.0) * val_per_100g;
                    measured_mass += *amount_g;
                }
            }

            // 数据已知覆盖率（%） = 有该项测试数据的食物克重 / 当日总食物克重
            let coverage_pct = if total_food_mass_g > 0.0 {
                (measured_mass / total_food_mass_g * 100.0).min(100.0)
            } else {
                0.0
            };

            // 目标达成百分比（%）
            let percent_of_target = if target > 0.0 {
                total_intake / target * 100.0
            } else {
                0.0
            };

            // 临床状态判定
            let status = if coverage_pct < Self::MIN_CONFIDENCE_COVERAGE_PCT {
                // 覆盖率不足 50%：标记为数据不足，避免引起假性焦虑
                DriStatus::LowDataCoverage { coverage_pct }
            } else if let Some(upper) = ul {
                if total_intake > upper {
                    let w = format!(
                        "突破 {} 的安全摄入上限 (UL): 实测 {:.1} {} (上限标准: {:.1} {})",
                        name, total_intake, unit, upper, unit
                    );
                    warnings.push(w);
                    DriStatus::Excessive {
                        upper_limit: upper,
                        percent_of_target,
                    }
                } else if percent_of_target >= 100.0 {
                    DriStatus::Optimal { percent_of_target }
                } else if percent_of_target >= 70.0 {
                    DriStatus::Adequate { percent_of_target }
                } else {
                    let w = format!(
                        "{} 摄入量明显偏低: 实测 {:.1} {} (仅达 RDA/AI 推荐标准的 {:.1}%)",
                        name, total_intake, unit, percent_of_target
                    );
                    warnings.push(w);
                    DriStatus::Deficient { percent_of_target }
                }
            } else if percent_of_target >= 100.0 {
                DriStatus::Optimal { percent_of_target }
            } else if percent_of_target >= 70.0 {
                DriStatus::Adequate { percent_of_target }
            } else {
                let w = format!(
                    "{} 摄入量明显偏低: 实测 {:.1} {} (仅达 RDA/AI 推荐标准的 {:.1}%)",
                    name, total_intake, unit, percent_of_target
                );
                warnings.push(w);
                DriStatus::Deficient { percent_of_target }
            };

            assessments.push(NutrientAssessment {
                nutrient_id: id.to_string(),
                nutrient_name: name.to_string(),
                unit: unit.to_string(),
                intake_amount: total_intake,
                target_value: target,
                standard,
                upper_limit: ul,
                percentage_of_target: percent_of_target,
                data_coverage_pct: coverage_pct,
                status,
            });
        };

        // ---------------------------------------------------------------------
        // 1. 矿物质类（Minerals）
        // ---------------------------------------------------------------------

        // 钙（Calcium）：基准 RDA 1000mg；女性 >50 岁或男性 >70 岁提升至 1200mg。安全上限 2500mg（中老年 2000mg）。
        let (calcium_target, calcium_ul) = if (is_female && age > 50) || (!is_female && age > 70) {
            (1200.0, 2000.0)
        } else {
            (1000.0, 2500.0)
        };
        eval_nutrient(
            "calcium",
            "Calcium (钙)",
            "mg",
            calcium_target,
            DriStandard::Rda,
            Some(calcium_ul),
            &|n| n.calcium_mg_100,
        );

        // 铁（Iron）：男性全年龄段 8mg；育龄女性（19-50岁）因月经失血为 18mg；女性 >50岁（绝经后）降回 8mg。安全上限 45mg。
        let iron_target = if is_female && age <= 50 { 18.0 } else { 8.0 };
        eval_nutrient(
            "iron",
            "Iron (铁)",
            "mg",
            iron_target,
            DriStandard::Rda,
            Some(45.0),
            &|n| n.iron_mg_100,
        );

        // 锌（Zinc）：男性 RDA 11mg；女性 RDA 8mg。安全上限 40mg。
        let zinc_target = if is_female { 8.0 } else { 11.0 };
        eval_nutrient(
            "zinc",
            "Zinc (锌)",
            "mg",
            zinc_target,
            DriStandard::Rda,
            Some(40.0),
            &|n| n.zinc_mg_100,
        );

        // 镁（Magnesium）：男性 400-420mg；女性 310-320mg。
        let magnesium_target = if is_female {
            if age <= 30 { 310.0 } else { 320.0 }
        } else {
            if age <= 30 { 400.0 } else { 420.0 }
        };
        eval_nutrient(
            "magnesium",
            "Magnesium (镁)",
            "mg",
            magnesium_target,
            DriStandard::Rda,
            None,
            &|n| n.magnesium_mg_100,
        );

        // 钾（Potassium）：男性 AI 3400mg；女性 AI 2600mg。
        let potassium_target = if is_female { 2600.0 } else { 3400.0 };
        eval_nutrient(
            "potassium",
            "Potassium (钾)",
            "mg",
            potassium_target,
            DriStandard::Ai,
            None,
            &|n| n.potassium_mg_100,
        );

        // 钠（Sodium）：AI 适宜量 1500mg；慢性病减风险上限 CDRR / UL 2300mg。
        eval_nutrient(
            "sodium",
            "Sodium (钠)",
            "mg",
            1500.0,
            DriStandard::Ai,
            Some(2300.0),
            &|n| n.sodium_mg_100,
        );

        // 磷（Phosphorus）：成骨与能量代谢。RDA 700mg；上限 4000mg。
        eval_nutrient(
            "phosphorus",
            "Phosphorus (磷)",
            "mg",
            700.0,
            DriStandard::Rda,
            Some(4000.0),
            &|n| n.phosphorus_mg_100,
        );

        // ---------------------------------------------------------------------
        // 2. 维生素类（Vitamins）
        // ---------------------------------------------------------------------

        // 维生素 A（Vitamin A）：男性 RDA 900µg RAE；女性 700µg RAE。安全上限 3000µg。
        let vit_a_target = if is_female { 700.0 } else { 900.0 };
        eval_nutrient(
            "vitamin_a",
            "Vitamin A (维生素A)",
            "µg",
            vit_a_target,
            DriStandard::Rda,
            Some(3000.0),
            &|n| n.vitamin_a_ug_100,
        );

        // 维生素 C（Vitamin C）：男性 RDA 90mg；女性 75mg。安全上限 2000mg。
        let vit_c_target = if is_female { 75.0 } else { 90.0 };
        eval_nutrient(
            "vitamin_c",
            "Vitamin C (维生素C)",
            "mg",
            vit_c_target,
            DriStandard::Rda,
            Some(2000.0),
            &|n| n.vitamin_c_mg_100,
        );

        // 维生素 D（Vitamin D）：<=70岁 RDA 15µg (600 IU)；>70岁 20µg。安全上限 100µg (4000 IU)。
        let vit_d_target = if age > 70 { 20.0 } else { 15.0 };
        eval_nutrient(
            "vitamin_d",
            "Vitamin D (维生素D)",
            "µg",
            vit_d_target,
            DriStandard::Rda,
            Some(100.0),
            &|n| n.vitamin_d_ug_100,
        );

        // 维生素 B6（Vitamin B6）：19-50岁 1.3mg；中老年男性 1.7mg；中老年女性 1.5mg。上限 100mg。
        let vit_b6_target = if age <= 50 {
            1.3
        } else if is_female {
            1.5
        } else {
            1.7
        };
        eval_nutrient(
            "vitamin_b6",
            "Vitamin B6 (维生素B6)",
            "mg",
            vit_b6_target,
            DriStandard::Rda,
            Some(100.0),
            &|n| n.vitamin_b6_mg_100,
        );

        // 维生素 B12（Vitamin B12）：RDA 2.4µg（红细胞与神经髓鞘合成不可或缺）。
        eval_nutrient(
            "vitamin_b12",
            "Vitamin B12 (维生素B12)",
            "µg",
            2.4,
            DriStandard::Rda,
            None,
            &|n| n.vitamin_b12_ug_100,
        );

        // 烟酸 / B3（Niacin）：男性 16mg NE；女性 14mg NE。上限 35mg。
        let niacin_target = if is_female { 14.0 } else { 16.0 };
        eval_nutrient(
            "niacin",
            "Niacin (烟酸/B3)",
            "mg",
            niacin_target,
            DriStandard::Rda,
            Some(35.0),
            &|n| n.niacin_mg_100,
        );

        // 膳食纤维（Dietary Fiber）：男性 <=50岁 38g，>50岁 30g；女性 <=50岁 25g，>50岁 21g。
        let fiber_target = if is_female {
            if age <= 50 { 25.0 } else { 21.0 }
        } else {
            if age <= 50 { 38.0 } else { 30.0 }
        };
        eval_nutrient(
            "fiber",
            "Dietary Fiber (膳食纤维)",
            "g",
            fiber_target,
            DriStandard::Ai,
            None,
            &|n| n.fiber_100,
        );

        // 计算整体微量元素达标得分：仅统计数据覆盖度充足（>=50%）的有效营养素
        let covered: Vec<&NutrientAssessment> = assessments
            .iter()
            .filter(|a| !matches!(a.status, DriStatus::LowDataCoverage { .. }))
            .collect();

        let adequacy_score = if !covered.is_empty() {
            let met_count = covered
                .iter()
                .filter(|a| matches!(a.status, DriStatus::Optimal { .. } | DriStatus::Adequate { .. }))
                .count();
            (met_count as f64 / covered.len() as f64) * 100.0
        } else {
            0.0
        };

        DailyDriReport {
            date: date.to_string(),
            total_food_mass_g,
            assessments,
            warnings,
            adequacy_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_target_gender_and_age_differentiation() {
        // 验证铁元素 RDA 标准在年龄与性别上的科学差异：
        // 育龄女性（25岁）18mg vs 年轻男性（25岁）8mg vs 绝经后女性（55岁）8mg
        let empty_items: Vec<(f64, Nutriments100g)> = vec![];

        let female_young =
            MicronutrientEvaluator::evaluate_daily_intake("2026-05-14", Gender::Female, 25, &empty_items);
        let iron_fy = female_young
            .assessments
            .iter()
            .find(|a| a.nutrient_id == "iron")
            .unwrap();
        assert_eq!(iron_fy.target_value, 18.0);

        let male_young = MicronutrientEvaluator::evaluate_daily_intake("2026-05-14", Gender::Male, 25, &empty_items);
        let iron_my = male_young.assessments.iter().find(|a| a.nutrient_id == "iron").unwrap();
        assert_eq!(iron_my.target_value, 8.0);

        let female_older =
            MicronutrientEvaluator::evaluate_daily_intake("2026-05-14", Gender::Female, 55, &empty_items);
        let iron_fo = female_older
            .assessments
            .iter()
            .find(|a| a.nutrient_id == "iron")
            .unwrap();
        assert_eq!(iron_fo.target_value, 8.0);
    }

    #[test]
    fn test_low_data_coverage_suppresses_false_deficiency() {
        // 核心防假性焦虑测试：
        // 用户总共吃了 1000g 食物，但仅有 200g 食物在数据库里包含锌（Zinc）测量值（覆盖度仅 20%）。
        // 此时绝不能直接判定为“严重缺锌”，必须标记为 LowDataCoverage，并且不弹报警！
        let mut nutriments_with_zinc = Nutriments100g::simple(100.0, 10.0, 10.0, 2.0);
        nutriments_with_zinc.zinc_mg_100 = Some(1.0); // 200g -> 2.0mg 锌（男性标准 11mg）

        let nutriments_without_zinc = Nutriments100g::simple(100.0, 10.0, 10.0, 2.0); // 锌为 None 未测

        let consumed = vec![(200.0, nutriments_with_zinc), (800.0, nutriments_without_zinc)];

        let report = MicronutrientEvaluator::evaluate_daily_intake("2026-05-14", Gender::Male, 30, &consumed);
        let zinc = report.assessments.iter().find(|a| a.nutrient_id == "zinc").unwrap();

        assert_eq!(zinc.intake_amount, 2.0);
        assert_eq!(zinc.data_coverage_pct, 20.0);
        // 覆盖度 20% (< 50%)，必须归类为 LowDataCoverage，而非 Deficient！
        assert!(
            matches!(zinc.status, DriStatus::LowDataCoverage { coverage_pct } if (coverage_pct - 20.0).abs() < 1e-4)
        );
        // 不得产生误导性的虚假报警
        assert!(!report.warnings.iter().any(|w| w.contains("Zinc")));
    }

    #[test]
    fn test_excessive_sodium_triggers_ul_warning() {
        // 高钠拦截测试：
        // 当天摄入钠 3500mg（远超 2300mg CDRR/UL 阈值），触发真实健康警报
        let mut nutriments = Nutriments100g::simple(200.0, 20.0, 15.0, 5.0);
        nutriments.sodium_mg_100 = Some(700.0); // 500g -> 3500mg 钠

        let consumed = vec![(500.0, nutriments)];

        let report = MicronutrientEvaluator::evaluate_daily_intake("2026-05-14", Gender::Male, 30, &consumed);
        let sodium = report.assessments.iter().find(|a| a.nutrient_id == "sodium").unwrap();

        assert_eq!(sodium.intake_amount, 3500.0);
        assert!(matches!(sodium.status, DriStatus::Excessive { upper_limit, .. } if upper_limit == 2300.0));
        assert!(report.warnings.iter().any(|w| w.contains("Sodium (钠)")));
    }
}
