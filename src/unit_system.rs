//! 多单位制度量衡转换系统 (unit_system.rs)
//!
//! 提供临床级高精度公制（Metric）与英制（Imperial）双向度量衡无损转换：
//! - 能量单位：千卡 (kcal) <-> 千焦 (kJ)（遵循 USDA / EU 食品标签热化学转换常数 4.184）；
//! - 身高单位：厘米 (cm) <-> 英寸 (in) <-> 英尺与英寸组合 (feet & inches，如 5 ft 9 in)；
//! - 体重单位：千克 (kg) <-> 磅 (lbs) <-> 英石与磅组合 (stone & lbs，英式 1 st = 14 lb)；
//! - 食品份量：克 (g) <-> 盎司 (oz)，毫升 (ml) <-> 液体盎司 (fl oz)。

use serde::{Deserialize, Serialize};

/// 度量衡标准体系分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitSystem {
    /// 国际公制 (kg, cm, kcal, g, ml)
    Metric,
    /// 英美制 (lbs, in, kJ/kcal, oz, fl oz, stone)
    Imperial,
}

/// 高精度度量衡转换引擎
pub struct UnitConverter;

impl UnitConverter {
    /// 热化学转换系数：1 kcal = 4.184 kJ
    pub const KCAL_TO_KJ_FACTOR: f64 = 4.184;
    /// 质量转换系数：1 kg = 2.2046226218 lbs
    pub const KG_TO_LBS_FACTOR: f64 = 2.2046226218;
    /// 长度转换系数：1 inch = 2.54 cm
    pub const INCH_TO_CM_FACTOR: f64 = 2.54;
    /// 常衡盎司转换系数：1 oz = 28.349523125 g
    pub const OZ_TO_G_FACTOR: f64 = 28.349523125;
    /// 美制液体盎司转换系数：1 fl oz = 29.5735295625 ml
    pub const FL_OZ_TO_ML_FACTOR: f64 = 29.5735295625;

    // --- 能量转换 ---

    /// 千卡 (kcal) 转换为 千焦 (kJ)
    pub fn kcal_to_kj(kcal: f64) -> f64 {
        kcal * Self::KCAL_TO_KJ_FACTOR
    }

    /// 千焦 (kJ) 转换为 千卡 (kcal)
    pub fn kj_to_kcal(kj: f64) -> f64 {
        kj / Self::KCAL_TO_KJ_FACTOR
    }

    // --- 身高长度转换 ---

    /// 厘米 (cm) 转换为 英寸 (inches)
    pub fn cm_to_inches(cm: f64) -> f64 {
        cm / Self::INCH_TO_CM_FACTOR
    }

    /// 英寸 (inches) 转换为 厘米 (cm)
    pub fn inches_to_cm(inches: f64) -> f64 {
        inches * Self::INCH_TO_CM_FACTOR
    }

    /// 厘米 (cm) 转换为 英尺与英寸复合形式 (feet, inches)
    /// 自动处理 12 英寸进位（例如绝不会输出 5 ft 12 in，而是进位为 6 ft 0 in）
    pub fn cm_to_feet_inches(cm: f64) -> (u32, u32) {
        let total_inches = Self::cm_to_inches(cm);
        let mut feet = (total_inches / 12.0).floor() as u32;
        let mut inches = (total_inches - (feet as f64 * 12.0)).round() as u32;
        if inches >= 12 {
            feet += 1;
            inches = 0;
        }
        (feet, inches)
    }

    /// 英尺与英寸复合形式 (feet, inches) 转换为 厘米 (cm)
    pub fn feet_inches_to_cm(feet: u32, inches: u32) -> f64 {
        let total_inches = (feet as f64 * 12.0) + inches as f64;
        (Self::inches_to_cm(total_inches) * 10.0).round() / 10.0
    }

    /// 格式化为常用英制身高显示（如 "5 ft 9 in"）
    pub fn format_feet_inches(feet: u32, inches: u32) -> String {
        format!("{} ft {} in", feet, inches)
    }

    // --- 体重转换 ---

    /// 千克 (kg) 转换为 磅 (lbs)
    pub fn kg_to_lbs(kg: f64) -> f64 {
        kg * Self::KG_TO_LBS_FACTOR
    }

    /// 磅 (lbs) 转换为 千克 (kg)
    pub fn lbs_to_kg(lbs: f64) -> f64 {
        lbs / Self::KG_TO_LBS_FACTOR
    }

    /// 千克 (kg) 转换为 英石与磅复合形式 (stones, lbs)
    /// 1 stone = 14 lbs，自动处理 14 磅进位
    pub fn kg_to_stone_lbs(kg: f64) -> (u32, f64) {
        let total_lbs = Self::kg_to_lbs(kg);
        let mut stones = (total_lbs / 14.0).floor() as u32;
        let mut rem_lbs = ((total_lbs - stones as f64 * 14.0) * 10.0).round() / 10.0;
        if rem_lbs >= 14.0 {
            stones += 1;
            rem_lbs = 0.0;
        }
        (stones, rem_lbs)
    }

    /// 英石与磅复合形式 (stones, lbs) 转换为 千克 (kg)
    pub fn stone_lbs_to_kg(stones: u32, lbs: f64) -> f64 {
        let total_lbs = (stones as f64 * 14.0) + lbs;
        (Self::lbs_to_kg(total_lbs) * 100.0).round() / 100.0
    }

    /// 格式化为英美显示（如 "11 st 5.2 lb"）
    pub fn format_stone_lbs(stones: u32, lbs: f64) -> String {
        format!("{} st {:.1} lb", stones, lbs)
    }

