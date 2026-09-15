//! 运动与体力活动代谢追踪引擎 (ActivityTracker & EnergyBalance)
//!
//! 核心标准与临床依据：
//! - **Herrmann et al. 2024 成人身体活动纲要** (2024 Adult Compendium of Physical Activities, PMID: 38242596)：
//!   采用最新标准化 MET (Metabolic Equivalent of Task) 任务代谢当量数值体系；
//! - **Pontzer & Trexler 2026 运动能量补偿模型** (Current Biology 2026)：
//!   根据运动模态（抗阻 / 有氧 / HIIT）与饮食代谢阶段，自动扣除 ~28% 至 ~70% 的代谢与行为代偿；
//! - **每日能量闭环 (Daily Energy Balance)**：
//!   计算“总摄入 - 基础维持消耗 (Base TDEE) - 运动名义消耗 (Gross) + 代偿扣除 (Compensated) = 调整后 TDEE”，
//!   并输出真实的每日净热量赤字/盈余。

use serde::{Deserialize, Serialize};

use crate::user::UserProfile;
use crate::workout_compensation::{
    CompensationMode, ExerciseModality, NutritionState, WorkoutCompensationCalc, WorkoutCompensationResult,
};

/// 身体活动大类分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityCategory {
    /// 跑步与竞走
    Running,
    /// 自行车与骑行
    Bicycling,
    /// 抗阻与力量体能
    Conditioning,
    /// 水上与游泳
    WaterActivities,
    /// 球类与竞技运动
    Sports,
    /// 低强度日常步行与拉伸
    Walking,
    /// 综合与高强度间歇 (HIIT)
    Hybrid,
    /// 用户自定义活动
    Custom,
}

impl ActivityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityCategory::Running => "running",
            ActivityCategory::Bicycling => "bicycling",
            ActivityCategory::Conditioning => "conditioning",
            ActivityCategory::WaterActivities => "water",
            ActivityCategory::Sports => "sports",
            ActivityCategory::Walking => "walking",
            ActivityCategory::Hybrid => "hybrid",
            ActivityCategory::Custom => "custom",
        }
    }

    pub fn display_name_zh(&self) -> &'static str {
        match self {
            ActivityCategory::Running => "跑步竞走",
            ActivityCategory::Bicycling => "自行车骑行",
            ActivityCategory::Conditioning => "抗阻力量训练",
            ActivityCategory::WaterActivities => "游泳与水上活动",
            ActivityCategory::Sports => "球类与竞技体育",
            ActivityCategory::Walking => "日常步行与拉伸",
            ActivityCategory::Hybrid => "高强度间歇 (HIIT)",
            ActivityCategory::Custom => "自定义运动",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" | "run" => ActivityCategory::Running,
            "bicycling" | "bike" | "cycling" => ActivityCategory::Bicycling,
            "conditioning" | "strength" | "weight" => ActivityCategory::Conditioning,
            "water" | "swimming" => ActivityCategory::WaterActivities,
            "sports" | "sport" => ActivityCategory::Sports,
            "walking" | "walk" => ActivityCategory::Walking,
            "hybrid" | "hiit" => ActivityCategory::Hybrid,
            _ => ActivityCategory::Custom,
        }
    }
}

/// 标准 MET 运动库条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityCatalogItem {
    /// 唯一识别码（如 "running_10kph"）
    pub code: &'static str,
    /// 中文规范名称
    pub name_zh: &'static str,
    /// 英文规范名称
    pub name_en: &'static str,
    /// 所属活动大类
    pub category: ActivityCategory,
    /// 临床标准 MET 代谢当量数值 (1 MET = 1.0 kcal/kg/h)
    pub met_value: f64,
    /// 运动模态（映射至 Pontzer 运动代偿模型）
    pub modality: ExerciseModality,
    /// 详细运动强度说明与场景描述
    pub description: &'static str,
}

