//! 本地自建食物全流程管理与归一化引擎 (custom_food.rs)
//!
//! 支持从食品包装标签高效录入自建食物：
//! 1. 双输入基准模式：
//!    - `Per100g`：每 100g / 100ml 标定输入；
//!    - `PerServing`：按包装单份（如每份 35g 含 180 kcal）录入，系统自动精确归一化为每 100g 标准数据；
//! 2. 践行“零污染”原则：
//!    未标注或未检测的微量元素严格保留为 `None`，杜绝因补 0 稀释整体健康雷达；
//! 3. 实体生成与本地库绑定：
//!    构造标准的 `FoodRecord` 与 `Nutriments100g`，便于持久化至 SQLite `foods` 与 FTS5 全文索引。

use crate::records::{FoodRecord, FoodSource, Nutriments100g};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 录入基准基数类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputBasis {
    /// 包装标定为每 100g 或每 100ml
    Per100g,
    /// 包装标定为每推荐单份（Serving）
    PerServing {
        /// 单份重量或体积（例如 35.0）
        serving_amount: f64,
        /// 单位（如 "g", "ml", "包", "片"）
        unit: String,
    },
}

/// 自建食品录入原材料草稿数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomFoodDraft {
    /// 食品名称（必填）
    pub name: String,
    /// 生产厂商 / 品牌（可选）
    pub brand: Option<String>,
    /// 用户自定义条形码（若无则自动生成以 "custom_" 为前缀的 UUID）
    pub barcode: Option<String>,
    /// 录入基准模式
    pub basis: Option<InputBasis>,

    // 核心能量与宏量素（基于录入基准模式的原始数值）
    pub energy_kcal: f64,
    pub proteins_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,

    // 可选细分素（若包装未标注则保持 None）
    pub sugars_g: Option<f64>,
    pub saturated_fat_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub sodium_mg: Option<f64>,
    pub potassium_mg: Option<f64>,
    pub calcium_mg: Option<f64>,
    pub iron_mg: Option<f64>,
    pub zinc_mg: Option<f64>,
    pub magnesium_mg: Option<f64>,
    pub vitamin_c_mg: Option<f64>,
    pub vitamin_d_ug: Option<f64>,
}

/// 归一化完成的自建食物输出结果（可直接插入存储层）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedCustomFood {
    pub food: FoodRecord,
    pub nutriments_100g: Nutriments100g,
}

/// 自建食品归一化核心引擎
pub struct CustomFoodEngine;

