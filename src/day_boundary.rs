//! 跨午夜生理日界线算法 (day_boundary.rs)
//!
//! 解决夜猫子与夜班人群跨午夜进食/运动归档痛点：
//! - 传统软件按自然日 00:00 刀切，导致凌晨 2 点的夜宵被强制割裂至次日，造成单日摄入统计失真。
//! - 本模块支持用户自定义“生理日界线时刻”（例如 04:00 AM，即偏移 240 分钟）。
//! - 凡在该时间界线之前的打卡（如凌晨 02:30），均按照生理连续清醒周期归集到前一个逻辑日。
//! - 提供严格的“纯日期标签”防漂移机制与 SQLite 检索时间范围计算器。

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// 生理日界线配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DayBoundaryConfig {
    /// 跨午夜生理日界线偏移总分钟数（0 ~ 1439，默认为 0 即自然日 00:00）
    pub offset_total_minutes: u32,
}

impl DayBoundaryConfig {
    /// 通过小时与分钟创建生理日界线配置
    pub fn new(hours: u32, minutes: u32) -> Self {
        let total = hours * 60 + minutes.min(59);
        Self {
            offset_total_minutes: total.min(1439),
        }
    }

    /// 解析字符串格式的时间点（如 "04:00", "4:30", "4"）
    pub fn from_str_loose(s: &str) -> Self {
        let trimmed = s.trim();
        if let Ok(naive_time) = NaiveTime::parse_from_str(trimmed, "%H:%M") {
            return Self::new(naive_time.hour(), naive_time.minute());
        }
        if let Ok(naive_time) = NaiveTime::parse_from_str(trimmed, "%H:%M:%S") {
            return Self::new(naive_time.hour(), naive_time.minute());
        }
        if let Ok(hours) = trimmed.parse::<u32>() {
            return Self::new(hours, 0);
        }
        Self::default()
    }

    /// 返回格式化为 "HH:MM" 的字符串表示
    pub fn formatted_time(&self) -> String {
        let h = self.offset_total_minutes / 60;
        let m = self.offset_total_minutes % 60;
        format!("{:02}:{:02}", h, m)
    }

    /// 是否为标准自然日零点界线
    pub fn is_midnight(&self) -> bool {
        self.offset_total_minutes == 0
    }
}

/// 生理日界线核心计算引擎
pub struct DayBoundaryEngine;

impl DayBoundaryEngine {
    /// 计算给定 UTC 瞬时时间戳在指定日界线偏移下的“逻辑归属日期”
    pub fn logical_date_of_utc(moment: &DateTime<Utc>, offset_minutes: u32) -> NaiveDate {
        let effective_minutes = offset_minutes.min(1439);
        let shifted = *moment - Duration::minutes(effective_minutes as i64);
        shifted.date_naive()
    }

    /// 计算给定本地无时区日期时间 (NaiveDateTime) 在指定日界线偏移下的“逻辑归属日期”
    pub fn logical_date_of_naive(moment: NaiveDateTime, offset_minutes: u32) -> NaiveDate {
        let effective_minutes = offset_minutes.min(1439);
        let shifted = moment - Duration::minutes(effective_minutes as i64);
        shifted.date()
    }

    /// 解析可能为纯日期（"YYYY-MM-DD"）或完整时间戳（ISO-8601）的字符串，并映射到逻辑日期。
    /// - 若字符串本身就是纯日期标签（长度为 10），则直接按字面解析，绝不二次回卷（防日历点击漂移！）；
    /// - 若包含具体时刻（长度 > 10），则结合日界线偏移执行回卷计算。
    pub fn parse_and_resolve_logical_date(iso_str: &str, offset_minutes: u32) -> Option<NaiveDate> {
        let s = iso_str.trim();
        if s.len() == 10 {
            return NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
        }

        // 尝试解析为 RFC3339 / ISO-8601 带时区时间戳
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let utc_dt: DateTime<Utc> = dt.with_timezone(&Utc);
            return Some(Self::logical_date_of_utc(&utc_dt, offset_minutes));
        }