    // --- 食物份量与容积转换 ---

    /// 克 (g) 转换为 盎司 (oz)
    pub fn g_to_oz(g: f64) -> f64 {
        g / Self::OZ_TO_G_FACTOR
    }

    /// 盎司 (oz) 转换为 克 (g)
    pub fn oz_to_g(oz: f64) -> f64 {
        oz * Self::OZ_TO_G_FACTOR
    }

    /// 毫升 (ml) 转换为 美制液体盎司 (fl oz)
    pub fn ml_to_fl_oz(ml: f64) -> f64 {
        ml / Self::FL_OZ_TO_ML_FACTOR
    }

    /// 美制液体盎司 (fl oz) 转换为 毫升 (ml)
    pub fn fl_oz_to_ml(fl_oz: f64) -> f64 {
        fl_oz * Self::FL_OZ_TO_ML_FACTOR
    }

    /// 智能通用转换：根据公制单位与数值输出英制数值及单位
    pub fn metric_to_imperial(value: f64, unit: &str) -> (f64, &'static str) {
        match unit.trim().to_lowercase().as_str() {
            "g" | "gram" | "克" => ((Self::g_to_oz(value) * 100.0).round() / 100.0, "oz"),
            "ml" | "milliliter" | "毫升" => ((Self::ml_to_fl_oz(value) * 100.0).round() / 100.0, "fl oz"),
            "kg" | "kilogram" | "公斤" => ((Self::kg_to_lbs(value) * 10.0).round() / 10.0, "lbs"),
            "cm" | "centimeter" | "厘米" => ((Self::cm_to_inches(value) * 10.0).round() / 10.0, "in"),
            "kcal" | "千卡" => ((Self::kcal_to_kj(value) * 10.0).round() / 10.0, "kJ"),
            _ => (value, "unit"),
        }
    }

    /// 智能通用转换：根据英制单位与数值输出公制数值及单位
    pub fn imperial_to_metric(value: f64, unit: &str) -> (f64, &'static str) {
        match unit.trim().to_lowercase().as_str() {
            "oz" | "ounce" | "盎司" => ((Self::oz_to_g(value) * 10.0).round() / 10.0, "g"),
            "fl oz" | "fl.oz" | "液体盎司" => ((Self::fl_oz_to_ml(value) * 10.0).round() / 10.0, "ml"),
            "lbs" | "lb" | "pound" | "磅" => ((Self::lbs_to_kg(value) * 10.0).round() / 10.0, "kg"),
            "in" | "inch" | "英寸" => ((Self::inches_to_cm(value) * 10.0).round() / 10.0, "cm"),
            "kj" | "千焦" => ((Self::kj_to_kcal(value) * 10.0).round() / 10.0, "kcal"),
            _ => (value, "unit"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_roundtrip() {
        let kcal = 2000.0;
        let kj = UnitConverter::kcal_to_kj(kcal);
        assert!((kj - 8368.0).abs() < 1e-3);
        let back = UnitConverter::kj_to_kcal(kj);
        assert!((back - 2000.0).abs() < 1e-3);
    }

    #[test]
    fn test_height_feet_inches_conversion() {
        // 175 cm 对应约 5 ft 9 in
        let (ft, inch) = UnitConverter::cm_to_feet_inches(175.0);
        assert_eq!((ft, inch), (5, 9));
        assert_eq!(UnitConverter::format_feet_inches(ft, inch), "5 ft 9 in");

        // 逆向折算回 cm
        let cm_back = UnitConverter::feet_inches_to_cm(5, 9);
        assert!((cm_back - 175.3).abs() < 0.2);

        // 进位测试：182.88 cm = 72 in = 6 ft 0 in (不能为 5 ft 12 in)
        let (ft6, in0) = UnitConverter::cm_to_feet_inches(182.88);
        assert_eq!((ft6, in0), (6, 0));
    }

    #[test]
    fn test_body_weight_conversion() {
        let kg = 80.0;
        let lbs = UnitConverter::kg_to_lbs(kg);
        assert!((lbs - 176.37).abs() < 0.1);

        let kg_back = UnitConverter::lbs_to_kg(lbs);
        assert!((kg_back - 80.0).abs() < 1e-4);

        // 80 kg 转换为 stone + lbs: 176.37 lbs / 14 = 12 st 8.37 lb
        let (st, rem_lb) = UnitConverter::kg_to_stone_lbs(kg);
        assert_eq!(st, 12);
        assert!((rem_lb - 8.4).abs() < 0.2);
    }

    #[test]
    fn test_food_portion_conversion() {
        // 100g 约为 3.53 oz
        let oz = UnitConverter::g_to_oz(100.0);
        assert!((oz - 3.527).abs() < 0.01);
        let g_back = UnitConverter::oz_to_g(oz);
        assert!((g_back - 100.0).abs() < 1e-4);

        // 250ml 约为 8.45 fl oz
        let fl_oz = UnitConverter::ml_to_fl_oz(250.0);
        assert!((fl_oz - 8.453).abs() < 0.01);
        let ml_back = UnitConverter::fl_oz_to_ml(fl_oz);
        assert!((ml_back - 250.0).abs() < 1e-4);
    }

    #[test]
    fn test_metric_to_imperial_dispatch() {
        let (val, unit) = UnitConverter::metric_to_imperial(100.0, "g");
        assert_eq!(unit, "oz");
        assert_eq!(val, 3.53);

        let (val_ml, unit_ml) = UnitConverter::metric_to_imperial(500.0, "ml");
        assert_eq!(unit_ml, "fl oz");
        assert_eq!(val_ml, 16.91);
    }
}
