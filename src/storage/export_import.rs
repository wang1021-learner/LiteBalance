use serde::{Deserialize, Serialize};

use crate::storage::error::StorageError;
use crate::storage::models::{
    ActivityLogRecord, FastingSessionRecord, FoodWithNutriments,
    IntakeLogRecord, RecipeWithDetails, UserGoalRecord,
    UserProfileRecord, WaterLogRecord, WeightLogRecord,
};
use crate::storage::StorageEngine;

/// 批量数据导入统计结果
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportStats {
    /// 成功导入的用户档案数
    pub users_imported: usize,
    /// 成功导入的食物数据库记录数
    pub foods_imported: usize,
    /// 成功导入的餐饮摄入日志数
    pub intakes_imported: usize,
    /// 成功导入的体重称重记录数
    pub weights_imported: usize,
    /// 成功导入的饮水记录数
    pub waters_imported: usize,
    /// 成功导入的断食记录数
    pub fasting_imported: usize,
    /// 成功导入的运动打卡记录数
    pub activities_imported: usize,
    /// 成功导入的食谱数
    pub recipes_imported: usize,
    /// 成功导入的体态目标配置数
    pub goals_imported: usize,
    /// 导入过程中的非致命错误与警告列表
    pub errors: Vec<String>,
}

/// 本地原生全量数据库单文件备份结构（便于用户完整掌控个人数据）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NutriTrackerBackup {
    /// 备份格式版本号
    pub version: u32,
    /// 备份导出时间（ISO-8601 UTC）
    pub exported_at: String,
    /// 用户身体档案快照
    pub user: Option<UserProfileRecord>,
    /// 本地存储的全部食物库（包含 24 项完整营养素）
    pub foods: Vec<FoodWithNutriments>,
    /// 历史饮食摄入日记
    pub intakes: Vec<IntakeLogRecord>,
    /// 历史体重与体脂称重记录
    pub weights: Vec<WeightLogRecord>,
    /// 历史饮水记录
    pub waters: Vec<WaterLogRecord>,
    /// 历史断食会话记录
    pub fasting: Vec<FastingSessionRecord>,
    /// 历史运动打卡记录
    #[serde(default)]
    pub activities: Vec<ActivityLogRecord>,
    /// 用户自定义食谱及原料配比
    pub recipes: Vec<RecipeWithDetails>,
    /// 用户体态目标与预算配置快照
    #[serde(default)]
    pub goal: Option<UserGoalRecord>,
}

/// 轻衡 (LiteBalance) 原生备份数据类型别名
pub type LiteBalanceBackup = NutriTrackerBackup;

// -----------------------------------------------------------------------------
// 数据导入与导出核心引擎实现
// -----------------------------------------------------------------------------

/// 数据导入与导出引擎
pub struct ExportImportEngine;

impl ExportImportEngine {
    /// 导出包含全部本地表数据的单文件 JSON 备份
    pub fn export_native_backup(
        storage: &StorageEngine,
        user_id: &str,
    ) -> Result<NutriTrackerBackup, StorageError> {
        let user = storage.get_user(user_id)?;
        let foods = storage.list_all_foods()?;
        let intakes = storage.list_all_intakes(user_id)?;
        let weights = storage.list_all_weights(user_id)?;
        let waters = storage.list_all_waters(user_id)?;
        let fasting = storage.list_all_fasting(user_id)?;
        let activities = storage.list_all_activities(user_id)?;
        let recipes = storage.list_all_recipes_with_details(user_id)?;
        let goal = storage.get_user_goal(user_id)?;

        let now = chrono::Utc::now().to_rfc3339();
        Ok(NutriTrackerBackup {
            version: 1,
            exported_at: now,
            user,
            foods,
            intakes,
            weights,
            waters,
            fasting,
            activities,
            recipes,
            goal,
        })
    }

