use crate::user::{ActivityLevel, Gender, UserProfile};
use serde::{Deserialize, Serialize};

/// 临床医学与运动经验性饮水目标计算引擎。
///
/// # 科学溯源与标准界限厘清
/// 1. **NASEM / IOM 官方适宜摄入量（Adequate Intake, AI）基准**：
///    - 美国国家医学院（NASEM，前身为 IOM 2005）官方标准：
///      * 参考成年男性：**3.7 L 每日总水分**
///      * 参考成年女性：**2.7 L 每日总水分**
///    - **关键学术界限**：此官方 AI 属于按生物学性别划分的人群常量基准，**并不随体重线性缩放**。
///    - **饮食水 vs 饮用水占比**：临床膳食营养调查表明，每日总水分摄入中约 **20%** 来自日常食物中的水分，
///      约 **80%** 来自纯饮用水及各类流质饮品。
///      因此，纯流质饮水的目标基准线为：男性约 2.96 L、女性约 2.16 L。
///
/// 2. **“35 ml / kg” 经验法则说明**：
///    - 诸如“体重 × 30-35 ml”（女性 27-31 ml/kg）等按体重线性缩放的公式，属于健身教练与运动营养圈中流传的实用经验法则。
///    - 其**并非** NASEM/IOM 等权威官方机构给出的临床法定公式，严禁将其包装为官方权威医学共识。
pub struct WaterCalc;

impl WaterCalc {
    /// NASEM 官方每日总水分适宜摄入量（包括饮水与食物水分）。
    pub const NASEM_AI_TOTAL_MALE_L: f64 = 3.7;
    pub const NASEM_AI_TOTAL_FEMALE_L: f64 = 2.7;
    pub const NASEM_AI_TOTAL_AVERAGED_L: f64 = 3.2;

    /// 每日总水分中来自饮用水/液体饮品的经验典型占比（~80%）。
    pub const BEVERAGE_FRACTION: f64 = 0.80;

    /// 运动教练圈流行的经验法则系数（35 ml / kg 体重）。
    pub const EMPIRICAL_COACHING_ML_PER_KG: f64 = 35.0;

    /// 计算每日建议饮水目标量（ml）。以官方 NASEM AI 为基准线，
    /// 并叠加透明标注的身体活动出汗补水修正系数。
    ///
    /// # 计算公式
    /// * `饮水基准(L) = NASEM_AI_总水量(L) * 0.80`（男性 2.96L / 女性 2.16L）
    /// * `活动补偿(L) = 依活动水平匹配 { 久坐 => 0.0, 轻度 => 0.25, 活跃 => 0.50, 高度活跃 => 0.80 }`
    /// * 四舍五入到最近的 50 ml 便于日常记录。
    pub fn nasem_beverage_target(user: &UserProfile) -> u32 {
        let base_l = match user.gender {
            Gender::Male => Self::NASEM_AI_TOTAL_MALE_L,
            Gender::Female => Self::NASEM_AI_TOTAL_FEMALE_L,
            Gender::NonBinary(_) => Self::NASEM_AI_TOTAL_AVERAGED_L,
        };

        let beverage_base_ml = base_l * Self::BEVERAGE_FRACTION * 1000.0;

        // 根据活动强度的排汗水分补充经验工程调整
        let activity_adjustment_ml = match user.activity_level {
            ActivityLevel::Inactive => 0.0,
            ActivityLevel::LowActive => 250.0,
            ActivityLevel::Active => 500.0,
            ActivityLevel::VeryActive => 800.0,
        };

        let total = beverage_base_ml + activity_adjustment_ml;
        ((total / 50.0).round() * 50.0) as u32
    }

    /// 采用按体重线性缩放的运动教练经验法则（~35 ml/kg）计算饮水目标。
    /// 明确标注为非 NASEM 官方的民间经验参考。
    pub fn empirical_weight_scaled(user: &UserProfile) -> u32 {
        let base = user.weight_kg * Self::EMPIRICAL_COACHING_ML_PER_KG;
        let activity_addition = match user.activity_level {
            ActivityLevel::Inactive => 0.0,
            ActivityLevel::LowActive => 250.0,
            ActivityLevel::Active => 500.0,
            ActivityLevel::VeryActive => 800.0,
        };
        let total = base + activity_addition;
        ((total / 50.0).round() * 50.0) as u32
    }

    /// 推荐默认饮水目标：NASEM 饮用水基准 + 活动量补水调整。
    pub fn recommend_daily_target(user: &UserProfile) -> u32 {
        Self::nasem_beverage_target(user)
    }
}

/// 每日饮水完成情况统计摘要。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyWaterSummary {
    pub date: String,
    pub total_consumed_ml: u32,
    pub goal_ml: u32,
    pub remaining_ml: u32,
    pub progress_pct: f64,
    pub is_goal_reached: bool,
}

impl DailyWaterSummary {
    pub fn new(date: &str, total_consumed_ml: u32, goal_ml: u32) -> Self {
        let remaining_ml = goal_ml.saturating_sub(total_consumed_ml);

        let progress_pct = if goal_ml > 0 {
            ((total_consumed_ml as f64 / goal_ml as f64) * 1000.0).round() / 10.0
        } else {
            100.0
        };

        Self {
            date: date.to_string(),
            total_consumed_ml,
            goal_ml,
            remaining_ml,
            progress_pct,
            is_goal_reached: total_consumed_ml >= goal_ml,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_nasem_recommendation_scaling() {
        // 活跃男性：NASEM 纯饮水基线 = 3.7 * 0.8 = 2.96 L (2960 ml)
        // 运动活跃补偿 = +500 ml -> 3460 ml -> 舍入到 3450 ml
        let male = UserProfile::new(28, 180.0, 80.0, Gender::Male, ActivityLevel::Active).unwrap();
        assert_eq!(WaterCalc::recommend_daily_target(&male), 3450);

        // 久坐女性：NASEM 纯饮水基线 = 2.7 * 0.8 = 2.16 L (2160 ml)
        // 久坐补偿 = 0 ml -> 2160 ml -> 舍入到 2150 ml
        let female = UserProfile::new(25, 165.0, 55.0, Gender::Female, ActivityLevel::Inactive).unwrap();
        assert_eq!(WaterCalc::recommend_daily_target(&female), 2150);
    }

    #[test]
    fn test_empirical_weight_scaled() {
        // 80kg 男性 * 35 = 2800 + 500 (活跃) = 3300 ml
        let male = UserProfile::new(28, 180.0, 80.0, Gender::Male, ActivityLevel::Active).unwrap();
        assert_eq!(WaterCalc::empirical_weight_scaled(&male), 3300);
    }

    #[test]
    fn test_daily_water_summary_progress() {
        let summary = DailyWaterSummary::new("2026-09-14", 1500, 3000);
        assert_eq!(summary.remaining_ml, 1500);
        assert_eq!(summary.progress_pct, 50.0);
        assert!(!summary.is_goal_reached);

        let finished = DailyWaterSummary::new("2026-09-14", 3200, 3000);
        assert_eq!(finished.remaining_ml, 0);
        assert_eq!(finished.progress_pct, 106.7);
        assert!(finished.is_goal_reached);
    }
}
