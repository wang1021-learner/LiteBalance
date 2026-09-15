use crate::records::Nutriments100g;
use serde::{Deserialize, Serialize};

/// 带有液体密度支持的食谱原料换算工具。
pub struct RecipeDensityConverter;

/// 兼容老代码引用的类型别名
pub type UnitConverter = RecipeDensityConverter;

impl RecipeDensityConverter {
    /// 默认回退密度（水密度基准，1.0 g/ml）。
    pub const WATER_DENSITY_G_PER_ML: f64 = 1.0;

    /// 常见食品液体参考密度 (g / ml)。
    pub const OLIVE_OIL_DENSITY_G_PER_ML: f64 = 0.918;
    pub const MILK_WHOLE_DENSITY_G_PER_ML: f64 = 1.032;
    pub const HONEY_DENSITY_G_PER_ML: f64 = 1.420;

    /// 将指定的数量和单位换算为克（g）。
    ///
    /// # 液体密度处理
    /// 对于容积体积单位（`ml`, `l`, `cl`, `dl`, `fl oz`）：
    /// - 若提供了 `density_g_per_ml`（例如橄榄油为 0.918），则 质量(g) = 容积(ml) * 密度。
    /// - 若 `density_g_per_ml` 为 `None`，则回退采用水的密度（1.0 g/ml）作为显式兜底策略。
    pub fn to_grams_with_density(
        amount: f64,
        unit: &str,
        serving_quantity_g: Option<f64>,
        density_g_per_ml: Option<f64>,
    ) -> Option<f64> {
        if amount <= 0.0 {
            return Some(0.0);
        }

        let density = density_g_per_ml.unwrap_or(Self::WATER_DENSITY_G_PER_ML);
        let normalized_unit = unit.trim().to_lowercase();

        match normalized_unit.as_str() {
            "g" | "gml" | "g/ml" => Some(amount),
            "kg" => Some(amount * 1000.0),
            "mg" => Some(amount / 1000.0),
            // 体积单位经由密度换算为克
            "ml" => Some(amount * density),
            "l" => Some(amount * 1000.0 * density),
            "cl" => Some(amount * 10.0 * density),
            "dl" => Some(amount * 100.0 * density),
            // 英制换算
            "oz" => Some(amount * 28.349523125),
            "fl oz" | "fl.oz" | "floz" => Some(amount * 29.5735295625 * density),
            "serving" => {
                let sq = serving_quantity_g?;
                if sq > 0.0 { Some(amount * sq) } else { None }
            }
            _ => None,
        }
    }

    /// 向后兼容的便捷换算方法（默认回退至水密度 1.0 g/ml）。
    pub fn to_grams(amount: f64, unit: &str, serving_quantity_g: Option<f64>) -> Option<f64> {
        Self::to_grams_with_density(amount, unit, serving_quantity_g, None)
    }
}

/// 考虑部分数据覆盖率的微量元素科学数值模型。
///
/// # 解决“部分缺失数据引发的虚假稀释与污染”难题：
/// - 在常见食物数据库中，多数原料仅标注了三大宏量，往往缺失锌、维生素 D 等微量检测。
/// - 假设某食谱包含 3 种原料（鸡胸肉 200g、西兰花 150g、橄榄油 10g），且仅鸡胸肉测定了锌（2mg/100g）：
///   - 若粗暴除以全菜总重（360g），等于隐式假定西兰花和油含 0mg 锌，导致计算浓度被虚假稀释；
///   - 若简单输出 `Option<f64>` 为 1.11mg，又会让前端误以为整道菜都做过完备的化验检测。
/// - `MicroValue` 明确将两者解耦：
///   1. `value`: **严格仅在具有化验实测数据的原料子集内**加权计算出的真实浓度。
///   2. `coverage_pct`: 实际贡献了化验数据的原料占食谱总重量的百分比（覆盖率）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum MicroValue {
    /// 食谱中没有任何原料包含该微量元素的实验室检测数据。
    #[default]
    Unknown,
    /// 仅部分原料拥有该微量元素的实验室数据。
    /// `value` 为已知原料内的营养素浓度，`coverage_pct` 为已知原料占总重量的质量百分比。
    Partial { value: f64, coverage_pct: f64 },
    /// 食谱中 100% 的原料（按重量计）均具备确切的化验数据。
    Complete(f64),
}