    /// 将原生单文件 JSON 备份完整恢复至 SQLite 数据库中
    pub fn import_native_backup(
        storage: &StorageEngine,
        backup: &NutriTrackerBackup,
    ) -> Result<ImportStats, StorageError> {
        let mut stats = ImportStats::default();

        if let Some(user) = &backup.user {
            storage.insert_user_raw(user)?;
            stats.users_imported += 1;
        }

        for item in &backup.foods {
            storage.insert_food_raw(&item.food, &item.nutriments)?;
            stats.foods_imported += 1;
        }

        for intake in &backup.intakes {
            storage.insert_intake_log_raw(intake)?;
            stats.intakes_imported += 1;
        }

        for weight in &backup.weights {
            storage.insert_weight_log_raw(weight)?;
            stats.weights_imported += 1;
        }

        for water in &backup.waters {
            storage.insert_water_log_raw(water)?;
            stats.waters_imported += 1;
        }

        for fast in &backup.fasting {
            storage.insert_fasting_session_raw(fast)?;
            stats.fasting_imported += 1;
        }

        for activity in &backup.activities {
            storage.insert_activity_raw(activity)?;
            stats.activities_imported += 1;
        }

        for recipe in &backup.recipes {
            let ingredient_inputs: Vec<crate::calc::recipe::RecipeIngredientInput> = recipe
                .ingredients
                .iter()
                .filter_map(|i| {
                    let food_tuple = storage.get_food_with_nutriments(&i.food_id).ok()??;
                    Some(crate::calc::recipe::RecipeIngredientInput::simple(
                        &i.food_id,
                        &i.food_name,
                        i.amount,
                        &i.unit,
                        food_tuple.1,
                    ))
                })
                .collect();

            let _ = storage.save_recipe(
                &recipe.recipe.user_id,
                Some(&recipe.recipe.id),
                &recipe.recipe.name,
                recipe.recipe.description.as_deref(),
                recipe.recipe.servings,
                Some(recipe.recipe.total_weight_g),
                &ingredient_inputs,
            );
            stats.recipes_imported += 1;
        }

        if let Some(goal) = &backup.goal {
            storage.set_user_goal(goal)?;
            stats.goals_imported += 1;
        }

        Ok(stats)
    }

    /// 从历史导出的 `user_intake.json` 文件恢复饮食历史

    /// 从历史导出的 `weight_log.json` 文件恢复体重历史

    /// 将用户的饮食摄入历史导出为 RFC-4180 标准 CSV 文件
    pub fn export_intakes_csv(
        storage: &StorageEngine,
        user_id: &str,
    ) -> Result<String, StorageError> {
        let intakes = storage.list_all_intakes(user_id)?;
        let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);

        // RFC-4180 标准表头
        wtr.write_record([
            "id",
            "date_time",
            "type",
            "amount",
            "unit",
            "meal_code",
            "meal_name",
            "meal_brands",
            "meal_source",
            "meal_quantity",
            "meal_unit",
            "meal_serving_quantity",
            "meal_serving_unit",
            "meal_serving_size",
            "meal_thumbnail_url",
            "meal_main_image_url",
            "meal_url",
            "kcal_per_100g",
            "carbs_per_100g",
            "fat_per_100g",
            "protein_per_100g",
            "sugars_per_100g",
            "saturated_fat_per_100g",
            "fiber_per_100g",
            "monounsaturated_fat_per_100g",
            "polyunsaturated_fat_per_100g",
            "trans_fat_per_100g",
            "cholesterol_per_100g",
            "sodium_per_100g",
            "potassium_per_100g",
            "magnesium_per_100g",
            "calcium_per_100g",
            "iron_per_100g",
            "zinc_per_100g",
            "phosphorus_per_100g",
            "vitamin_a_per_100g",
            "vitamin_c_per_100g",
            "vitamin_d_per_100g",
            "vitamin_b6_per_100g",
            "vitamin_b12_per_100g",
            "niacin_per_100g",
        ])
        .map_err(|e| StorageError::Conversion(format!("CSV 表头写入错误: {}", e)))?;

        for item in intakes {
            let food_details = storage.get_food_with_nutriments(&item.food_id)?;
            let (food, n) = match food_details {
                Some((f, nutr)) => (Some(f), Some(nutr)),
                None => (None, None),
            };

            let opt_num = |val: Option<f64>| val.map(|v| v.to_string()).unwrap_or_default();

            wtr.write_record([
                item.id.as_str(),
                item.consumed_at.as_str(),
                item.meal_type.as_str(),
                item.amount.to_string().as_str(),
                item.unit.as_str(),
                food.as_ref().map(|f| f.id.as_str()).unwrap_or_default(),
                item.food_name.as_str(),
                food.as_ref().and_then(|f| f.brand.as_deref()).unwrap_or_default(),
                food.as_ref().map(|f| f.source.as_str()).unwrap_or("custom"),
                "",
                "",
                opt_num(food.as_ref().and_then(|f| f.serving_quantity)).as_str(),
                food.as_ref().and_then(|f| f.serving_unit.as_deref()).unwrap_or_default(),
                "",
                "",
                food.as_ref().and_then(|f| f.image_url.as_deref()).unwrap_or_default(),
                "",
                n.as_ref().map(|x| x.energy_kcal_100.to_string()).unwrap_or_default().as_str(),
                n.as_ref().map(|x| x.carbohydrates_100.to_string()).unwrap_or_default().as_str(),
                n.as_ref().map(|x| x.fat_100.to_string()).unwrap_or_default().as_str(),
                n.as_ref().map(|x| x.proteins_100.to_string()).unwrap_or_default().as_str(),
                opt_num(n.as_ref().and_then(|x| x.sugars_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.saturated_fat_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.fiber_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.monounsaturated_fat_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.polyunsaturated_fat_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.trans_fat_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.cholesterol_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.sodium_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.potassium_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.magnesium_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.calcium_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.iron_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.zinc_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.phosphorus_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.vitamin_a_ug_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.vitamin_c_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.vitamin_d_ug_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.vitamin_b6_mg_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.vitamin_b12_ug_100)).as_str(),
                opt_num(n.as_ref().and_then(|x| x.niacin_mg_100)).as_str(),
            ])
            .map_err(|e| StorageError::Conversion(format!("CSV 写入记录错误: {}", e)))?;
        }

