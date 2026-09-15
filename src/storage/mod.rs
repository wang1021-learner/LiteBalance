//! 本地优先持久化存储模块
//!
//! 基于纯原生 SQLite + FTS5 全文索引，提供离线数据库模式定义、CRUD 操作、
//! 食谱管理、基准食物播种、以及 JSON/CSV 数据迁移与备份还原。

pub mod cache_db;
pub mod db;
pub mod error;
pub mod export_import;
pub mod models;
pub mod schema;
pub mod seed;

pub use cache_db::{CacheStats, CacheStorageEngine, CachedFoodRecord};
pub use db::StorageEngine;
pub use error::StorageError;
pub use export_import::{ExportImportEngine, ImportStats, LiteBalanceBackup, NutriTrackerBackup};
pub use models::{
    ActivityLogRecord, DailySummary, FastingSessionRecord, FoodRecord, FoodSource, FoodWithNutriments, IntakeLogRecord,
    MealType, Nutriments100g, RecipeIngredientRecord, RecipeRecord, RecipeWithDetails, UserGoalRecord,
    UserProfileRecord, WaterLogRecord, WeightLogRecord,
};
pub use seed::seed_default_foods_if_empty;