impl MicroValue {
    /// 返回该微量元素数值（不论是完全还是部分覆盖，只要有值即返回）。
    pub fn raw_value(&self) -> Option<f64> {
        match self {
            MicroValue::Unknown => None,
            MicroValue::Partial { value, .. } => Some(*value),
            MicroValue::Complete(v) => Some(*v),
        }
    }

    /// 返回该微量元素的数据覆盖率（0.0% 至 100.0%）。
    pub fn coverage_pct(&self) -> f64 {
        match self {
            MicroValue::Unknown => 0.0,
            MicroValue::Partial { coverage_pct, .. } => *coverage_pct,
            MicroValue::Complete(_) => 100.0,
        }
    }

    /// 根据客户端 UI 置信度阈值评估是否应在前端展示。
    ///
    /// # 产品交互展示策略
    /// * `Complete`: 100% 完整覆盖，高置信度正常展示。
    /// * `Partial (覆盖率 >= 阈值)`: 展示数值，但附带“部分原料估算”及覆盖率徽标。
    /// * `Partial (覆盖率 < 阈值)` 或 `Unknown`: 在界面中隐去或标为缺失，防止误导用户做出极端营养判断。
    pub fn display_status(&self, min_confidence_threshold_pct: f64) -> MicroDisplayStatus {
        match self {
            MicroValue::Complete(v) => MicroDisplayStatus::Verified(*v),
            MicroValue::Partial { value, coverage_pct } if *coverage_pct >= min_confidence_threshold_pct => {
                MicroDisplayStatus::Estimated {
                    value: *value,
                    coverage_pct: *coverage_pct,
                }
            }
            _ => MicroDisplayStatus::Hidden,
        }
    }
}

/// 前端 UI 展示决策状态。
#[derive(Debug, Clone, PartialEq)]
pub enum MicroDisplayStatus {
    Verified(f64),
    Estimated { value: f64, coverage_pct: f64 },
    Hidden,
}

/// 食谱原料输入项（支持液体密度与嵌套食谱覆盖率元数据）。
#[derive(Debug, Clone)]
pub struct RecipeIngredientInput {
    pub food_id: String,
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub serving_quantity_g: Option<f64>,
    pub density_g_per_ml: Option<f64>,
    pub nutriments: Nutriments100g,
    /// 用于嵌套食谱：记录各营养素字段在内部的真实覆盖率（0.0 到 1.0）。
    /// 若为空，则 `nutriments` 中所有 `Some` 字段均假定为 100%（1.0）完整覆盖。
    pub coverage_overrides: Option<std::collections::HashMap<String, f64>>,
}

impl RecipeIngredientInput {
    pub fn simple(food_id: &str, name: &str, amount: f64, unit: &str, nutriments: Nutriments100g) -> Self {
        Self {
            food_id: food_id.to_string(),
            name: name.to_string(),
            amount,
            unit: unit.to_string(),
            serving_quantity_g: None,
            density_g_per_ml: None,
            nutriments,
            coverage_overrides: None,
        }
    }
}

/// 食谱计算最终结果（包含每100g归一化指标与微量覆盖率元数据）。
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeComputationResult {
    pub total_weight_g: f64,
    pub servings: f64,
    pub ingredient_weights_g: Vec<f64>,
    pub per_100g: Nutriments100g,
    pub per_serving: Nutriments100g,
    pub micro_coverage: RecipeMicroCoverage,
}

