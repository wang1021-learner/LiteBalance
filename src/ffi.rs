//! 移动端 UniFFI 窄门面：Android / iOS 只应依赖本模块导出的类型与 [`LiteBalanceSession`]。
//!
//! 设计约束：
//! - DTO 仅用 String / 基本数值 / 简单 enum，避免把 `Path`、`NaiveDate`、复杂嵌套枚举直接暴露给绑定层。
//! - 数据库路径由宿主 App 传入（沙箱 `filesDir` / Application Support）。
//! - 网络相关能力（若后续扩展条码）必须在宿主后台线程调用；当前门面以离线日常能力为主。

use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate};
use uuid::Uuid;

use crate::cache_db::CacheStorageEngine;
use crate::db::StorageEngine;
use crate::dynamic_weight::DynamicWeightPlanner;
use crate::energy::EnergyCalc;
use crate::goal_profile::{GoalConfig, GoalKind, GoalProfileEngine};
use crate::macro_engine::{DietProtocol, MacroEngine};
use crate::records::{MealType, UserGoalRecord};
use crate::seed::seed_default_foods_if_empty;
use crate::user::{ActivityLevel, Gender, UserProfile};
use crate::water::WaterCalc;

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