/// 获取系统内建临床标准 MET 运动知识库（遵循 Herrmann et al. 2024）。
pub fn get_standard_activity_catalog() -> Vec<ActivityCatalogItem> {
    vec![
        // 1. 跑步类
        ActivityCatalogItem {
            code: "running_8kph",
            name_zh: "慢跑 (8 km/h, 配速 7分30秒)",
            name_en: "Running, 5 mph (8 km/h)",
            category: ActivityCategory::Running,
            met_value: 8.3,
            modality: ExerciseModality::Aerobic,
            description: "慢速持续性稳态有氧慢跑，心率主要处于有氧耐力二区 (Zone 2)。",
        },
        ActivityCatalogItem {
            code: "running_10kph",
            name_zh: "常规跑步 (10 km/h, 配速 6分00秒)",
            name_en: "Running, 6.2 mph (10 km/h)",
            category: ActivityCategory::Running,
            met_value: 9.8,
            modality: ExerciseModality::Aerobic,
            description: "中等强度公路或跑步机稳态有氧跑。",
        },
        ActivityCatalogItem {
            code: "running_12kph",
            name_zh: "进阶跑步 (12 km/h, 配速 5分00秒)",
            name_en: "Running, 7.5 mph (12 km/h)",
            category: ActivityCategory::Running,
            met_value: 11.5,
            modality: ExerciseModality::Aerobic,
            description: "高强度乳酸阈值耐力跑。",
        },
        // 2. 步行与低强度活动
        ActivityCatalogItem {
            code: "walking_brisk_5kph",
            name_zh: "快走 (5 km/h, 健步走)",
            name_en: "Walking, brisk pace (5 km/h)",
            category: ActivityCategory::Walking,
            met_value: 3.8,
            modality: ExerciseModality::LowIntensityActive,
            description: "日常高步频快走，提升非运动性活动消耗 (NEAT)。",
        },
        ActivityCatalogItem {
            code: "walking_casual_4kph",
            name_zh: "休闲散步 (4 km/h)",
            name_en: "Walking, casual pace (4 km/h)",
            category: ActivityCategory::Walking,
            met_value: 3.0,
            modality: ExerciseModality::LowIntensityActive,
            description: "轻松平地慢走散步，促进消化与主动恢复。",
        },
        // 3. 骑行类
        ActivityCatalogItem {
            code: "cycling_moderate_20kph",
            name_zh: "公路骑行 (中速 19-22 km/h)",
            name_en: "Bicycling, moderate effort (19-22 km/h)",
            category: ActivityCategory::Bicycling,
            met_value: 8.0,
            modality: ExerciseModality::Aerobic,
            description: "户外平地中等强度公路自行车骑行。",
        },
        ActivityCatalogItem {
            code: "cycling_leisure",
            name_zh: "休闲骑行 (<16 km/h)",
            name_en: "Bicycling, leisure (<16 km/h)",
            category: ActivityCategory::Bicycling,
            met_value: 4.0,
            modality: ExerciseModality::Aerobic,
            description: "日常通勤或轻松低速骑行。",
        },
        ActivityCatalogItem {
            code: "spinning_bike",
            name_zh: "动感单车 / 室内单车课程",
            name_en: "Spinning / Indoor cycling class",
            category: ActivityCategory::Bicycling,
            met_value: 8.5,
            modality: ExerciseModality::HybridHiit,
            description: "高强度间歇爬坡与冲刺单车训练。",
        },
        // 4. 抗阻力量训练
        ActivityCatalogItem {
            code: "strength_training_general",
            name_zh: "抗阻力量训练 (综合器械/哑铃举重)",
            name_en: "Resistance training, free weights & machines",
            category: ActivityCategory::Conditioning,
            met_value: 5.0,
            modality: ExerciseModality::ResistanceTraining,
            description: "健美增肌与肌肥大力量训练，组间常规间歇 (60-90秒)。代偿率极低。",
        },
        ActivityCatalogItem {
            code: "strength_powerlifting",
            name_zh: "大重量力量训练 (三大项力量举)",
            name_en: "Powerlifting (Squat, Bench, Deadlift)",
            category: ActivityCategory::Conditioning,
            met_value: 6.0,
            modality: ExerciseModality::ResistanceTraining,
            description: "大重量低次数核心力量训练，组间长间歇 (2-4分钟)。",
        },
        ActivityCatalogItem {
            code: "calisthenics_bodyweight",
            name_zh: "自重健身 (俯卧撑/引体向上/徒手深蹲)",
            name_en: "Calisthenics, bodyweight exercises",
            category: ActivityCategory::Conditioning,
            met_value: 4.5,
            modality: ExerciseModality::ResistanceTraining,
            description: "徒手自身体重体能与核心力量训练。",
        },
        // 5. 间歇与高强度综合训练 (HIIT / CrossFit)
        ActivityCatalogItem {
            code: "hiit_circuit",
            name_zh: "HIIT 高强度间歇循环训练",
            name_en: "HIIT circuit training",
            category: ActivityCategory::Hybrid,
            met_value: 8.0,
            modality: ExerciseModality::HybridHiit,
            description: "短时间冲刺爆发与短间歇交替的代谢体能训练。",
        },
        ActivityCatalogItem {
            code: "crossfit_wod",
            name_zh: "CrossFit 综合体能训练 (WOD)",
            name_en: "CrossFit Workout of the Day",
            category: ActivityCategory::Hybrid,
            met_value: 8.5,
            modality: ExerciseModality::HybridHiit,
            description: "多关节复合大强度体能挑战。",
        },
        ActivityCatalogItem {
            code: "jump_rope_moderate",
            name_zh: "跳绳 (中等速度, 100-120次/分)",
            name_en: "Jump rope, moderate pace",
            category: ActivityCategory::Hybrid,
            met_value: 10.0,
            modality: ExerciseModality::HybridHiit,
            description: "高效率全身协调跳跃体能训练。",
        },
        // 6. 水上活动
        ActivityCatalogItem {
            code: "swimming_freestyle_moderate",
            name_zh: "游泳 (自由泳中速)",
            name_en: "Swimming, freestyle (moderate effort)",
            category: ActivityCategory::WaterActivities,
            met_value: 8.0,
            modality: ExerciseModality::Aerobic,
            description: "持续有氧自由泳打腿划水。",
        },
        ActivityCatalogItem {
            code: "swimming_breaststroke",
            name_zh: "游泳 (蛙泳休闲速度)",
            name_en: "Swimming, breaststroke (casual pace)",
            category: ActivityCategory::WaterActivities,
            met_value: 5.3,
            modality: ExerciseModality::Aerobic,
            description: "常规中低强度休闲蛙泳锻炼。",
        },
        // 7. 球类体育
        ActivityCatalogItem {
            code: "basketball_game",
            name_zh: "全场竞技篮球比赛",
            name_en: "Basketball, competitive game",
            category: ActivityCategory::Sports,
            met_value: 8.0,
            modality: ExerciseModality::HybridHiit,
            description: "全场折返跑、对抗与急停跳投。",
        },
        ActivityCatalogItem {
            code: "football_soccer",
            name_zh: "足球 (全场竞技比赛)",
            name_en: "Soccer / Football, competitive",
            category: ActivityCategory::Sports,
            met_value: 10.0,
            modality: ExerciseModality::HybridHiit,
            description: "高强度奔跑、冲刺与变向对抗。",
        },
        ActivityCatalogItem {
            code: "badminton_singles",
            name_zh: "羽毛球 (单打对抗)",
            name_en: "Badminton, singles competition",
            category: ActivityCategory::Sports,
            met_value: 5.5,
            modality: ExerciseModality::Aerobic,
            description: "频繁前后场折返跑动与挥拍扣杀。",
        },
        // 8. 身心拉伸与柔韧
        ActivityCatalogItem {
            code: "yoga_flow",
            name_zh: "瑜伽 (哈他/流瑜伽)",
            name_en: "Yoga, Hatha / Vinyasa flow",
            category: ActivityCategory::Walking,
            met_value: 2.8,
            modality: ExerciseModality::LowIntensityActive,
            description: "静态拉伸、呼吸控制与核心平衡稳定。",
        },
    ]
}