/// 20 种微量与细分营养素的覆盖率追踪结构体。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecipeMicroCoverage {
    pub sugars: MicroValue,
    pub saturated_fat: MicroValue,
    pub fiber: MicroValue,
    pub monounsaturated_fat: MicroValue,
    pub polyunsaturated_fat: MicroValue,
    pub trans_fat: MicroValue,
    pub cholesterol: MicroValue,
    pub sodium: MicroValue,
    pub potassium: MicroValue,
    pub magnesium: MicroValue,
    pub calcium: MicroValue,
    pub iron: MicroValue,
    pub zinc: MicroValue,
    pub phosphorus: MicroValue,
    pub vitamin_a: MicroValue,
    pub vitamin_c: MicroValue,
    pub vitamin_d: MicroValue,
    pub vitamin_b6: MicroValue,
    pub vitamin_b12: MicroValue,
    pub niacin: MicroValue,
}

/// 单字段累加器：在单次循环遍历中追踪已知质量与加权求和。
#[derive(Default)]
struct FieldAccumulator {
    weighted_sum: f64,
    known_weight_g: f64,
}

impl FieldAccumulator {
    fn add(&mut self, val_per_100: Option<f64>, ingredient_weight_g: f64, coverage_factor: f64) {
        if let Some(val) = val_per_100 {
            let effective_known_g = ingredient_weight_g * coverage_factor.clamp(0.0, 1.0);
            if effective_known_g > 0.0 {
                self.known_weight_g += effective_known_g;
                self.weighted_sum += (val * effective_known_g) / 100.0;
            }
        }
    }

    fn to_micro_value(&self, total_weight_g: f64) -> MicroValue {
        if self.known_weight_g <= 0.0 || total_weight_g <= 0.0 {
            return MicroValue::Unknown;
        }

        let coverage = (self.known_weight_g / total_weight_g).min(1.0);
        // 科学浓度计算：已知质量加权和 / 已知总质量 * 100.0
        let concentration = (self.weighted_sum / self.known_weight_g) * 100.0;
        let rounded_val = (concentration * 100.0).round() / 100.0;
        let rounded_cov = (coverage * 1000.0).round() / 10.0;

        if (coverage - 1.0).abs() < 1e-4 {
            MicroValue::Complete(rounded_val)
        } else {
            MicroValue::Partial {
                value: rounded_val,
                coverage_pct: rounded_cov,
            }
        }
    }

    fn to_serving_value(&self, servings: f64) -> Option<f64> {
        if self.known_weight_g > 0.0 && servings > 0.0 {
            Some(((self.weighted_sum / servings) * 100.0).round() / 100.0)
        } else {
            None
        }
    }
}

/// 单次遍历累加器：在 O(N) 单重循环中累加三大宏量与 20 种微量营养素。
#[derive(Default)]
struct RecipeAccumulator {
    energy_kcal: f64,
    carbs_g: f64,
    protein_g: f64,
    fat_g: f64,

    sugars: FieldAccumulator,
    sat_fat: FieldAccumulator,
    fiber: FieldAccumulator,
    mono_fat: FieldAccumulator,
    poly_fat: FieldAccumulator,
    trans_fat: FieldAccumulator,
    cholesterol: FieldAccumulator,
    sodium: FieldAccumulator,
    potassium: FieldAccumulator,
    magnesium: FieldAccumulator,
    calcium: FieldAccumulator,
    iron: FieldAccumulator,
    zinc: FieldAccumulator,
    phosphorus: FieldAccumulator,
    vit_a: FieldAccumulator,
    vit_c: FieldAccumulator,
    vit_d: FieldAccumulator,
    vit_b6: FieldAccumulator,
    vit_b12: FieldAccumulator,
    niacin: FieldAccumulator,
}

