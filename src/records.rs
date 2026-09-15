use serde::{Deserialize, Serialize};

/// 食品数据源归属
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoodSource {
    /// 全球开源食品库 Open Food Facts
    OpenFoodFacts,
    /// 美国农业部官方食物数据中心 USDA FoodData Central
    UsdaFoodDataCentral,
    /// 用户本地自定义录入食物
    Custom,
    /// 用户自制聚合烹饪食谱
    Recipe,
}

impl FoodSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            FoodSource::OpenFoodFacts => "off",
            FoodSource::UsdaFoodDataCentral => "fdc",
            FoodSource::Custom => "custom",
            FoodSource::Recipe => "recipe",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "off" => FoodSource::OpenFoodFacts,
            "fdc" => FoodSource::UsdaFoodDataCentral,
            "recipe" => FoodSource::Recipe,
            _ => FoodSource::Custom,
        }
    }
}

/// 食品基础元数据记录（对应 SQLite `foods` 表）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FoodRecord {
    /// 条形码（EAN-13/UPC）或 UUID
    pub id: String,
    /// 食品名称
    pub name: String,
    /// 生产厂商 / 品牌
    pub brand: Option<String>,
    /// 数据来源渠道
    pub source: FoodSource,
    /// 单份推荐份量（如 30.0）
    pub serving_quantity: Option<f64>,
    /// 份量单位（如 "g", "ml", "serving"）
    pub serving_unit: Option<String>,
    /// 食品主图或缩略图网络地址 / 本地相对路径
    pub image_url: Option<String>,
}

/// 归一化营养素档案（严格按每 100g 或 100ml 标定，未知微量元素保持 None 避免数据稀释污染）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Nutriments100g {
    /// 热量（千卡/100g）
    pub energy_kcal_100: f64,
    /// 碳水化合物（克/100g）
    pub carbohydrates_100: f64,
    /// 蛋白质（克/100g）
    pub proteins_100: f64,
    /// 脂肪（克/100g）
    pub fat_100: f64,
    /// 糖类（克/100g）
    pub sugars_100: Option<f64>,
    /// 饱和脂肪酸（克/100g）
    pub saturated_fat_100: Option<f64>,
    /// 膳食纤维（克/100g）
    pub fiber_100: Option<f64>,
    /// 单不饱和脂肪酸（克/100g）
    pub monounsaturated_fat_100: Option<f64>,
    /// 多不饱和脂肪酸（克/100g）
    pub polyunsaturated_fat_100: Option<f64>,
    /// 反式脂肪酸（克/100g）
    pub trans_fat_100: Option<f64>,
    /// 胆固醇（毫克/100g）
    pub cholesterol_mg_100: Option<f64>,
    /// 钠（毫克/100g）
    pub sodium_mg_100: Option<f64>,
    /// 钾（毫克/100g）
    pub potassium_mg_100: Option<f64>,
    /// 镁（毫克/100g）
    pub magnesium_mg_100: Option<f64>,
    /// 钙（毫克/100g）
    pub calcium_mg_100: Option<f64>,
    /// 铁（毫克/100g）
    pub iron_mg_100: Option<f64>,
    /// 锌（毫克/100g）
    pub zinc_mg_100: Option<f64>,
    /// 磷（毫克/100g）
    pub phosphorus_mg_100: Option<f64>,
    /// 维生素 A（微克 RAE/100g）
    pub vitamin_a_ug_100: Option<f64>,
    /// 维生素 C（毫克/100g）
    pub vitamin_c_mg_100: Option<f64>,
    /// 维生素 D（微克/100g）
    pub vitamin_d_ug_100: Option<f64>,
    /// 维生素 B6（毫克/100g）
    pub vitamin_b6_mg_100: Option<f64>,
    /// 维生素 B12（微克/100g）
    pub vitamin_b12_ug_100: Option<f64>,
    /// 烟酸 / B3（毫克 NE/100g）
    pub niacin_mg_100: Option<f64>,
}

impl Nutriments100g {
    pub fn simple(kcal: f64, carbs: f64, protein: f64, fat: f64) -> Self {
        Self {
            energy_kcal_100: kcal,
            carbohydrates_100: carbs,
            proteins_100: protein,
            fat_100: fat,
            sugars_100: None,
            saturated_fat_100: None,
            fiber_100: None,
            monounsaturated_fat_100: None,
            polyunsaturated_fat_100: None,
            trans_fat_100: None,
            cholesterol_mg_100: None,
            sodium_mg_100: None,
            potassium_mg_100: None,
            magnesium_mg_100: None,
            calcium_mg_100: None,
            iron_mg_100: None,
            zinc_mg_100: None,
            phosphorus_mg_100: None,
            vitamin_a_ug_100: None,
            vitamin_c_mg_100: None,
            vitamin_d_ug_100: None,
            vitamin_b6_mg_100: None,
            vitamin_b12_ug_100: None,
            niacin_mg_100: None,
        }
    }
}