        let bytes = wtr.into_inner().map_err(|e| {
            StorageError::Conversion(format!("CSV 序列化错误: {}", e))
        })?;
        String::from_utf8(bytes).map_err(|e| {
            StorageError::Conversion(format!("UTF-8 字符集转换错误: {}", e))
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{FoodRecord, FoodSource, MealType, Nutriments100g};

    #[test]
    fn test_native_backup_and_restore() {
        let storage1 = StorageEngine::open_in_memory().unwrap();
        let user_id = "user-native-backup";

        storage1.insert_user_raw(&UserProfileRecord {
            id: user_id.into(),
            name: "Alice".into(),
            birthday: "1995-06-15".into(),
            height_cm: 168.0,
            weight_kg: 62.0,
            gender: "female".into(),
            hormone_profile: None,
            activity_level: "active".into(),
        }).unwrap();

        let apple = FoodRecord {
            id: "apple-1".into(),
            name: "Fuji Apple".into(),
            brand: Some("Fresh Farm".into()),
            source: FoodSource::Custom,
            serving_quantity: Some(150.0),
            serving_unit: Some("g".into()),
            image_url: None,
        };
        storage1.insert_food(&apple, &Nutriments100g::simple(52.0, 14.0, 0.3, 0.2)).unwrap();

        storage1.log_intake(user_id, "apple-1", 150.0, "g", MealType::Snack, "2026-05-14T15:00:00").unwrap();
        storage1.log_water(user_id, 500, "2026-05-14T10:00:00").unwrap();
        storage1.set_user_goal(&UserGoalRecord {
            id: "goal-alice".into(),
            user_id: user_id.into(),
            kind: "lose_weight".into(),
            target_weight_kg: Some(58.0),
            weekly_rate_kg: -0.5,
            taper_enabled: true,
            adaptive_enabled: true,
            manual_calorie_offset: 0.0,
            created_at: "2026-05-14T08:00:00Z".into(),
            updated_at: "2026-05-14T08:00:00Z".into(),
        }).unwrap();

        // 1. 导出备份
        let backup = ExportImportEngine::export_native_backup(&storage1, user_id).unwrap();
        assert_eq!(backup.intakes.len(), 1);
        assert_eq!(backup.waters.len(), 1);
        assert!(backup.goal.is_some());

        // 2. 导入到全新空数据库中
        let storage2 = StorageEngine::open_in_memory().unwrap();
        let stats = ExportImportEngine::import_native_backup(&storage2, &backup).unwrap();
        assert_eq!(stats.users_imported, 1);
        assert_eq!(stats.intakes_imported, 1);
        assert_eq!(stats.waters_imported, 1);
        assert_eq!(stats.goals_imported, 1);

        let user = storage2.get_user(user_id).unwrap().unwrap();
        assert_eq!(user.name, "Alice");
        let restored_goal = storage2.get_user_goal(user_id).unwrap().unwrap();
        assert_eq!(restored_goal.kind, "lose_weight");
        assert_eq!(restored_goal.target_weight_kg, Some(58.0));
        let daily_water = storage2.get_daily_water(user_id, "2026-05-14", 2000).unwrap();
        assert_eq!(daily_water.total_consumed_ml, 500);
    }

    #[test]
    fn test_csv_export_contains_headers_and_rows() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user_id = "user-csv-export";
        storage
            .insert_user_raw(&UserProfileRecord {
                id: user_id.into(),
                name: "Bob".into(),
                birthday: "1990-01-01".into(),
                height_cm: 175.0,
                weight_kg: 70.0,
                gender: "male".into(),
                hormone_profile: None,
                activity_level: "moderate".into(),
            })
            .unwrap();
        let food = FoodRecord {
            id: "rice-1".into(),
            name: "Rice".into(),
            brand: None,
            source: FoodSource::Custom,
            serving_quantity: Some(100.0),
            serving_unit: Some("g".into()),
            image_url: None,
        };
        storage
            .insert_food(&food, &Nutriments100g::simple(130.0, 28.0, 2.7, 0.3))
            .unwrap();
        storage
            .log_intake(user_id, "rice-1", 100.0, "g", MealType::Lunch, "2026-05-14T12:00:00")
            .unwrap();
        let csv = ExportImportEngine::export_intakes_csv(&storage, user_id).unwrap();
        assert!(csv.contains("meal_name"));
        assert!(csv.contains("Rice"));
        assert!(csv.contains("kcal_per_100g"));
    }
}