/// 根据编码查找标准运动条目
pub fn find_activity_by_code(code: &str) -> Option<ActivityCatalogItem> {
    get_standard_activity_catalog()
        .into_iter()
        .find(|item| item.code.eq_ignore_ascii_case(code))
}

/// 单次运动能量消耗与 Pontzer 2026 代偿结算引擎
pub struct ActivityEnergyCalculator;

impl ActivityEnergyCalculator {
    /// 计算运动的名义总能量消耗（粗消耗 Gross Burned Kcal）。
    ///
    /// 临床标准公式 (Herrmann et al. 2024)：
    /// $$\text{Gross Kcal} = \text{MET} \times \text{Weight}_{\text{kg}} \times \frac{\text{Duration}_{\text{minutes}}}{60}$$
    pub fn calculate_gross_kcal(met_value: f64, weight_kg: f64, duration_minutes: f64) -> f64 {
        if met_value <= 0.0 || weight_kg <= 0.0 || duration_minutes <= 0.0 {
            return 0.0;
        }
        (met_value * weight_kg * (duration_minutes / 60.0) * 10.0).round() / 10.0
    }

    /// 结合用户生理画像与 Pontzer-Trexler 2026 代偿模型，计算单次运动的净计入与代偿详情。
    pub fn calculate_with_compensation(
        met_value: f64,
        modality: ExerciseModality,
        weight_kg: f64,
        duration_minutes: f64,
        user: &UserProfile,
        body_fat_pct: Option<f64>,
        nutrition_state: NutritionState,
    ) -> WorkoutCompensationResult {
        let gross_kcal = Self::calculate_gross_kcal(met_value, weight_kg, duration_minutes);
        let mode = CompensationMode::PontzerTrexler2026 {
            modality,
            nutrition_state,
        };
        WorkoutCompensationCalc::calculate(gross_kcal, user, body_fat_pct, mode)
    }
}