impl RecipeAccumulator {
    fn add_ingredient(&mut self, ing: &RecipeIngredientInput, weight_g: f64) {
        let ratio = weight_g / 100.0;
        self.energy_kcal += ing.nutriments.energy_kcal_100 * ratio;
        self.carbs_g += ing.nutriments.carbohydrates_100 * ratio;
        self.protein_g += ing.nutriments.proteins_100 * ratio;
        self.fat_g += ing.nutriments.fat_100 * ratio;

        let get_cov = |key: &str| -> f64 {
            ing.coverage_overrides
                .as_ref()
                .and_then(|m| m.get(key).copied())
                .unwrap_or(1.0)
        };

        let n = &ing.nutriments;
        self.sugars.add(n.sugars_100, weight_g, get_cov("sugars"));
        self.sat_fat
            .add(n.saturated_fat_100, weight_g, get_cov("saturated_fat"));
        self.fiber.add(n.fiber_100, weight_g, get_cov("fiber"));
        self.mono_fat
            .add(n.monounsaturated_fat_100, weight_g, get_cov("monounsaturated_fat"));
        self.poly_fat
            .add(n.polyunsaturated_fat_100, weight_g, get_cov("polyunsaturated_fat"));
        self.trans_fat.add(n.trans_fat_100, weight_g, get_cov("trans_fat"));
        self.cholesterol
            .add(n.cholesterol_mg_100, weight_g, get_cov("cholesterol"));
        self.sodium.add(n.sodium_mg_100, weight_g, get_cov("sodium"));
        self.potassium.add(n.potassium_mg_100, weight_g, get_cov("potassium"));
        self.magnesium.add(n.magnesium_mg_100, weight_g, get_cov("magnesium"));
        self.calcium.add(n.calcium_mg_100, weight_g, get_cov("calcium"));
        self.iron.add(n.iron_mg_100, weight_g, get_cov("iron"));
        self.zinc.add(n.zinc_mg_100, weight_g, get_cov("zinc"));
        self.phosphorus
            .add(n.phosphorus_mg_100, weight_g, get_cov("phosphorus"));
        self.vit_a.add(n.vitamin_a_ug_100, weight_g, get_cov("vitamin_a"));
        self.vit_c.add(n.vitamin_c_mg_100, weight_g, get_cov("vitamin_c"));
        self.vit_d.add(n.vitamin_d_ug_100, weight_g, get_cov("vitamin_d"));
        self.vit_b6.add(n.vitamin_b6_mg_100, weight_g, get_cov("vitamin_b6"));
        self.vit_b12.add(n.vitamin_b12_ug_100, weight_g, get_cov("vitamin_b12"));
        self.niacin.add(n.niacin_mg_100, weight_g, get_cov("niacin"));
    }
}

