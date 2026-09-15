//! # 轻衡 (LiteBalance Core) 营养与代谢计算核心引擎
//!
//! 基于 Rust 构建的高性能、本地优先（Local-First）移动端临床代谢与营养计算共享核心引擎（Shared Rust Core）。
//! 核心实现：
//! - NASEM 2023 官方能量参考摄入量（DRI for Energy）双能量模型与回归方程矩阵。
//! - 美国国家卫生研究院（NIH）Kevin Hall 动态能量平衡非线性微分方程模型（Lancet 2011）。
//! - 运动方式感知的能量补偿计算引擎（Pontzer & Trexler 2026 力量与耐力分化补偿；Howard 2025 PNAS 加性模型）。
//! - 循证三大宏量营养素分配与瘦体重（FFM）保护引擎（含碳水循环与生酮/平衡/低脂等协议）。
//! - 动态卡路里预算与自适应目标调节器（平稳着陆缓冲、OLS 实测体重自适应反馈、内分泌安全红线）。
//! - 跨午夜生理日界线算法（Physiological Day Boundary，支持熬夜与轮班作息防日历漂移）。
//! - 本地自建食物全流程管理（包装双基准归一化、微量元素“零污染”原则与 FTS5 自动索引）。
//! - 多单位制转换系统（公制 Metric、英制 Imperial、英石 UK Stone 复合进位与双向高精度换算）。
//! - NASEM 临床微量元素 DRI 雷达与防误判覆盖度评估系统。
//! - 周期性代谢偏离度（Metabolic Divergence）与赤字精准追踪报表。
//! - 支持标准 JSON/CSV 格式双向无损迁移与本地全量备份的导入导出引擎。
//! - 基于 SQLite 与 FTS5 全文检索的本地嵌入式存储引擎（用户数据与网络缓存物理分库隔离）。
//! - 外部食品数据库与条形码查询客户端（OpenFoodFacts API、GS1 模 10 校验、合规 UA 与防封限速）。
//!
//! ## 模块组织
//!
//! 本 crate 采用**扁平模块结构**：全部模块直接置于 `src/` 根目录，不再分 `calc` / `storage` /
//! `client` / `models` 四级子目录。原因是模块总量适中（26 个），扁平结构可避免
//! `crate::calc::energy::EnergyCalc` 这类冗长路径，且为后续移动端 FFI 层提供更直接的引用形式。
//!
//! 模块按职责可分为四类（仅概念划分，非目录划分）：
//! - **代谢与营养计算**：[`energy`]、[`dynamic_weight`]、[`activity`]、[`workout_compensation`]、
//!   [`goal_profile`]、[`macro_engine`]、[`micronutrient_eval`]、[`recipe`]、[`custom_food`]、
//!   [`fasting`]、[`water`]、[`trends`]、[`analytics_reporting`]、[`day_boundary`]、[`unit_system`]
//! - **持久化存储**：[`db`]、[`cache_db`]、[`schema`]、[`records`]、[`seed`]、[`export_import`]、[`storage_error`]
//! - **外部数据客户端**：[`barcode`]、[`remote_food_client`]
//! - **领域模型**：[`user`]、[`nutrition_error`]

// ---------- 代谢与营养计算 ----------
pub mod activity;
pub mod analytics_reporting;
pub mod custom_food;
pub mod day_boundary;
pub mod dynamic_weight;
pub mod energy;
pub mod fasting;
pub mod goal_profile;
pub mod macro_engine;
pub mod micronutrient_eval;
pub mod recipe;
pub mod trends;
pub mod unit_system;
pub mod water;
pub mod workout_compensation;

// ---------- 持久化存储 ----------
pub mod cache_db;
pub mod db;
pub mod export_import;
pub mod records;
pub mod schema;
pub mod seed;
pub mod storage_error;

// ---------- 外部数据客户端 ----------
pub mod barcode;
pub mod remote_food_client;

// ---------- 领域模型 ----------
pub mod nutrition_error;
pub mod user;

// ============================================================================
// 统一公开 API 门面（crate 根 re-export）
//
// 调用方（含 CLI 与后续移动端 FFI 层）一律从 crate 根引入所需类型，
// 无需关心其落在哪个具体模块文件中。这样内部模块可自由重命名或拆分，
// 而不破坏外部调用方的引用路径。
// ============================================================================

// ---------- 代谢与营养计算 ----------
pub use activity::{
    ActivityCatalogItem, ActivityCategory, ActivityEnergyCalculator, DailyEnergyBalance, find_activity_by_code,
    get_standard_activity_catalog,
};
pub use analytics_reporting::{
    DailyIntakeData, DailyWeightData, EnergyAnalytics, FastingSessionData, HabitAnalytics, MacroAnalytics,
    PeriodicAnalyticsEngine, PeriodicReport,
};
pub use custom_food::{CustomFoodDraft, CustomFoodEngine, InputBasis, NormalizedCustomFood};
pub use day_boundary::{DayBoundaryConfig, DayBoundaryEngine};
pub use dynamic_weight::{DynamicSimulationResult, DynamicWeightPlanner, TrajectoryDay};
pub use energy::{ADULT_ENERGY_MIN_AGE, EnergyCalc, EnergyComparison, EnergyStandard};
pub use fasting::{CyclePhase, FastingProtocol, FastingSession, FastingState};
pub use goal_profile::{AdaptiveBudgetResult, GoalConfig, GoalKind, GoalProfileEngine};
pub use macro_engine::{CarbCyclingDayType, CarbCyclingSchedule, DietProtocol, MacroEngine, MacroTarget};
pub use micronutrient_eval::{DailyDriReport, DriStandard, DriStatus, MicronutrientEvaluator, NutrientAssessment};
pub use recipe::{
    MicroDisplayStatus, MicroValue, RecipeComputationResult, RecipeDensityConverter, RecipeIngredientInput,
    RecipeMicroCoverage, compute_recipe_nutrition,
};
pub use trends::{NutritionMovingAverage, StreakStats, WeightProjection, streak_stats, weight_projection};
pub use unit_system::{UnitConverter, UnitSystem};
pub use water::{DailyWaterSummary, WaterCalc};
pub use workout_compensation::{
    CompensationMode, ExerciseModality, NutritionState, WorkoutCompensationCalc, WorkoutCompensationResult,
};

// ---------- 持久化存储 ----------
pub use cache_db::{CacheStats, CacheStorageEngine, CachedFoodRecord};
pub use db::StorageEngine;
pub use export_import::{ExportImportEngine, ImportStats, LiteBalanceBackup, NutriTrackerBackup};
pub use records::{
    ActivityLogRecord, DailySummary, FastingSessionRecord, FoodRecord, FoodSource, FoodWithNutriments, IntakeLogRecord,
    MealType, Nutriments100g, RecipeIngredientRecord, RecipeRecord, RecipeWithDetails, UserGoalRecord,
    UserProfileRecord, WaterLogRecord, WeightLogRecord,
};
pub use seed::seed_default_foods_if_empty;
pub use storage_error::StorageError;

// ---------- 外部数据客户端 ----------
pub use barcode::{BarcodeError, BarcodeType, BarcodeValidator, NormalizedBarcode};
pub use remote_food_client::{
    DEFAULT_USER_AGENT, OFF_API_BASE_URL, RemoteClientError, RemoteFoodClient, RemoteFoodProduct,
    SlidingWindowRateLimiter,
};

// ---------- 领域模型 ----------
pub use nutrition_error::NutritionError;
pub use user::{ActivityLevel, Gender, HormoneProfile, ReproductiveStatus, UserProfile};