/// 每日全量能量平衡闭环结算数据结构 (Daily Energy Balance Dashboard)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyEnergyBalance {
    /// 查询评估日期 (YYYY-MM-DD)
    pub date: String,
    /// 每日饮食总能量摄入 (kcal)
    pub total_intake_kcal: f64,
    /// 基于 NASEM 2023 DRI 能量方程测算的静息基准总能耗 (Base TDEE, kcal)
    pub base_tdee_kcal: f64,
    /// 记录的所有运动名义总消耗累加 (Gross Burned, kcal)
    pub gross_activity_burned_kcal: f64,
    /// 经由 Pontzer 2026 运动代偿模型计算后，科学计入的净运动消耗 (Net Credited, kcal)
    pub net_activity_credited_kcal: f64,
    /// 人体代谢自发性吸收抵消的代偿扣减值 (Compensated, kcal)
    pub compensated_amount_kcal: f64,
    /// 今日调整后的真实生理总能耗 (Adjusted TDEE = Base TDEE + Net Credited, kcal)
    pub adjusted_tdee_kcal: f64,
    /// 今日真实能量差 (Net Balance = Total Intake - Adjusted TDEE, kcal)
    pub net_caloric_balance_kcal: f64,
    /// 是否处于热量赤字减脂状态 (true: 赤字; false: 维持或盈余)
    pub is_deficit: bool,
    /// 预计以此热量差维持一周的理论体成分/体重变化量 (kg/周, 基于 7700 kcal 能量守恒)
    pub projected_weekly_weight_change_kg: f64,
    /// 代偿防护诊断与防暴食警告说明
    pub compensation_diagnostic: String,
}

