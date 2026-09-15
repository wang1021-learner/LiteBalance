//! 临床代谢动力学与高精度营养计算模块
//!
//! 包含能量代谢（NASEM 2023 DRI / IOM 2005）、NIH Kevin Hall 常微分动态体重规划、
//! 运动能量补偿模型（Pontzer-Trexler 2026）、宏量营养素协议与碳循环编排、
//! 微量元素防误报雷达评估、食谱多原料聚合归一化、临床补水、间歇性断食以及周期性报表引擎。

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

pub use activity::{
    find_activity_by_code, get_standard_activity_catalog, ActivityCatalogItem, ActivityCategory,
    ActivityEnergyCalculator, DailyEnergyBalance,
};
pub use analytics_reporting::{
    DailyIntakeData, DailyWeightData, EnergyAnalytics, FastingSessionData, HabitAnalytics,
    MacroAnalytics, PeriodicAnalyticsEngine, PeriodicReport,
};
pub use custom_food::{
    CustomFoodDraft, CustomFoodEngine, InputBasis, NormalizedCustomFood,
};
pub use day_boundary::{DayBoundaryConfig, DayBoundaryEngine};
pub use dynamic_weight::{DynamicSimulationResult, DynamicWeightPlanner, TrajectoryDay};
pub use energy::{EnergyCalc, EnergyComparison, EnergyStandard};
pub use fasting::{CyclePhase, FastingProtocol, FastingSession, FastingState};
pub use goal_profile::{AdaptiveBudgetResult, GoalConfig, GoalKind, GoalProfileEngine};
pub use macro_engine::{
    CarbCyclingDayType, CarbCyclingSchedule, DietProtocol, MacroEngine, MacroTarget,
};
pub use micronutrient_eval::{
    DailyDriReport, DriStandard, DriStatus, MicronutrientEvaluator, NutrientAssessment,
};
pub use recipe::{
    compute_recipe_nutrition, MicroDisplayStatus, MicroValue, RecipeComputationResult,
    RecipeDensityConverter, RecipeIngredientInput, RecipeMicroCoverage,
};
pub use trends::{streak_stats, weight_projection, NutritionMovingAverage, StreakStats, WeightProjection};
pub use unit_system::{UnitConverter, UnitSystem};
pub use water::{DailyWaterSummary, WaterCalc};
pub use workout_compensation::{
    CompensationMode, ExerciseModality, NutritionState, WorkoutCompensationCalc,
    WorkoutCompensationResult,
};

