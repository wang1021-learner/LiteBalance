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

pub mod calc;
pub mod client;
pub mod models;
pub mod storage;

pub use calc::{
    find_activity_by_code, get_standard_activity_catalog, ActivityCatalogItem, ActivityCategory,
    ActivityEnergyCalculator, AdaptiveBudgetResult, CarbCyclingDayType, CarbCyclingSchedule,
    CompensationMode, CustomFoodDraft, CustomFoodEngine, DailyDriReport, DailyEnergyBalance,
    DailyIntakeData, DailyWeightData, DayBoundaryConfig, DayBoundaryEngine, DietProtocol,
    DriStandard, DriStatus, DynamicSimulationResult, DynamicWeightPlanner, EnergyAnalytics,
    EnergyCalc, EnergyComparison, EnergyStandard, ExerciseModality, FastingSessionData,
    GoalConfig, GoalKind, GoalProfileEngine, HabitAnalytics, InputBasis, MacroAnalytics,
    MacroEngine, MacroTarget, MicronutrientEvaluator, NormalizedCustomFood, NutrientAssessment,
    NutritionState, PeriodicAnalyticsEngine, PeriodicReport, TrajectoryDay, UnitConverter,
    UnitSystem, WorkoutCompensationCalc, WorkoutCompensationResult,
};
pub use client::{
    BarcodeError, BarcodeType, BarcodeValidator, NormalizedBarcode, RemoteClientError,
    RemoteFoodClient, RemoteFoodProduct, SlidingWindowRateLimiter, DEFAULT_USER_AGENT,
    OFF_API_BASE_URL,
};
pub use models::{ActivityLevel, Gender, HormoneProfile, NutritionError, UserProfile};
pub use storage::{
    ActivityLogRecord, CacheStats, CacheStorageEngine, CachedFoodRecord, DailySummary,
    ExportImportEngine, FastingSessionRecord, FoodRecord, FoodSource, FoodWithNutriments,
    ImportStats, IntakeLogRecord, LiteBalanceBackup, MealType, Nutriments100g, NutriTrackerBackup,
    StorageEngine, StorageError, UserGoalRecord, UserProfileRecord, WaterLogRecord, WeightLogRecord,
};