impl DailyEnergyBalance {
    /// 聚合饮食摄入、基础代谢与多项运动打卡，推导每日能量平衡闭环。
    pub fn compute(
        date: String,
        total_intake_kcal: f64,
        base_tdee_kcal: f64,
        gross_activity_burned_kcal: f64,
        net_activity_credited_kcal: f64,
    ) -> Self {
        let compensated_amount_kcal = (gross_activity_burned_kcal - net_activity_credited_kcal).max(0.0);
        let adjusted_tdee_kcal = (base_tdee_kcal + net_activity_credited_kcal).round();
        let net_caloric_balance_kcal = (total_intake_kcal - adjusted_tdee_kcal).round();
        let is_deficit = net_caloric_balance_kcal < 0.0;
        let projected_weekly_weight_change_kg = ((net_caloric_balance_kcal * 7.0 / 7700.0) * 100.0).round() / 100.0;

        let compensation_diagnostic = if compensated_amount_kcal > 1.0 {
            format!(
                "根据 Pontzer-Trexler 2026 运动代偿模型，系统已科学抵扣 {:.0} kcal 基础能耗挤压，避免了因高估运动消耗多吃而诱发体脂反弹。",
                compensated_amount_kcal
            )
        } else {
            "今日无额外高代偿运动记录，实际代谢能耗与基准模型高度一致。".to_string()
        };

        Self {
            date,
            total_intake_kcal,
            base_tdee_kcal,
            gross_activity_burned_kcal,
            net_activity_credited_kcal,
            compensated_amount_kcal,
            adjusted_tdee_kcal,
            net_caloric_balance_kcal,
            is_deficit,
            projected_weekly_weight_change_kg,
            compensation_diagnostic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::{ActivityLevel, Gender};

    #[test]
    fn test_calculate_gross_kcal_formula() {
        // 80kg 用户慢跑 (MET 8.3) 30 分钟：8.3 * 80 * 0.5 = 332.0 kcal
        let kcal = ActivityEnergyCalculator::calculate_gross_kcal(8.3, 80.0, 30.0);
        assert!((kcal - 332.0).abs() < 0.1);
    }

    #[test]
    fn test_pontzer_compensation_differentiation() {
        let user = UserProfile::new(28, 178.0, 80.0, Gender::Male, ActivityLevel::Active).unwrap();

        // 1. 抗阻力量训练 (ResistanceTraining)：代偿极低，计入率 ~85%
        let res_result = ActivityEnergyCalculator::calculate_with_compensation(
            5.0,
            ExerciseModality::ResistanceTraining,
            80.0,
            60.0,
            &user,
            None,
            NutritionState::Maintenance,
        );
        // Gross = 5.0 * 80 * 1 = 400 kcal
        assert_eq!(res_result.reported_kcal, 400.0);
        assert!(res_result.credited_kcal >= 340.0); // 至少计入 85%
        assert!(res_result.effective_multiplier >= 0.85);

        // 2. 有氧跑步 (Aerobic)：代偿显著，计入率 ~35%
        let aero_result = ActivityEnergyCalculator::calculate_with_compensation(
            9.8,
            ExerciseModality::Aerobic,
            80.0,
            60.0,
            &user,
            None,
            NutritionState::Maintenance,
        );
        // Gross = 9.8 * 80 * 1 = 784 kcal
        assert_eq!(aero_result.reported_kcal, 784.0);
        assert!(aero_result.credited_kcal < 300.0); // 代偿后仅计入约 35% (~274.4 kcal)
        assert!(aero_result.compensated_kcal > 450.0); // 扣减超 450 kcal
    }

    #[test]
    fn test_daily_energy_balance_deficit_calculation() {
        // 摄入 2000 kcal，基础 TDEE 2400 kcal，运动粗消耗 600 kcal，净计入 250 kcal
        let balance = DailyEnergyBalance::compute("2026-09-15".into(), 2000.0, 2400.0, 600.0, 250.0);

        assert_eq!(balance.adjusted_tdee_kcal, 2650.0); // 2400 + 250
        assert_eq!(balance.net_caloric_balance_kcal, -650.0); // 2000 - 2650
        assert!(balance.is_deficit);
        assert_eq!(balance.compensated_amount_kcal, 350.0); // 600 - 250
        assert_eq!(balance.projected_weekly_weight_change_kg, -0.59); // -650 * 7 / 7700 = -0.59 kg
    }
}
