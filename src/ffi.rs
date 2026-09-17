//! 移动端 UniFFI 窄门面：Android / iOS 只应依赖本模块导出的类型与 [`LiteBalanceSession`]。
//!
//! 设计约束：
//! - DTO 仅用 String / 基本数值 / 简单 enum，避免把 `Path`、`NaiveDate`、复杂嵌套枚举直接暴露给绑定层。
//! - 数据库路径由宿主 App 传入（沙箱 `filesDir` / Application Support）。
//! - 网络相关能力（若后续扩展条码）必须在宿主后台线程调用；当前门面以离线日常能力为主。

use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate};
use uuid::Uuid;

use crate::activity::{ActivityEnergyCalculator, DailyEnergyBalance, find_activity_by_code, get_standard_activity_catalog};
use crate::analytics_reporting::{
    DailyIntakeData, DailyWeightData, FastingSessionData, PeriodicAnalyticsEngine,
};
use crate::cache_db::CacheStorageEngine;
use crate::custom_food::{CustomFoodDraft, CustomFoodEngine, InputBasis};
use crate::db::StorageEngine;
use crate::dynamic_weight::DynamicWeightPlanner;
use crate::energy::EnergyCalc;
use crate::export_import::ExportImportEngine;
use crate::fasting::{FastingProtocol, FastingSession, FastingState};
use crate::goal_profile::{GoalConfig, GoalKind, GoalProfileEngine};
use crate::macro_engine::{DietProtocol, MacroEngine};
use crate::micronutrient_eval::{DriStatus, MicronutrientEvaluator};
use crate::records::{ActivityLogRecord, MealType, UserGoalRecord};
use crate::remote_food_client::RemoteFoodClient;
use crate::seed::seed_default_foods_if_empty;
use crate::unit_system::UnitConverter;
use crate::user::{ActivityLevel, Gender, UserProfile};
use crate::water::WaterCalc;
use crate::workout_compensation::NutritionState;

// ---------------------------------------------------------------------------
// 错误与 DTO
// ---------------------------------------------------------------------------

/// FFI 统一错误（扁平字符串，便于 Kotlin/Swift 直接展示）。
#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum FfiError {
    #[error("{0}")]
    Message(String),
}

impl FfiError {
    fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}

/// 用户档案写入参数。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiUserInput {
    pub user_id: String,
    pub name: String,
    /// 生日 `YYYY-MM-DD`（用于展示与年龄推算）
    pub birthday: String,
    /// 显式年龄（整岁）。能量方程以该字段为准；应与生日大致一致。
    pub age: u8,
    pub height_cm: f64,
    pub weight_kg: f64,
    /// `male` / `female`
    pub gender: String,
    /// `inactive` / `low_active` / `active` / `very_active`
    pub activity_level: String,
}

/// 用户档案读出结果。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiUserProfile {
    pub user_id: String,
    pub name: String,
    pub birthday: String,
    pub age: u8,
    pub height_cm: f64,
    pub weight_kg: f64,
    pub gender: String,
    pub activity_level: String,
}

/// NASEM / IOM 能量对比。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiEnergyComparison {
    pub nasem_2023_kcal: f64,
    pub iom_2005_kcal: f64,
    pub difference_kcal: f64,
    pub difference_pct: f64,
}

/// 精简减重/增重规划结果。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiPlanResult {
    pub maintenance_kcal: f64,
    pub recommended_intake_kcal: f64,
    pub final_weight_kg: f64,
    pub total_weight_change_kg: f64,
    pub fat_mass_change_kg: f64,
    pub fat_free_mass_change_kg: f64,
    pub metabolic_adaptation_kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
}

/// 目标配置。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiGoalInput {
    /// `lose` / `maintain` / `gain`
    pub kind: String,
    pub target_weight_kg: Option<f64>,
    pub weekly_rate_kg: f64,
    pub taper_enabled: bool,
    pub adaptive_enabled: bool,
    pub manual_calorie_offset: f64,
}

/// 自适应预算结果。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiBudgetResult {
    pub base_tdee_kcal: f64,
    pub daily_budget_kcal: f64,
    pub safety_floor_kcal: f64,
    pub safety_floor_triggered: bool,
    pub recommended_protein_g: f64,
    pub recommended_carbs_g: f64,
    pub recommended_fat_g: f64,
    pub clinical_notes: Vec<String>,
}

/// 食物搜索条目。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiFoodItem {
    pub id: String,
    pub name: String,
    pub brand: Option<String>,
    pub energy_kcal_100: f64,
    pub protein_g_100: f64,
    pub carbs_g_100: f64,
    pub fat_g_100: f64,
}

/// 自建食物写入参数（避免 UniFFI 方法参数过多触发 clippy::too_many_arguments）。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiCustomFoodInput {
    pub name: String,
    pub brand: Option<String>,
    pub energy_kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    /// true = 按每 100g；false = 按一份（见 serving_amount）
    pub per_100g: bool,
    pub serving_amount: f64,
}

/// 摄入打卡结果。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiIntakeLog {
    pub id: String,
    pub food_id: String,
    pub food_name: String,
    pub amount: f64,
    pub unit: String,
    pub meal_type: String,
    pub consumed_at: String,
    pub energy_kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
}

/// 日汇总。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDailySummary {
    pub date: String,
    pub total_energy_kcal: f64,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub items_count: u32,
    pub logs: Vec<FfiIntakeLog>,
}

/// 体重历史点。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiWeightPoint {
    pub date: String,
    pub weight_kg: f64,
}