        // 尝试解析为标准本地无时区时间格式 YYYY-MM-DDTHH:MM:SS
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return Some(Self::logical_date_of_naive(ndt, offset_minutes));
        }

        // 尝试解析前 19 位时间前缀
        s.get(..19)
            .and_then(|prefix| NaiveDateTime::parse_from_str(prefix, "%Y-%m-%dT%H:%M:%S").ok())
            .map(|ndt| Self::logical_date_of_naive(ndt, offset_minutes))
    }

    /// 判断两个瞬时时间戳在指定日界线规则下是否属于同一个“生理作息逻辑日”
    pub fn is_same_logical_day_utc(a: &DateTime<Utc>, b: &DateTime<Utc>, offset_minutes: u32) -> bool {
        Self::logical_date_of_utc(a, offset_minutes) == Self::logical_date_of_utc(b, offset_minutes)
    }

    /// 计算指定逻辑日期 (NaiveDate) 所对应的数据库检索时间区间 [start_str, end_str)
    /// 例如：逻辑日期为 2026-09-15，日界线为 04:00 (240分钟)
    /// 则对应检索区间为：`["2026-09-15T04:00:00", "2026-09-16T04:00:00")`
    pub fn get_logical_day_sql_range(logical_day: NaiveDate, offset_minutes: u32) -> (String, String) {
        let effective_minutes = offset_minutes.min(1439);
        let h = effective_minutes / 60;
        let m = effective_minutes % 60;

        let start_time = NaiveTime::from_hms_opt(h, m, 0).unwrap_or(NaiveTime::MIN);
        let start_dt = NaiveDateTime::new(logical_day, start_time);

        let next_day = logical_day.succ_opt().unwrap_or(logical_day);
        let end_dt = NaiveDateTime::new(next_day, start_time);

        (
            start_dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            end_dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_boundary_config_parsing() {
        let cfg = DayBoundaryConfig::from_str_loose("04:00");
        assert_eq!(cfg.offset_total_minutes, 240);
        assert_eq!(cfg.formatted_time(), "04:00");

        let cfg2 = DayBoundaryConfig::from_str_loose("5:30");
        assert_eq!(cfg2.offset_total_minutes, 330);

        let cfg3 = DayBoundaryConfig::from_str_loose("4");
        assert_eq!(cfg3.offset_total_minutes, 240);

        let cfg_default = DayBoundaryConfig::from_str_loose("invalid");
        assert_eq!(cfg_default.offset_total_minutes, 0);
    }

    #[test]
    fn test_logical_date_mapping_with_0400_boundary() {
        let offset = 240; // 04:00 AM

        // 1. 2026-09-16 02:30 AM (凌晨夜宵) -> 逻辑属于 2026-09-15
        let late_snack = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let logical_d1 = DayBoundaryEngine::logical_date_of_naive(late_snack, offset);
        assert_eq!(logical_d1, NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());

        // 2. 2026-09-16 03:59:59 AM (临界点前) -> 仍然属于 2026-09-15
        let before_limit = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(3, 59, 59)
            .unwrap();
        let logical_d2 = DayBoundaryEngine::logical_date_of_naive(before_limit, offset);
        assert_eq!(logical_d2, NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());

        // 3. 2026-09-16 04:00:00 AM (新日界线起始) -> 属于 2026-09-16
        let boundary_start = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(4, 0, 0)
            .unwrap();
        let logical_d3 = DayBoundaryEngine::logical_date_of_naive(boundary_start, offset);
        assert_eq!(logical_d3, NaiveDate::from_ymd_opt(2026, 9, 16).unwrap());

        // 4. 2026-09-16 23:30:00 PM (当夜早些时候) -> 属于 2026-09-16
        let late_night = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap();
        let logical_d4 = DayBoundaryEngine::logical_date_of_naive(late_night, offset);
        assert_eq!(logical_d4, NaiveDate::from_ymd_opt(2026, 9, 16).unwrap());
    }

    #[test]
    fn test_date_label_preservation_avoids_drift() {
        let offset = 240; // 04:00 AM
        // 纯日期字符串标签 "2026-09-15"
        let res = DayBoundaryEngine::parse_and_resolve_logical_date("2026-09-15", offset);
        assert_eq!(res, Some(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()));

        // 完整时间戳字符串 "2026-09-16T02:30:00" -> 回卷为 2026-09-15
        let res_stamp = DayBoundaryEngine::parse_and_resolve_logical_date("2026-09-16T02:30:00", offset);
        assert_eq!(res_stamp, Some(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()));
    }

    #[test]
    fn test_sql_range_generator() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        let (start, end) = DayBoundaryEngine::get_logical_day_sql_range(day, 240);
        assert_eq!(start, "2026-09-15T04:00:00");
        assert_eq!(end, "2026-09-16T04:00:00");

        // 00:00 自然日
        let (start0, end0) = DayBoundaryEngine::get_logical_day_sql_range(day, 0);
        assert_eq!(start0, "2026-09-15T00:00:00");
        assert_eq!(end0, "2026-09-16T00:00:00");
    }
}
