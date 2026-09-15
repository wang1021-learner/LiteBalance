use chrono::NaiveDate;
use std::collections::HashSet;

/// 饮食/饮水目标达标连续打卡天数统计（连胜天数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreakStats {
    /// 截至当前日期（包含当天）的最新连续达标天数。
    pub current: usize,
    /// 在所评估历史时间窗口内的最长连续达标天数。
    pub longest: usize,
}

/// 计算当前与历史最长连续达标天数。
pub fn streak_stats(
    on_track_days: &HashSet<NaiveDate>,
    window_start: NaiveDate,
    today: NaiveDate,
) -> StreakStats {
    if today < window_start {
        return StreakStats {
            current: 0,
            longest: 0,
        };
    }

    let total_days = (today - window_start).num_days();

    // 1. 正向遍历计算历史最长连胜天数
    let mut longest = 0;
    let mut current_run = 0;
    for i in 0..=total_days {
        let day = window_start + chrono::Duration::days(i);
        if on_track_days.contains(&day) {
            current_run += 1;
            if current_run > longest {
                longest = current_run;
            }
        } else {
            current_run = 0;
        }
    }

    // 2. 从今天逆向回溯计算当前活跃连续打卡天数
    let mut current = 0;
    for i in 0..=total_days {
        let day = today - chrono::Duration::days(i);
        if on_track_days.contains(&day) {
            current += 1;
        } else {
            break;
        }
    }

    StreakStats { current, longest }
}

/// 基于历史体重记录推导的体重变化速率与达标预测。
#[derive(Debug, Clone, PartialEq)]
pub struct WeightProjection {
    /// 每周体重变化速率（kg/周，正数表示增重，负数表示减重）。
    pub rate_per_week_kg: f64,
    /// 预计达到 `target_kg` 所需的周数（仅在设定了目标且正向目标推进时返回）。
    pub weeks_to_target: Option<u32>,
}

/// 对 `(日期, 体重kg)` 样本数据点执行一元线性最小二乘法回归拟合。
/// 若有效数据点少于 2 个，则返回 `None`。
pub fn weight_projection(
    points: &[(NaiveDate, f64)],
    target_kg: Option<f64>,
) -> Option<WeightProjection> {
    if points.len() < 2 {
        return None;
    }

    let first_date = points[0].0;
    let n = points.len() as f64;

    let xs: Vec<f64> = points
        .iter()
        .map(|(d, _)| (*d - first_date).num_days() as f64)
        .collect();
    let ys: Vec<f64> = points.iter().map(|(_, kg)| *kg).collect();

    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for i in 0..points.len() {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        numerator += dx * dy;
        denominator += dx * dx;
    }

    let slope_per_day = if denominator.abs() < 1e-12 {
        0.0
    } else {
        numerator / denominator
    };

    let rate_per_week = (slope_per_day * 7.0 * 100.0).round() / 100.0;

    let mut weeks_to_target = None;
    if let Some(target) = target_kg
        && slope_per_day.abs() > 1e-6
    {
        let current_weight = ys[ys.len() - 1];
        let days_to_target = (target - current_weight) / slope_per_day;

        // 仅在朝向目标推进且在合理的 10 年时间跨度内时给出预测
        if days_to_target > 0.0 && days_to_target < 3650.0 {
            weeks_to_target = Some((days_to_target / 7.0).ceil() as u32);
        }
    }

    Some(WeightProjection {
        rate_per_week_kg: rate_per_week,
        weeks_to_target,
    })
}

/// 历史时间跨度内的宏量与热量摄入移动平均线。
#[derive(Debug, Clone, PartialEq)]
pub struct NutritionMovingAverage {
    pub days_count: usize,
    pub avg_daily_calories: f64,
    pub avg_daily_protein_g: f64,
    pub avg_daily_carbs_g: f64,
    pub avg_daily_fat_g: f64,
    pub protein_kcal_pct: f64,
    pub carbs_kcal_pct: f64,
    pub fat_kcal_pct: f64,
}