/// 饮水日摘要。
#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiWaterSummary {
    pub date: String,
    pub total_consumed_ml: u32,
    pub goal_ml: u32,
    pub remaining_ml: u32,
    pub progress_pct: f64,
    pub is_goal_reached: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiActivityCatalogItem {
    pub code: String,
    pub name_zh: String,
    pub category: String,
    pub met_value: f64,
    pub modality: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiWorkoutLog {
    pub id: String,
    pub activity_code: String,
    pub activity_name: String,
    pub category: String,
    pub duration_min: f64,
    pub met_value: f64,
    pub gross_kcal: f64,
    pub net_kcal: f64,
    pub compensation_pct: f64,
    pub date: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiEnergyBalance {
    pub date: String,
    pub total_intake_kcal: f64,
    pub base_tdee_kcal: f64,
    pub gross_activity_kcal: f64,
    pub net_activity_kcal: f64,
    pub compensated_kcal: f64,
    pub adjusted_tdee_kcal: f64,
    pub net_balance_kcal: f64,
    pub is_deficit: bool,
    pub diagnostic: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiFastingStatus {
    pub session_id: Option<String>,
    pub is_running: bool,
    pub protocol_label: String,
    pub elapsed_minutes: i64,
    pub target_minutes: i64,
    pub remaining_minutes: i64,
    pub progress_pct: f64,
    pub state_label: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiNutrientRow {
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub target: f64,
    pub coverage_pct: f64,
    pub status: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiDriReport {
    pub date: String,
    pub adequacy_score: f64,
    pub warnings: Vec<String>,
    pub nutrients: Vec<FfiNutrientRow>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiUnitConversion {
    pub kg_lbs: String,
    pub kg_stone: String,
    pub cm_ft_in: String,
    pub kcal_kj: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiPeriodReport {
    pub start_date: String,
    pub end_date: String,
    pub period_days: u32,
    pub metabolic_divergence_kg: Option<f64>,
    pub logging_adherence_pct: f64,
    pub protein_compliance_pct: f64,
    pub water_compliance_pct: f64,
    pub fasting_compliance_pct: f64,
    pub avg_intake_kcal: f64,
    pub insights: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiCacheStats {
    pub total_entries: u32,
    pub active_entries: u32,
    pub expired_entries: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiBarcodeFood {
    pub id: String,
    pub name: String,
    pub brand: Option<String>,
    pub energy_kcal_100: f64,
    pub protein_g_100: f64,
    pub carbs_g_100: f64,
    pub fat_g_100: f64,
    pub from_cache: bool,
    pub attribution: String,
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// 轻衡移动端会话：持有主库与缓存库连接。
#[derive(uniffi::Object)]
pub struct LiteBalanceSession {
    storage: StorageEngine,
    /// 预留：条码/OFF 缓存库（一期日常路径可不触达）。
    #[allow(dead_code)]
    cache: CacheStorageEngine,
}

#[uniffi::export]
impl LiteBalanceSession {
    /// 打开（或创建）沙箱内主库与缓存库，并在空库时写入种子食物。
    #[uniffi::constructor]
    pub fn open(db_path: String, cache_path: String) -> Result<Arc<Self>, FfiError> {
        let storage = StorageEngine::open(&db_path).map_err(|e| FfiError::msg(e.to_string()))?;
        let cache = CacheStorageEngine::open(&cache_path).map_err(|e| FfiError::msg(e.to_string()))?;
        let _ = seed_default_foods_if_empty(&storage);
        Ok(Arc::new(Self { storage, cache }))
    }

    /// 内存库会话（单元测试 / 无磁盘环境）。
    #[uniffi::constructor]
    pub fn open_in_memory() -> Result<Arc<Self>, FfiError> {
        let storage = StorageEngine::open_in_memory().map_err(|e| FfiError::msg(e.to_string()))?;
        let cache = CacheStorageEngine::open_in_memory().map_err(|e| FfiError::msg(e.to_string()))?;
        let _ = seed_default_foods_if_empty(&storage);
        Ok(Arc::new(Self { storage, cache }))
    }

    /// 写入或更新用户档案。
    pub fn upsert_user(&self, input: FfiUserInput) -> Result<(), FfiError> {
        let profile = build_profile(&input)?;
        self.storage
            .insert_user(&input.user_id, &input.name, &input.birthday, &profile)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    /// 读取用户档案；不存在返回 `None`。
    pub fn get_user(&self, user_id: String) -> Result<Option<FfiUserProfile>, FfiError> {
        let rec = self
            .storage
            .get_user(&user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(rec.map(|r| {
            let age = age_from_birthday(&r.birthday).unwrap_or(30);
            FfiUserProfile {
                user_id: r.id,
                name: r.name,
                birthday: r.birthday,
                age,
                height_cm: r.height_cm,
                weight_kg: r.weight_kg,
                gender: r.gender,
                activity_level: r.activity_level,
            }
        }))
    }

    /// 计算 NASEM 2023 / IOM 2005 TDEE（成人非孕非哺）。
    pub fn compute_tdee(&self, input: FfiUserInput) -> Result<FfiEnergyComparison, FfiError> {
        let profile = build_profile(&input)?;
        let comp = EnergyCalc::compare(&profile).map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiEnergyComparison {
            nasem_2023_kcal: comp.nasem_2023_kcal,
            iom_2005_kcal: comp.iom_2005_kcal,
            difference_kcal: comp.difference_kcal,
            difference_pct: comp.difference_pct,
        })
    }

    /// Kevin Hall 反向求解 + 宏量分配的精简计划。
    pub fn compute_plan(
        &self,
        input: FfiUserInput,
        target_weight_kg: f64,
        weeks: u32,
    ) -> Result<FfiPlanResult, FfiError> {
        let profile = build_profile(&input)?;
        let days = (weeks.max(1) * 7) as usize;
        let intake = DynamicWeightPlanner::solve_target_intake(&profile, None, target_weight_kg, days)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let sim = DynamicWeightPlanner::simulate(&profile, None, intake, days, Some(target_weight_kg))
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let macros = MacroEngine::calculate(intake, &profile, None, DietProtocol::HighProteinBalanced);
        Ok(FfiPlanResult {
            maintenance_kcal: sim.initial_maintenance_kcal,
            recommended_intake_kcal: intake,
            final_weight_kg: sim.final_weight_kg,
            total_weight_change_kg: sim.total_weight_change_kg,
            fat_mass_change_kg: sim.fat_mass_change_kg,
            fat_free_mass_change_kg: sim.fat_free_mass_change_kg,
            metabolic_adaptation_kcal: sim.metabolic_adaptation_kcal,
            protein_g: macros.protein_g,
            carbs_g: macros.carbs_g,
            fat_g: macros.fat_g,
        })
    }

    /// 设置体态目标。
    pub fn set_goal(&self, user_id: String, goal: FfiGoalInput) -> Result<(), FfiError> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let kind = GoalKind::from_str(&goal.kind);
        let record = UserGoalRecord {
            id: Uuid::new_v4().to_string(),
            user_id,
            kind: kind.as_str().to_string(),
            target_weight_kg: goal.target_weight_kg,
            weekly_rate_kg: goal.weekly_rate_kg,
            taper_enabled: goal.taper_enabled,
            adaptive_enabled: goal.adaptive_enabled,
            manual_calorie_offset: goal.manual_calorie_offset,
            created_at: now.clone(),
            updated_at: now,
        };
        self.storage
            .set_user_goal(&record)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    /// 读取当前目标；无则 `None`。
    pub fn get_goal(&self, user_id: String) -> Result<Option<FfiGoalInput>, FfiError> {
        let g = self
            .storage
            .get_user_goal(&user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(g.map(|r| FfiGoalInput {
            kind: r.kind,
            target_weight_kg: r.target_weight_kg,
            weekly_rate_kg: r.weekly_rate_kg,
            taper_enabled: r.taper_enabled,
            adaptive_enabled: r.adaptive_enabled,
            manual_calorie_offset: r.manual_calorie_offset,
        }))
    }

    /// 基于档案 + 已存目标 + 近期体重，计算自适应预算。
    pub fn compute_budget(&self, input: FfiUserInput) -> Result<FfiBudgetResult, FfiError> {
        let profile = build_profile(&input)?;
        let goal_rec = self
            .storage
            .get_user_goal(&input.user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let goal_config = match goal_rec {
            Some(g) => GoalConfig {
                kind: GoalKind::from_str(&g.kind),
                target_weight_kg: g.target_weight_kg,
                weekly_rate_kg: g.weekly_rate_kg,
                taper_enabled: g.taper_enabled,
                adaptive_enabled: g.adaptive_enabled,
                manual_calorie_offset: g.manual_calorie_offset,
            },
            None => GoalConfig::default(),
        };
        let history = self
            .storage
            .get_weight_history(&input.user_id, 30)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let result = GoalProfileEngine::compute_adaptive_budget(&profile, &goal_config, &history)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiBudgetResult {
            base_tdee_kcal: result.base_tdee_kcal,
            daily_budget_kcal: result.daily_budget_kcal,
            safety_floor_kcal: result.safety_floor_kcal,
            safety_floor_triggered: result.safety_floor_triggered,
            recommended_protein_g: result.recommended_protein_g,
            recommended_carbs_g: result.recommended_carbs_g,
            recommended_fat_g: result.recommended_fat_g,
            clinical_notes: result.clinical_notes,
        })
    }

    /// FTS5 食物搜索。
    pub fn search_foods(&self, query: String, limit: u32) -> Result<Vec<FfiFoodItem>, FfiError> {
        let foods = self
            .storage
            .search_foods_fts(&query, limit as usize)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let mut out = Vec::with_capacity(foods.len());
        for f in foods {
            let nutr = self
                .storage
                .get_food_with_nutriments(&f.id)
                .map_err(|e| FfiError::msg(e.to_string()))?;
            let (energy, protein, carbs, fat) = nutr
                .map(|(_food, n)| {
                    (
                        n.energy_kcal_100,
                        n.proteins_100,
                        n.carbohydrates_100,
                        n.fat_100,
                    )
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            out.push(FfiFoodItem {
                id: f.id,
                name: f.name,
                brand: f.brand,
                energy_kcal_100: energy,
                protein_g_100: protein,
                carbs_g_100: carbs,
                fat_g_100: fat,
            });
        }
        Ok(out)
    }

    /// 记录饮食摄入。`meal_type`: breakfast/lunch/dinner/snack；`amount` 为克。
    pub fn log_food(
        &self,
        user_id: String,
        food_id: String,
        amount_g: f64,
        meal_type: String,
        consumed_at: String,
    ) -> Result<FfiIntakeLog, FfiError> {
        let meal = MealType::from_str(&meal_type);
        let log = self
            .storage
            .log_intake(&user_id, &food_id, amount_g, "g", meal, &consumed_at)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiIntakeLog {
            id: log.id,
            food_id: log.food_id,
            food_name: log.food_name,
            amount: log.amount,
            unit: log.unit,
            meal_type: log.meal_type.as_str().to_string(),
            consumed_at: log.consumed_at,
            energy_kcal: log.snapshot_energy_kcal,
            protein_g: log.snapshot_protein_g,
            carbs_g: log.snapshot_carbs_g,
            fat_g: log.snapshot_fat_g,
        })
    }

    /// 删除一笔摄入。
    pub fn delete_intake(&self, user_id: String, intake_id: String) -> Result<bool, FfiError> {
        self.storage
            .delete_intake(&user_id, &intake_id)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    /// 日汇总。`boundary_minutes`：生理日界线距午夜的分钟数（0 表示自然日）。
    pub fn daily_summary(
        &self,
        user_id: String,
        date: String,
        boundary_minutes: u32,
    ) -> Result<FfiDailySummary, FfiError> {
        let summary = if boundary_minutes == 0 {
            self.storage
                .get_daily_summary(&user_id, &date)
                .map_err(|e| FfiError::msg(e.to_string()))?
        } else {
            self.storage
                .get_daily_summary_with_boundary(&user_id, &date, boundary_minutes)
                .map_err(|e| FfiError::msg(e.to_string()))?
        };
        Ok(FfiDailySummary {
            date: summary.date,
            total_energy_kcal: summary.total_energy_kcal,
            total_protein_g: summary.total_protein_g,
            total_carbs_g: summary.total_carbs_g,
            total_fat_g: summary.total_fat_g,
            items_count: summary.items_count as u32,
            logs: summary
                .logs
                .into_iter()
                .map(|log| FfiIntakeLog {
                    id: log.id,
                    food_id: log.food_id,
                    food_name: log.food_name,
                    amount: log.amount,
                    unit: log.unit,
                    meal_type: log.meal_type.as_str().to_string(),
                    consumed_at: log.consumed_at,
                    energy_kcal: log.snapshot_energy_kcal,
                    protein_g: log.snapshot_protein_g,
                    carbs_g: log.snapshot_carbs_g,
                    fat_g: log.snapshot_fat_g,
                })
                .collect(),
        })
    }

    /// 记录体重。
    pub fn log_weight(
        &self,
        user_id: String,
        weight_kg: f64,
        body_fat_pct: Option<f64>,
        logged_at: String,
    ) -> Result<String, FfiError> {
        self.storage
            .log_weight(&user_id, weight_kg, body_fat_pct, &logged_at, None)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    /// 体重历史（按日聚合）。
    pub fn weight_history(&self, user_id: String, limit: u32) -> Result<Vec<FfiWeightPoint>, FfiError> {
        let points = self
            .storage
            .get_weight_history(&user_id, limit as usize)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(points
            .into_iter()
            .map(|(d, w)| FfiWeightPoint {
                date: d.format("%Y-%m-%d").to_string(),
                weight_kg: w,
            })
            .collect())
    }

    /// 记录饮水（毫升）。
    pub fn log_water(&self, user_id: String, amount_ml: u32, logged_at: String) -> Result<String, FfiError> {
        self.storage
            .log_water(&user_id, amount_ml, &logged_at)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    /// 当日饮水进度；若 `goal_ml` 为 0，则按档案推荐目标。
    pub fn daily_water(
        &self,
        user_id: String,
        date: String,
        goal_ml: u32,
        profile_for_target: Option<FfiUserInput>,
    ) -> Result<FfiWaterSummary, FfiError> {
        let goal = if goal_ml > 0 {
            goal_ml
        } else if let Some(input) = profile_for_target {
            let profile = build_profile(&input)?;
            WaterCalc::recommend_daily_target(&profile)
        } else {
            2000
        };
        let s = self
            .storage
            .get_daily_water(&user_id, &date, goal)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiWaterSummary {
            date: s.date,
            total_consumed_ml: s.total_consumed_ml,
            goal_ml: s.goal_ml,
            remaining_ml: s.remaining_ml,
            progress_pct: s.progress_pct,
            is_goal_reached: s.is_goal_reached,
        })
    }

    /// NASEM 饮水推荐目标（毫升）。
    pub fn water_target(&self, input: FfiUserInput) -> Result<u32, FfiError> {
        let profile = build_profile(&input)?;
        Ok(WaterCalc::recommend_daily_target(&profile))
    }

    pub fn list_activity_catalog(&self) -> Vec<FfiActivityCatalogItem> {
        get_standard_activity_catalog()
            .into_iter()
            .map(|a| FfiActivityCatalogItem {
                code: a.code.to_string(),
                name_zh: a.name_zh.to_string(),
                category: a.category.display_name_zh().to_string(),
                met_value: a.met_value,
                modality: format!("{:?}", a.modality),
            })
            .collect()
    }

    pub fn log_workout(
        &self,
        input: FfiUserInput,
        activity_code: String,
        duration_min: f64,
        date: String,
        nutrition_state: String,
    ) -> Result<FfiWorkoutLog, FfiError> {
        let profile = build_profile(&input)?;
        let item = find_activity_by_code(&activity_code)
            .ok_or_else(|| FfiError::msg(format!("未知运动编码: {}", activity_code)))?;
        let state = parse_nutrition_state(&nutrition_state);
        let result = ActivityEnergyCalculator::calculate_with_compensation(
            item.met_value,
            item.modality,
            profile.weight_kg,
            duration_min,
            &profile,
            None,
            state,
        );
        let now = Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let record = ActivityLogRecord {
            id: Uuid::new_v4().to_string(),
            user_id: input.user_id.clone(),
            activity_code: item.code.to_string(),
            activity_name: item.name_zh.to_string(),
            category: item.category.as_str().to_string(),
            duration_minutes: duration_min,
            met_value: item.met_value,
            gross_burned_kcal: result.reported_kcal,
            net_credited_kcal: result.credited_kcal,
            compensation_ratio: 1.0 - result.effective_multiplier,
            date: date.clone(),
            created_at: now,
            external_id: None,
        };
        self.storage
            .log_activity(&record)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(workout_from_record(&record))
    }

    pub fn list_workouts(&self, user_id: String, date: String) -> Result<Vec<FfiWorkoutLog>, FfiError> {
        let logs = self
            .storage
            .get_activities_for_date(&user_id, &date)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(logs.iter().map(workout_from_record).collect())
    }

    pub fn delete_workout(&self, user_id: String, workout_id: String) -> Result<bool, FfiError> {
        self.storage
            .delete_activity(&user_id, &workout_id)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn energy_balance(&self, input: FfiUserInput, date: String) -> Result<FfiEnergyBalance, FfiError> {
        let profile = build_profile(&input)?;
        let tdee = EnergyCalc::nasem_2023(&profile).map_err(|e| FfiError::msg(e.to_string()))?;
        let summary = self
            .storage
            .get_daily_summary(&input.user_id, &date)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let (gross, net) = self
            .storage
            .get_daily_activity_totals(&input.user_id, &date)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let bal = DailyEnergyBalance::compute(date, summary.total_energy_kcal, tdee, gross, net);
        Ok(FfiEnergyBalance {
            date: bal.date,
            total_intake_kcal: bal.total_intake_kcal,
            base_tdee_kcal: bal.base_tdee_kcal,
            gross_activity_kcal: bal.gross_activity_burned_kcal,
            net_activity_kcal: bal.net_activity_credited_kcal,
            compensated_kcal: bal.compensated_amount_kcal,
            adjusted_tdee_kcal: bal.adjusted_tdee_kcal,
            net_balance_kcal: bal.net_caloric_balance_kcal,
            is_deficit: bal.is_deficit,
            diagnostic: bal.compensation_diagnostic,
        })
    }

    pub fn start_fasting(&self, user_id: String, protocol: String, started_at: String) -> Result<String, FfiError> {
        let proto = parse_fasting_protocol(&protocol);
        self.storage
            .start_fasting(&user_id, &started_at, proto.target_duration_minutes())
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn fasting_status(&self, user_id: String) -> Result<FfiFastingStatus, FfiError> {
        let active = self
            .storage
            .get_active_fasting(&user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(match active {
            None => FfiFastingStatus {
                session_id: None,
                is_running: false,
                protocol_label: "未开始".into(),
                elapsed_minutes: 0,
                target_minutes: 0,
                remaining_minutes: 0,
                progress_pct: 0.0,
                state_label: "空闲".into(),
            },
            Some((id, started, target)) => {
                let session = FastingSession {
                    id: id.clone(),
                    user_id,
                    started_at: started,
                    target_duration_minutes: target,
                    completed_at: None,
                    cancelled_at: None,
                };
                let state = session.state_at(chrono::Utc::now());
                fasting_status_from_state(Some(id), target, &state)
            }
        })
    }

    pub fn complete_fasting(&self, session_id: String, completed_at: String) -> Result<(), FfiError> {
        self.storage
            .complete_fasting(&session_id, &completed_at)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn cancel_fasting(&self, session_id: String, cancelled_at: String) -> Result<(), FfiError> {
        self.storage
            .cancel_fasting(&session_id, &cancelled_at)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn evaluate_dri(&self, input: FfiUserInput, date: String) -> Result<FfiDriReport, FfiError> {
        let profile = build_profile(&input)?;
        let items = self
            .storage
            .get_daily_intake_nutriments(&input.user_id, &date)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let report = MicronutrientEvaluator::evaluate_daily_intake(&date, profile.gender, profile.age as u32, &items);
        Ok(FfiDriReport {
            date: report.date,
            adequacy_score: report.adequacy_score,
            warnings: report.warnings,
            nutrients: report
                .assessments
                .into_iter()
                .map(|a| FfiNutrientRow {
                    name: a.nutrient_name,
                    amount: a.intake_amount,
                    unit: a.unit,
                    target: a.target_value,
                    coverage_pct: a.data_coverage_pct,
                    status: dri_status_label(&a.status),
                })
                .collect(),
        })
    }

    pub fn convert_units(&self, kg: f64, cm: f64, kcal: f64) -> FfiUnitConversion {
        let lbs = UnitConverter::kg_to_lbs(kg);
        let (st, st_lbs) = UnitConverter::kg_to_stone_lbs(kg);
        let (ft, inch) = UnitConverter::cm_to_feet_inches(cm);
        let kj = UnitConverter::kcal_to_kj(kcal);
        FfiUnitConversion {
            kg_lbs: format!("{:.1} kg = {:.1} lb", kg, lbs),
            kg_stone: format!("{:.1} kg = {}", kg, UnitConverter::format_stone_lbs(st, st_lbs)),
            cm_ft_in: format!("{:.1} cm = {}", cm, UnitConverter::format_feet_inches(ft, inch)),
            kcal_kj: format!("{:.0} kcal = {:.0} kJ", kcal, kj),
        }
    }

    pub fn export_backup_json(&self, user_id: String) -> Result<String, FfiError> {
        let backup = ExportImportEngine::export_native_backup(&self.storage, &user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        serde_json::to_string_pretty(&backup).map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn import_backup_json(&self, json: String) -> Result<String, FfiError> {
        let backup = serde_json::from_str(&json).map_err(|e| FfiError::msg(e.to_string()))?;
        let stats = ExportImportEngine::import_native_backup(&self.storage, &backup)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(format!(
            "已导入 用户{} 食物{} 摄入{} 体重{} 饮水{} 断食{} 运动{}",
            stats.users_imported,
            stats.foods_imported,
            stats.intakes_imported,
            stats.weights_imported,
            stats.waters_imported,
            stats.fasting_imported,
            stats.activities_imported
        ))
    }

    pub fn create_custom_food(&self, input: FfiCustomFoodInput) -> Result<FfiFoodItem, FfiError> {
        let draft = CustomFoodDraft {
            name: input.name,
            brand: input.brand,
            barcode: None,
            basis: Some(if input.per_100g {
                InputBasis::Per100g
            } else {
                InputBasis::PerServing {
                    serving_amount: input.serving_amount.max(1.0),
                    unit: "g".into(),
                }
            }),
            energy_kcal: input.energy_kcal,
            proteins_g: input.protein_g,
            carbs_g: input.carbs_g,
            fat_g: input.fat_g,
            ..Default::default()
        };
        let normalized = CustomFoodEngine::normalize_draft(&draft).map_err(FfiError::msg)?;
        self.storage
            .insert_food(&normalized.food, &normalized.nutriments_100g)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiFoodItem {
            id: normalized.food.id,
            name: normalized.food.name,
            brand: normalized.food.brand,
            energy_kcal_100: normalized.nutriments_100g.energy_kcal_100,
            protein_g_100: normalized.nutriments_100g.proteins_100,
            carbs_g_100: normalized.nutriments_100g.carbohydrates_100,
            fat_g_100: normalized.nutriments_100g.fat_100,
        })
    }

    pub fn list_custom_foods(&self) -> Result<Vec<FfiFoodItem>, FfiError> {
        let foods = self
            .storage
            .list_custom_foods()
            .map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(foods
            .into_iter()
            .map(|w| FfiFoodItem {
                id: w.food.id,
                name: w.food.name,
                brand: w.food.brand,
                energy_kcal_100: w.nutriments.energy_kcal_100,
                protein_g_100: w.nutriments.proteins_100,
                carbs_g_100: w.nutriments.carbohydrates_100,
                fat_g_100: w.nutriments.fat_100,
            })
            .collect())
    }

    pub fn delete_custom_food(&self, food_id: String) -> Result<bool, FfiError> {
        self.storage
            .delete_custom_food(&food_id)
            .map_err(|e| FfiError::msg(e.to_string()))
    }

    pub fn lookup_barcode(&self, barcode: String) -> Result<FfiBarcodeFood, FfiError> {
        let client = RemoteFoodClient::new(None).map_err(|e| FfiError::msg(e.to_string()))?;
        let product = client
            .fetch_by_barcode(&barcode, Some(&self.cache))
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let id = product.barcode.standard_code.clone();
        let food = crate::records::FoodRecord {
            id: id.clone(),
            name: product.food_name.clone(),
            brand: product.brand.clone(),
            source: crate::records::FoodSource::OpenFoodFacts,
            serving_quantity: product.serving_quantity,
            serving_unit: product.serving_unit.clone(),
            image_url: None,
        };
        let _ = self.storage.insert_food(&food, &product.nutriments);
        Ok(FfiBarcodeFood {
            id,
            name: product.food_name,
            brand: product.brand,
            energy_kcal_100: product.nutriments.energy_kcal_100,
            protein_g_100: product.nutriments.proteins_100,
            carbs_g_100: product.nutriments.carbohydrates_100,
            fat_g_100: product.nutriments.fat_100,
            from_cache: product.from_cache,
            attribution: product.attribution,
        })
    }

    pub fn cache_stats(&self) -> Result<FfiCacheStats, FfiError> {
        let s = self.cache.get_cache_stats().map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(FfiCacheStats {
            total_entries: s.total_entries as u32,
            active_entries: s.active_entries as u32,
            expired_entries: s.expired_entries as u32,
        })
    }

    pub fn clear_food_cache(&self) -> Result<u32, FfiError> {
        let n = self.cache.clear_all().map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(n as u32)
    }

    pub fn period_report(
        &self,
        input: FfiUserInput,
        start_date: String,
        end_date: String,
        period_days: u32,
    ) -> Result<FfiPeriodReport, FfiError> {
        let profile = build_profile(&input)?;
        let tdee = EnergyCalc::nasem_2023(&profile).map_err(|e| FfiError::msg(e.to_string()))?;
        let water_goal = WaterCalc::recommend_daily_target(&profile);
        let start = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
            .map_err(|_| FfiError::msg("起始日期格式无效".to_string()))?;
        let mut daily = Vec::new();
        let mut waters = Vec::new();
        for i in 0..period_days {
            let d = start + chrono::Duration::days(i as i64);
            let ds = d.format("%Y-%m-%d").to_string();
            if let Ok(sum) = self.storage.get_daily_summary(&input.user_id, &ds)
                && sum.items_count > 0
            {
                daily.push(DailyIntakeData {
                    date: ds.clone(),
                    intake_kcal: sum.total_energy_kcal,
                    protein_g: sum.total_protein_g,
                    carbs_g: sum.total_carbs_g,
                    fat_g: sum.total_fat_g,
                });
            }
            if let Ok(w) = self.storage.get_daily_water(&input.user_id, &ds, water_goal) {
                waters.push((ds, w.total_consumed_ml));
            }
        }
        let weights = self
            .storage
            .get_weight_history(&input.user_id, period_days as usize)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let weight_data: Vec<DailyWeightData> = weights
            .into_iter()
            .map(|(d, kg)| DailyWeightData {
                day_offset: (d - start).num_days() as f64,
                weight_kg: kg,
            })
            .collect();
        let fasting_rows = self
            .storage
            .list_all_fasting(&input.user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let fasting: Vec<FastingSessionData> = fasting_rows
            .into_iter()
            .map(|f| FastingSessionData {
                duration_hours: f.target_duration_minutes as f64 / 60.0,
                target_hours: f.target_duration_minutes as f64 / 60.0,
                is_completed: f.completed_at.is_some(),
            })
            .collect();
        let goal = self
            .storage
            .get_user_goal(&input.user_id)
            .map_err(|e| FfiError::msg(e.to_string()))?;
        let target_intake = goal
            .as_ref()
            .map(|_| tdee - 400.0)
            .unwrap_or(tdee);
        let report = PeriodicAnalyticsEngine::generate_report(
            &start_date,
            &end_date,
            period_days as usize,
            tdee,
            target_intake.max(1200.0),
            1.6 * profile.weight_kg,
            water_goal,
            &daily,
            &weight_data,
            &waters,
            &fasting,
        );
        Ok(FfiPeriodReport {
            start_date: report.start_date,
            end_date: report.end_date,
            period_days: report.period_days as u32,
            metabolic_divergence_kg: report.energy.metabolic_divergence_kg,
            logging_adherence_pct: report.energy.logging_adherence_pct,
            protein_compliance_pct: report.macros.protein_compliance_rate_pct,
            water_compliance_pct: report.habits.water_compliance_rate_pct,
            fasting_compliance_pct: report.habits.fasting_compliance_rate_pct,
            avg_intake_kcal: report.energy.avg_daily_intake_kcal,
            insights: report.insights,
        })
    }

    pub fn list_users(&self) -> Result<Vec<FfiUserProfile>, FfiError> {
        let users = self.storage.list_all_users().map_err(|e| FfiError::msg(e.to_string()))?;
        Ok(users
            .into_iter()
            .map(|r| FfiUserProfile {
                user_id: r.id,
                name: r.name,
                birthday: r.birthday.clone(),
                age: age_from_birthday(&r.birthday).unwrap_or(30),
                height_cm: r.height_cm,
                weight_kg: r.weight_kg,
                gender: r.gender,
                activity_level: r.activity_level,
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// 内部辅助
// ---------------------------------------------------------------------------

fn build_profile(input: &FfiUserInput) -> Result<UserProfile, FfiError> {
    let gender = match input.gender.to_lowercase().as_str() {
        "female" | "f" => Gender::Female,
        "male" | "m" => Gender::Male,
        other => return Err(FfiError::msg(format!("不支持的性别值: {}（请用 male/female）", other))),
    };
    let activity = parse_activity(&input.activity_level)?;
    UserProfile::new(input.age, input.height_cm, input.weight_kg, gender, activity)
        .map_err(|e| FfiError::msg(e.to_string()))
}

fn parse_nutrition_state(s: &str) -> NutritionState {
    match s.to_lowercase().as_str() {
        "deficit" | "cut" => NutritionState::CaloricDeficit,
        "surplus" | "bulk" => NutritionState::CaloricSurplus,
        _ => NutritionState::Maintenance,
    }
}

fn parse_fasting_protocol(s: &str) -> FastingProtocol {
    match s.trim() {
        "18:6" | "18_6" => FastingProtocol::F18_6,
        "20:4" | "20_4" => FastingProtocol::F20_4,
        "14:10" | "14_10" => FastingProtocol::Circadian14_10,
        _ => FastingProtocol::F16_8,
    }
}

fn workout_from_record(r: &ActivityLogRecord) -> FfiWorkoutLog {
    FfiWorkoutLog {
        id: r.id.clone(),
        activity_code: r.activity_code.clone(),
        activity_name: r.activity_name.clone(),
        category: r.category.clone(),
        duration_min: r.duration_minutes,
        met_value: r.met_value,
        gross_kcal: r.gross_burned_kcal,
        net_kcal: r.net_credited_kcal,
        compensation_pct: r.compensation_ratio * 100.0,
        date: r.date.clone(),
    }
}

fn fasting_status_from_state(id: Option<String>, target: u32, state: &FastingState) -> FfiFastingStatus {
    match state {
        FastingState::Fasting {
            elapsed_minutes,
            remaining_minutes,
            progress_pct,
            ..
        } => FfiFastingStatus {
            session_id: id,
            is_running: true,
            protocol_label: format!("{} 分钟目标", target),
            elapsed_minutes: *elapsed_minutes,
            target_minutes: target as i64,
            remaining_minutes: *remaining_minutes,
            progress_pct: *progress_pct,
            state_label: "断食中".into(),
        },
        FastingState::Overtime {
            elapsed_minutes,
            overtime_minutes,
            progress_pct,
            ..
        } => FfiFastingStatus {
            session_id: id,
            is_running: true,
            protocol_label: format!("{} 分钟目标", target),
            elapsed_minutes: *elapsed_minutes,
            target_minutes: target as i64,
            remaining_minutes: -overtime_minutes,
            progress_pct: *progress_pct,
            state_label: format!("已超时 +{} 分钟", overtime_minutes),
        },
        FastingState::Completed {
            total_duration_minutes,
            reached_target,
        } => FfiFastingStatus {
            session_id: id,
            is_running: false,
            protocol_label: format!("{} 分钟目标", target),
            elapsed_minutes: *total_duration_minutes,
            target_minutes: target as i64,
            remaining_minutes: 0,
            progress_pct: 100.0,
            state_label: if *reached_target {
                "已完成".into()
            } else {
                "已结束".into()
            },
        },
        FastingState::Cancelled { duration_minutes } => FfiFastingStatus {
            session_id: id,
            is_running: false,
            protocol_label: format!("{} 分钟目标", target),
            elapsed_minutes: *duration_minutes,
            target_minutes: target as i64,
            remaining_minutes: 0,
            progress_pct: 0.0,
            state_label: "已取消".into(),
        },
    }
}

fn dri_status_label(status: &DriStatus) -> String {
    match status {
        DriStatus::LowDataCoverage { coverage_pct } => format!("覆盖不足 ({:.0}%)", coverage_pct),
        DriStatus::Deficient { .. } => "不足".into(),
        DriStatus::Adequate { .. } => "基本充足".into(),
        DriStatus::Optimal { .. } => "达标".into(),
        DriStatus::Excessive { .. } => "过量".into(),
    }
}

fn parse_activity(s: &str) -> Result<ActivityLevel, FfiError> {
    Ok(match s.to_lowercase().as_str() {
        "inactive" | "sedentary" => ActivityLevel::Inactive,
        "low" | "low_active" | "lowactive" => ActivityLevel::LowActive,
        "very" | "very_active" | "veryactive" => ActivityLevel::VeryActive,
        "active" => ActivityLevel::Active,
        other => {
            return Err(FfiError::msg(format!(
                "不支持的活动水平: {}（inactive/low_active/active/very_active）",
                other
            )));
        }
    })
}

fn age_from_birthday(birthday: &str) -> Result<u8, FfiError> {
    let b = NaiveDate::parse_from_str(birthday, "%Y-%m-%d")
        .map_err(|_| FfiError::msg(format!("生日格式无效: {}（需要 YYYY-MM-DD）", birthday)))?;
    let today = Local::now().date_naive();
    let mut age = today.year() - b.year();
    if (today.month(), today.day()) < (b.month(), b.day()) {
        age -= 1;
    }
    if !(0..=120).contains(&age) {
        return Err(FfiError::msg(format!("由生日推算的年龄不合理: {}", age)));
    }
    Ok(age as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adult_female() -> FfiUserInput {
        FfiUserInput {
            user_id: "u1".into(),
            name: "Test".into(),
            birthday: "2004-01-01".into(),
            age: 22,
            height_cm: 165.0,
            weight_kg: 63.0,
            gender: "female".into(),
            activity_level: "low_active".into(),
        }
    }

    #[test]
    fn test_ffi_official_tdee_example() {
        let session = LiteBalanceSession::open_in_memory().unwrap();
        let tdee = session.compute_tdee(adult_female()).unwrap();
        assert!((tdee.nasem_2023_kcal.round() - 2275.0).abs() < 1e-9);
    }

    #[test]
    fn test_ffi_rejects_minor() {
        let session = LiteBalanceSession::open_in_memory().unwrap();
        let mut teen = adult_female();
        teen.age = 16;
        let err = session.compute_tdee(teen).unwrap_err();
        assert!(err.to_string().contains("19") || err.to_string().contains("未成年") || err.to_string().contains("生命阶段"));
    }

    #[test]
    fn test_ffi_search_log_summary_water_weight_goal() {
        let session = LiteBalanceSession::open_in_memory().unwrap();
        let user = adult_female();
        session.upsert_user(user.clone()).unwrap();

        let foods = session.search_foods("鸡".into(), 10).unwrap();
        assert!(!foods.is_empty());
        let food_id = foods[0].id.clone();

        let log = session
            .log_food(
                user.user_id.clone(),
                food_id,
                100.0,
                "lunch".into(),
                "2026-09-15T12:00:00Z".into(),
            )
            .unwrap();
        assert!(log.energy_kcal > 0.0);

        let summary = session
            .daily_summary(user.user_id.clone(), "2026-09-15".into(), 0)
            .unwrap();
        assert_eq!(summary.items_count, 1);

        session
            .log_weight(
                user.user_id.clone(),
                63.0,
                None,
                "2026-09-15T08:00:00Z".into(),
            )
            .unwrap();
        let hist = session.weight_history(user.user_id.clone(), 10).unwrap();
        assert!(!hist.is_empty());

        assert!(!session.list_activity_catalog().is_empty());
        let units = session.convert_units(70.0, 170.0, 2000.0);
        assert!(units.kg_lbs.contains("lb"));
        let custom = session
            .create_custom_food(FfiCustomFoodInput {
                name: "测试鸡胸".into(),
                brand: Some("自制".into()),
                energy_kcal: 110.0,
                protein_g: 23.0,
                carbs_g: 0.0,
                fat_g: 1.5,
                per_100g: true,
                serving_amount: 100.0,
            })
            .unwrap();
        assert!(custom.energy_kcal_100 > 0.0);
        assert!(!session.list_custom_foods().unwrap().is_empty());

        let fid = session
            .start_fasting(user.user_id.clone(), "16:8".into(), "2026-09-15T08:00:00Z".into())
            .unwrap();
        let st = session.fasting_status(user.user_id.clone()).unwrap();
        assert!(st.is_running);
        session
            .complete_fasting(fid, "2026-09-15T20:00:00Z".into())
            .unwrap();

        let json = session.export_backup_json(user.user_id.clone()).unwrap();
        assert!(json.contains("litebalance") || json.contains("user") || json.contains("{"));
        let stats = session.cache_stats().unwrap();
        assert_eq!(stats.total_entries, 0);

        session
            .log_water(user.user_id.clone(), 500, "2026-09-15T09:00:00Z".into())
            .unwrap();
        let water = session
            .daily_water(user.user_id.clone(), "2026-09-15".into(), 0, Some(user.clone()))
            .unwrap();
        assert_eq!(water.total_consumed_ml, 500);

        session
            .set_goal(
                user.user_id.clone(),
                FfiGoalInput {
                    kind: "lose".into(),
                    target_weight_kg: Some(60.0),
                    weekly_rate_kg: -0.5,
                    taper_enabled: true,
                    adaptive_enabled: true,
                    manual_calorie_offset: 0.0,
                },
            )
            .unwrap();
        let budget = session.compute_budget(user).unwrap();
        assert!(budget.daily_budget_kcal > 1000.0);
    }
}