/// 烹饪食谱营养成分单次遍历聚合计算引擎（支持液体密度换算与缺失值覆盖率追踪）。
pub fn compute_recipe_nutrition(
    ingredients: &[RecipeIngredientInput],
    servings: f64,
    total_weight_override: Option<f64>,
) -> Result<RecipeComputationResult, String> {
    if ingredients.is_empty() {
        return Err("食谱原料列表不能为空，至少需包含一种原料".to_string());
    }

    let valid_servings = if servings > 0.0 { servings } else { 1.0 };
    let mut natural_total_weight = 0.0;
    let mut ingredient_weights_g = Vec::with_capacity(ingredients.len());
    let mut accumulator = RecipeAccumulator::default();

    for (i, ing) in ingredients.iter().enumerate() {
        let weight_g =
            UnitConverter::to_grams_with_density(ing.amount, &ing.unit, ing.serving_quantity_g, ing.density_g_per_ml)
                .ok_or_else(|| {
                format!(
                    "无法换算第 {} 种原料 ('{}') 的数量 {} (单位 '{}')",
                    i + 1,
                    ing.name,
                    ing.amount,
                    ing.unit
                )
            })?;

        ingredient_weights_g.push(weight_g);
        natural_total_weight += weight_g;
        accumulator.add_ingredient(ing, weight_g);
    }

    let final_weight_g = total_weight_override.unwrap_or(natural_total_weight);
    if final_weight_g <= 0.0 {
        return Err("食谱最终总重量必须大于0".to_string());
    }

    // 构建微量覆盖率地图
    let micro_coverage = RecipeMicroCoverage {
        sugars: accumulator.sugars.to_micro_value(final_weight_g),
        saturated_fat: accumulator.sat_fat.to_micro_value(final_weight_g),
        fiber: accumulator.fiber.to_micro_value(final_weight_g),
        monounsaturated_fat: accumulator.mono_fat.to_micro_value(final_weight_g),
        polyunsaturated_fat: accumulator.poly_fat.to_micro_value(final_weight_g),
        trans_fat: accumulator.trans_fat.to_micro_value(final_weight_g),
        cholesterol: accumulator.cholesterol.to_micro_value(final_weight_g),
        sodium: accumulator.sodium.to_micro_value(final_weight_g),
        potassium: accumulator.potassium.to_micro_value(final_weight_g),
        magnesium: accumulator.magnesium.to_micro_value(final_weight_g),
        calcium: accumulator.calcium.to_micro_value(final_weight_g),
        iron: accumulator.iron.to_micro_value(final_weight_g),
        zinc: accumulator.zinc.to_micro_value(final_weight_g),
        phosphorus: accumulator.phosphorus.to_micro_value(final_weight_g),
        vitamin_a: accumulator.vit_a.to_micro_value(final_weight_g),
        vitamin_c: accumulator.vit_c.to_micro_value(final_weight_g),
        vitamin_d: accumulator.vit_d.to_micro_value(final_weight_g),
        vitamin_b6: accumulator.vit_b6.to_micro_value(final_weight_g),
        vitamin_b12: accumulator.vit_b12.to_micro_value(final_weight_g),
        niacin: accumulator.niacin.to_micro_value(final_weight_g),
    };

    let scale_100 = 100.0 / final_weight_g;
    let per_100g = Nutriments100g {
        energy_kcal_100: ((accumulator.energy_kcal * scale_100) * 100.0).round() / 100.0,
        carbohydrates_100: ((accumulator.carbs_g * scale_100) * 100.0).round() / 100.0,
        proteins_100: ((accumulator.protein_g * scale_100) * 100.0).round() / 100.0,
        fat_100: ((accumulator.fat_g * scale_100) * 100.0).round() / 100.0,
        sugars_100: micro_coverage.sugars.raw_value(),
        saturated_fat_100: micro_coverage.saturated_fat.raw_value(),
        fiber_100: micro_coverage.fiber.raw_value(),
        monounsaturated_fat_100: micro_coverage.monounsaturated_fat.raw_value(),
        polyunsaturated_fat_100: micro_coverage.polyunsaturated_fat.raw_value(),
        trans_fat_100: micro_coverage.trans_fat.raw_value(),
        cholesterol_mg_100: micro_coverage.cholesterol.raw_value(),
        sodium_mg_100: micro_coverage.sodium.raw_value(),
        potassium_mg_100: micro_coverage.potassium.raw_value(),
        magnesium_mg_100: micro_coverage.magnesium.raw_value(),
        calcium_mg_100: micro_coverage.calcium.raw_value(),
        iron_mg_100: micro_coverage.iron.raw_value(),
        zinc_mg_100: micro_coverage.zinc.raw_value(),
        phosphorus_mg_100: micro_coverage.phosphorus.raw_value(),
        vitamin_a_ug_100: micro_coverage.vitamin_a.raw_value(),
        vitamin_c_mg_100: micro_coverage.vitamin_c.raw_value(),
        vitamin_d_ug_100: micro_coverage.vitamin_d.raw_value(),
        vitamin_b6_mg_100: micro_coverage.vitamin_b6.raw_value(),
        vitamin_b12_ug_100: micro_coverage.vitamin_b12.raw_value(),
        niacin_mg_100: micro_coverage.niacin.raw_value(),
    };

    let per_serving = Nutriments100g {
        energy_kcal_100: ((accumulator.energy_kcal / valid_servings) * 100.0).round() / 100.0,
        carbohydrates_100: ((accumulator.carbs_g / valid_servings) * 100.0).round() / 100.0,
        proteins_100: ((accumulator.protein_g / valid_servings) * 100.0).round() / 100.0,
        fat_100: ((accumulator.fat_g / valid_servings) * 100.0).round() / 100.0,
        sugars_100: accumulator.sugars.to_serving_value(valid_servings),
        saturated_fat_100: accumulator.sat_fat.to_serving_value(valid_servings),
        fiber_100: accumulator.fiber.to_serving_value(valid_servings),
        monounsaturated_fat_100: accumulator.mono_fat.to_serving_value(valid_servings),
        polyunsaturated_fat_100: accumulator.poly_fat.to_serving_value(valid_servings),
        trans_fat_100: accumulator.trans_fat.to_serving_value(valid_servings),
        cholesterol_mg_100: accumulator.cholesterol.to_serving_value(valid_servings),
        sodium_mg_100: accumulator.sodium.to_serving_value(valid_servings),
        potassium_mg_100: accumulator.potassium.to_serving_value(valid_servings),
        magnesium_mg_100: accumulator.magnesium.to_serving_value(valid_servings),
        calcium_mg_100: accumulator.calcium.to_serving_value(valid_servings),
        iron_mg_100: accumulator.iron.to_serving_value(valid_servings),
        zinc_mg_100: accumulator.zinc.to_serving_value(valid_servings),
        phosphorus_mg_100: accumulator.phosphorus.to_serving_value(valid_servings),
        vitamin_a_ug_100: accumulator.vit_a.to_serving_value(valid_servings),
        vitamin_c_mg_100: accumulator.vit_c.to_serving_value(valid_servings),
        vitamin_d_ug_100: accumulator.vit_d.to_serving_value(valid_servings),
        vitamin_b6_mg_100: accumulator.vit_b6.to_serving_value(valid_servings),
        vitamin_b12_ug_100: accumulator.vit_b12.to_serving_value(valid_servings),
        niacin_mg_100: accumulator.niacin.to_serving_value(valid_servings),
    };

    Ok(RecipeComputationResult {
        total_weight_g: final_weight_g,
        servings: valid_servings,
        ingredient_weights_g,
        per_100g,
        per_serving,
        micro_coverage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density_conversion_olive_oil() {
        // 10ml 橄榄油（密度 0.918 g/ml）= 9.18g
        let grams =
            UnitConverter::to_grams_with_density(10.0, "ml", None, Some(UnitConverter::OLIVE_OIL_DENSITY_G_PER_ML))
                .unwrap();
        assert!((grams - 9.18).abs() < 1e-4);

        // 未指定密度时，默认回退为水密度 (10.0g)
        let water_fallback = UnitConverter::to_grams(10.0, "ml", None).unwrap();
        assert_eq!(water_fallback, 10.0);
    }

    #[test]
    fn test_partial_coverage_zinc_not_diluted() {
        // 鸡胸肉 200g: 锌 = 2.0 mg / 100g
        let mut chicken_nutr = Nutriments100g::simple(165.0, 0.0, 31.0, 3.6);
        chicken_nutr.zinc_mg_100 = Some(2.0);

        // 西兰花 150g: 锌未测 (None)
        let broccoli_nutr = Nutriments100g::simple(34.0, 6.6, 2.8, 0.4);

        // 橄榄油 10ml (~9.18g): 锌未测 (None)
        let oil_nutr = Nutriments100g::simple(884.0, 0.0, 0.0, 100.0);

        let ingredients = vec![
            RecipeIngredientInput::simple("chicken", "鸡胸肉", 200.0, "g", chicken_nutr),
            RecipeIngredientInput::simple("broccoli", "西兰花", 150.0, "g", broccoli_nutr),
            RecipeIngredientInput {
                food_id: "oil".to_string(),
                name: "橄榄油".to_string(),
                amount: 10.0,
                unit: "ml".to_string(),
                serving_quantity_g: None,
                density_g_per_ml: Some(UnitConverter::OLIVE_OIL_DENSITY_G_PER_ML),
                nutriments: oil_nutr,
                coverage_overrides: None,
            },
        ];

        let res = compute_recipe_nutrition(&ingredients, 1.0, None).unwrap();
        // 总重量 = 200 + 150 + 9.18 = 359.18g
        assert!((res.total_weight_g - 359.18).abs() < 1e-2);

        // 锌仅在鸡胸肉中测定（359.18g 中占 200g = 覆盖率约 55.7%）
        match res.micro_coverage.zinc {
            MicroValue::Partial { value, coverage_pct } => {
                // 已知原料内的浓度严格为 2.0 mg/100g！
                // 不会被未测定的油和西兰花虚假稀释到 1.11 mg！
                assert_eq!(value, 2.0);
                assert!((coverage_pct - 55.7).abs() < 0.2);
            }
            _ => panic!("预期锌应为 Partial 部分覆盖"),
        }

        // 客户端 UI 展示状态评估（以 50% 阈值为例）：
        // 因覆盖率 55.7% >= 50%，展示 Estimated 且附带 55.7% 徽标
        let status = res.micro_coverage.zinc.display_status(50.0);
        match status {
            MicroDisplayStatus::Estimated { value, coverage_pct } => {
                assert_eq!(value, 2.0);
                assert!((coverage_pct - 55.7).abs() < 0.2);
            }
            _ => panic!("预期为 Estimated 状态"),
        }

        // 维生素 D：所有原料均完全未测 -> 必须为 Unknown
        assert_eq!(res.micro_coverage.vitamin_d, MicroValue::Unknown);
        assert_eq!(
            res.micro_coverage.vitamin_d.display_status(50.0),
            MicroDisplayStatus::Hidden
        );
    }

    #[test]
    fn test_nested_recipe_coverage_propagation() {
        // 假设食谱 A（鸡胸肉碗）总重 200g，其中锌覆盖率为 60%（120g 已知）
        let mut recipe_a_nutr = Nutriments100g::simple(150.0, 5.0, 20.0, 5.0);
        recipe_a_nutr.zinc_mg_100 = Some(1.8);

        let mut cov_map = std::collections::HashMap::new();
        cov_map.insert("zinc".to_string(), 0.60); // 继承 60% 覆盖率

        // 现在食谱 B 使用了 100g 食谱 A + 100g 白米饭（锌为 None）
        let rice_nutr = Nutriments100g::simple(130.0, 28.0, 2.7, 0.3);

        let ingredients = vec![
            RecipeIngredientInput {
                food_id: "recipe_a".to_string(),
                name: "食谱A".to_string(),
                amount: 100.0,
                unit: "g".to_string(),
                serving_quantity_g: None,
                density_g_per_ml: None,
                nutriments: recipe_a_nutr,
                coverage_overrides: Some(cov_map),
            },
            RecipeIngredientInput::simple("rice", "白米饭", 100.0, "g", rice_nutr),
        ];

        let res = compute_recipe_nutrition(&ingredients, 1.0, None).unwrap();
        assert_eq!(res.total_weight_g, 200.0);

        // 食谱 B 中锌的有效已知重量：
        // 100g 食谱 A * 0.60 = 60g 已知。
        // 食谱 B 总重 = 200g。
        // 食谱 B 中锌覆盖率 = 60 / 200 = 30.0%！
        match res.micro_coverage.zinc {
            MicroValue::Partial { value, coverage_pct } => {
                assert_eq!(value, 1.8);
                assert_eq!(coverage_pct, 30.0);
            }
            _ => panic!("预期为 Partial 覆盖"),
        }

        // 在 50% UI 置信度阈值下，30% 覆盖率被自动隐藏 (Hidden)
        assert_eq!(res.micro_coverage.zinc.display_status(50.0), MicroDisplayStatus::Hidden);
    }
}