impl NutritionMovingAverage {
    /// 计算每日饮食记录的移动均值：(热量, 蛋白质, 碳水, 脂肪)。
    pub fn compute(daily_entries: &[(f64, f64, f64, f64)]) -> Option<Self> {
        if daily_entries.is_empty() {
            return None;
        }

        let n = daily_entries.len() as f64;
        let mut sum_kcal = 0.0;
        let mut sum_p = 0.0;
        let mut sum_c = 0.0;
        let mut sum_f = 0.0;

        for (kcal, p, c, f) in daily_entries {
            sum_kcal += *kcal;
            sum_p += *p;
            sum_c += *c;
            sum_f += *f;
        }

        let avg_daily_calories = (sum_kcal / n * 10.0).round() / 10.0;
        let avg_daily_protein_g = (sum_p / n * 10.0).round() / 10.0;
        let avg_daily_carbs_g = (sum_c / n * 10.0).round() / 10.0;
        let avg_daily_fat_g = (sum_f / n * 10.0).round() / 10.0;

        // 阿特沃特系数（Atwater factors）：蛋白质 4 kcal/g, 碳水 4 kcal/g, 脂肪 9 kcal/g
        let total_macro_kcal = (avg_daily_protein_g * 4.0)
            + (avg_daily_carbs_g * 4.0)
            + (avg_daily_fat_g * 9.0);

        let (protein_kcal_pct, carbs_kcal_pct, fat_kcal_pct) = if total_macro_kcal > 0.0 {
            (
                ((avg_daily_protein_g * 4.0 / total_macro_kcal * 100.0) * 10.0).round() / 10.0,
                ((avg_daily_carbs_g * 4.0 / total_macro_kcal * 100.0) * 10.0).round() / 10.0,
                ((avg_daily_fat_g * 9.0 / total_macro_kcal * 100.0) * 10.0).round() / 10.0,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        Some(Self {
            days_count: daily_entries.len(),
            avg_daily_calories,
            avg_daily_protein_g,
            avg_daily_carbs_g,
            avg_daily_fat_g,
            protein_kcal_pct,
            carbs_kcal_pct,
            fat_kcal_pct,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streak_stats() {
        let mut on_track = HashSet::new();
        let d1 = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 9, 3).unwrap();
        // 9月4日跳过未打卡
        let d5 = NaiveDate::from_ymd_opt(2026, 9, 5).unwrap();
        let d6 = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();

        on_track.insert(d1);
        on_track.insert(d2);
        on_track.insert(d3);
        on_track.insert(d5);
        on_track.insert(d6);

        // 窗口：9月1日至9月6日（今天）
        let stats = streak_stats(&on_track, d1, d6);
        // 截至今天（9月6日）的连续打卡天数为 2（d5, d6）
        assert_eq!(stats.current, 2);
        // 评估窗口内最长连续打卡天数为 3（d1, d2, d3）
        assert_eq!(stats.longest, 3);
    }

    #[test]
    fn test_weight_projection_weight_loss() {
        let d0 = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let d7 = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let d14 = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();

        // 14 天内 80.0kg -> 79.5kg -> 79.0kg（每周速率：-0.5 kg/周）
        let points = vec![(d0, 80.0), (d7, 79.5), (d14, 79.0)];

        // 目标：75.0 kg（还需减重 4kg = 按 -0.5 kg/周计算需 8 周）
        let proj = weight_projection(&points, Some(75.0)).unwrap();
        assert_eq!(proj.rate_per_week_kg, -0.5);
        assert_eq!(proj.weeks_to_target, Some(8));
    }

    #[test]
    fn test_weight_projection_moving_away_returns_none_target() {
        let d0 = NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();
        let d7 = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();

        // 正在增重：70kg -> 71kg
        let points = vec![(d0, 70.0), (d7, 71.0)];

        // 设定减重目标：65kg（与当前斜率方向背离）
        let proj = weight_projection(&points, Some(65.0)).unwrap();
        assert_eq!(proj.rate_per_week_kg, 1.0);
        // 方向背离时不应给出虚假达标周数承诺
        assert_eq!(proj.weeks_to_target, None);
    }

    #[test]
    fn test_nutrition_moving_average() {
        let days = vec![
            (2000.0, 150.0, 200.0, 66.7), // 第1天
            (2200.0, 160.0, 220.0, 75.0), // 第2天
        ];
        let ma = NutritionMovingAverage::compute(&days).unwrap();
        assert_eq!(ma.days_count, 2);
        assert_eq!(ma.avg_daily_calories, 2100.0);
        assert_eq!(ma.avg_daily_protein_g, 155.0);
        assert_eq!(ma.avg_daily_carbs_g, 210.0);
        assert_eq!(ma.avg_daily_fat_g, 70.9);
        assert!(ma.protein_kcal_pct > 0.0);
    }
}