/// 自定义食谱表头记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeRecord {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub servings: f64,
    pub total_weight_g: f64,
    pub created_at: String,
}

/// 自定义食谱中的单项原料明细
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeIngredientRecord {
    pub id: String,
    pub recipe_id: String,
    pub food_id: String,
    pub food_name: String,
    pub amount: f64,
    pub unit: String,
    pub converted_amount_g: f64,
}

/// 包含原料明细及归一化每100g/每份营养数据的完整食谱实体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeWithDetails {
    pub recipe: RecipeRecord,
    pub ingredients: Vec<RecipeIngredientRecord>,
    pub per_100g: Nutriments100g,
    pub per_serving: Nutriments100g,
}

/// 餐次分类枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MealType {
    Breakfast,
    Lunch,
    Dinner,
    Snack,
}

impl MealType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MealType::Breakfast => "breakfast",
            MealType::Lunch => "lunch",
            MealType::Dinner => "dinner",
            MealType::Snack => "snack",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "breakfast" => MealType::Breakfast,
            "lunch" => MealType::Lunch,
            "dinner" => MealType::Dinner,
            _ => MealType::Snack,
        }
    }
}

/// 不可篡改的单笔餐饮摄入日志快照记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntakeLogRecord {
    pub id: String,
    pub user_id: String,
    pub food_id: String,
    pub food_name: String,
    pub amount: f64,
    pub unit: String,
    pub meal_type: MealType,
    pub consumed_at: String,
    pub snapshot_energy_kcal: f64,
    pub snapshot_protein_g: f64,
    pub snapshot_carbs_g: f64,
    pub snapshot_fat_g: f64,
}

/// 单日饮食聚合总览
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DailySummary {
    pub date: String,
    pub total_energy_kcal: f64,
    pub total_protein_g: f64,
    pub total_carbs_g: f64,
    pub total_fat_g: f64,
    pub items_count: usize,
    pub logs: Vec<IntakeLogRecord>,
}

/// 包含完整 24 项营养素档案的食物条目
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FoodWithNutriments {
    pub food: FoodRecord,
    pub nutriments: Nutriments100g,
}

/// 用户身体属性与活动水平数据库持久化记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfileRecord {
    pub id: String,
    pub name: String,
    pub birthday: String,
    pub height_cm: f64,
    pub weight_kg: f64,
    pub gender: String,
    pub hormone_profile: Option<String>,
    pub activity_level: String,
}

/// 体重称重历史记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeightLogRecord {
    pub id: String,
    pub user_id: String,
    pub logged_at: String,
    pub weight_kg: f64,
    pub body_fat_pct: Option<f64>,
    pub notes: Option<String>,
}

/// 饮水打卡记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaterLogRecord {
    pub id: String,
    pub user_id: String,
    pub logged_at: String,
    pub amount_ml: i64,
}

/// 间歇性断食会话记录
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FastingSessionRecord {
    pub id: String,
    pub user_id: String,
    pub started_at: String,
    pub target_duration_minutes: i64,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
}

/// 运动与体力活动打卡记录（对应 SQLite `activity_logs` 表）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityLogRecord {
    pub id: String,
    pub user_id: String,
    pub activity_code: String,
    pub activity_name: String,
    pub category: String,
    pub duration_minutes: f64,
    pub met_value: f64,
    pub gross_burned_kcal: f64,
    pub net_credited_kcal: f64,
    pub compensation_ratio: f64,
    pub date: String,
    pub created_at: String,
    #[serde(default)]
    pub external_id: Option<String>,
}

/// 用户体态与体重目标持久化记录（对应 SQLite `user_goals` 表）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserGoalRecord {
    pub id: String,
    pub user_id: String,
    pub kind: String,
    pub target_weight_kg: Option<f64>,
    pub weekly_rate_kg: f64,
    pub taper_enabled: bool,
    pub adaptive_enabled: bool,
    pub manual_calorie_offset: f64,
    pub created_at: String,
    pub updated_at: String,
}