impl CustomFoodEngine {
    /// 校验并将草稿数据严格归一化为标准的每 100g 格式
    pub fn normalize_draft(draft: &CustomFoodDraft) -> Result<NormalizedCustomFood, String> {
        let name = draft.name.trim();
        if name.is_empty() {
            return Err("食品名称不能为空".to_string());
        }

        if draft.energy_kcal < 0.0 || draft.proteins_g < 0.0 || draft.carbs_g < 0.0 || draft.fat_g < 0.0 {
            return Err("能量与三大宏量素不能为负数".to_string());
        }

        // 计算缩放比例系数：scale = 100.0 / 基准克数
        let (scale_factor, serving_quantity, serving_unit) = match &draft.basis {
            Some(InputBasis::PerServing { serving_amount, unit }) => {
                if *serving_amount <= 0.0 {
                    return Err("单份重量/容量必须大于 0".to_string());
                }
                (100.0 / *serving_amount, Some(*serving_amount), Some(unit.clone()))
            }
            _ => (1.0, Some(100.0), Some("g".to_string())),
        };

        // 缩放宏量营养素
        let round_1dec = |v: f64| (v * 10.0).round() / 10.0;
        let energy_kcal_100 = round_1dec(draft.energy_kcal * scale_factor);
        let proteins_100 = round_1dec(draft.proteins_g * scale_factor);
        let carbohydrates_100 = round_1dec(draft.carbs_g * scale_factor);
        let fat_100 = round_1dec(draft.fat_g * scale_factor);

        let scale_opt = |opt: Option<f64>| opt.map(|v| round_1dec(v * scale_factor));

        let nutriments_100g = Nutriments100g {
            energy_kcal_100,
            carbohydrates_100,
            proteins_100,
            fat_100,
            sugars_100: scale_opt(draft.sugars_g),
            saturated_fat_100: scale_opt(draft.saturated_fat_g),
            fiber_100: scale_opt(draft.fiber_g),
            sodium_mg_100: scale_opt(draft.sodium_mg),
            potassium_mg_100: scale_opt(draft.potassium_mg),
            calcium_mg_100: scale_opt(draft.calcium_mg),
            iron_mg_100: scale_opt(draft.iron_mg),
            zinc_mg_100: scale_opt(draft.zinc_mg),
            magnesium_mg_100: scale_opt(draft.magnesium_mg),
            vitamin_c_mg_100: scale_opt(draft.vitamin_c_mg),
            vitamin_d_ug_100: scale_opt(draft.vitamin_d_ug),
            ..Default::default()
        };

        let food_id = match &draft.barcode {
            Some(bc) if !bc.trim().is_empty() => bc.trim().to_string(),
            _ => format!("custom_{}", Uuid::new_v4().simple()),
        };

        let food = FoodRecord {
            id: food_id,
            name: name.to_string(),
            brand: draft
                .brand
                .clone()
                .map(|b| b.trim().to_string())
                .filter(|b| !b.is_empty()),
            source: FoodSource::Custom,
            serving_quantity,
            serving_unit,
            image_url: None,
        };

        Ok(NormalizedCustomFood { food, nutriments_100g })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_per_serving_to_100g() {
        // 假设一份薯片为 35g：热量 180 kcal, 蛋白质 2.0g, 脂肪 10.0g, 碳水 20.0g, 钠 140mg
        let draft = CustomFoodDraft {
            name: "经典原味薯片".to_string(),
            brand: Some("乐事".to_string()),
            barcode: Some("6901234567890".to_string()),
            basis: Some(InputBasis::PerServing {
                serving_amount: 35.0,
                unit: "g".to_string(),
            }),
            energy_kcal: 180.0,
            proteins_g: 2.0,
            carbs_g: 20.0,
            fat_g: 10.0,
            sodium_mg: Some(140.0),
            calcium_mg: None, // 未检测/未标明，严禁补 0
            ..Default::default()
        };

        let normalized = CustomFoodEngine::normalize_draft(&draft).unwrap();
        assert_eq!(normalized.food.name, "经典原味薯片");
        assert_eq!(normalized.food.id, "6901234567890");
        assert_eq!(normalized.food.source, FoodSource::Custom);
        assert_eq!(normalized.food.serving_quantity, Some(35.0));

        // 比例系数 = 100 / 35 = 2.85714
        // 180 * 2.85714 = 514.3 kcal
        assert_eq!(normalized.nutriments_100g.energy_kcal_100, 514.3);
        // 2.0 * 2.85714 = 5.7g
        assert_eq!(normalized.nutriments_100g.proteins_100, 5.7);
        // 20.0 * 2.85714 = 57.1g
        assert_eq!(normalized.nutriments_100g.carbohydrates_100, 57.1);
        // 10.0 * 2.85714 = 28.6g
        assert_eq!(normalized.nutriments_100g.fat_100, 28.6);
        // 140 * 2.85714 = 400.0 mg
        assert_eq!(normalized.nutriments_100g.sodium_mg_100, Some(400.0));
        // 钙保持 None，未受 0 污染
        assert_eq!(normalized.nutriments_100g.calcium_mg_100, None);
    }

    #[test]
    fn test_normalize_per_100g_direct() {
        let draft = CustomFoodDraft {
            name: "自制希腊酸奶".to_string(),
            brand: None,
            barcode: None,
            basis: Some(InputBasis::Per100g),
            energy_kcal: 97.0,
            proteins_g: 10.3,
            carbs_g: 3.6,
            fat_g: 5.0,
            calcium_mg: Some(110.0),
            ..Default::default()
        };

        let normalized = CustomFoodEngine::normalize_draft(&draft).unwrap();
        assert!(normalized.food.id.starts_with("custom_"));
        assert_eq!(normalized.nutriments_100g.energy_kcal_100, 97.0);
        assert_eq!(normalized.nutriments_100g.proteins_100, 10.3);
        assert_eq!(normalized.nutriments_100g.calcium_mg_100, Some(110.0));
    }

    #[test]
    fn test_normalize_empty_name_fails() {
        let draft = CustomFoodDraft {
            name: "   ".to_string(),
            ..Default::default()
        };
        assert!(CustomFoodEngine::normalize_draft(&draft).is_err());
    }
}
