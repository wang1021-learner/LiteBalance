//! 外部食品数据库网络缓存存储引擎 (CacheStorageEngine)
//!
//! 物理隔离设计：
//! 本模块独立管理 `food_cache.db` 文件，专门存放 OpenFoodFacts (OFF) 等外部 API 查询
//! 结果的本地缓存与 TTL 生命周期，彻底隔离于用户私有数据主库 (`litebalance.db`)。
//!
//! 架构合规优势：
//! 1. 规避 ODbL 协议对用户私有数据库的潜在传染风险（数据零合并，物理分库）；
//! 2. 保证用户核心备份（`litebalance_backup.json`）轻量纯净，不随网络爬取缓存膨胀；
//! 3. 支持无痛随时清空、过期驱逐与重建，完全不影响用户个人饮食打卡与自建食谱。

use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::storage::error::StorageError;
use crate::storage::models::Nutriments100g;

/// 缓存食品记录数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFoodRecord {
    /// 原始扫描输入的条形码
    pub barcode: String,
    /// 规范化后的 13 位 EAN-13 条码
    pub ean13: String,
    /// 商品名称
    pub food_name: String,
    /// 品牌信息
    pub brand: Option<String>,
    /// 标准份量数值（如 100）
    pub serving_quantity: Option<f64>,
    /// 标准份量单位（如 g 或 ml）
    pub serving_unit: Option<String>,
    /// 清洗后的每 100g 标准营养素
    pub nutriments: Nutriments100g,
    /// 远程原始 JSON 响应（保留溯源与冷存储）
    pub raw_json: Option<String>,
    /// 缓存写入时间 (ISO-8601 UTC)
    pub cached_at: String,
    /// 缓存有效时长（秒），默认 30 天 (2,592,000 秒)
    pub ttl_seconds: i64,
    /// 数据版权与署名（合规要求：Open Food Facts under ODbL）
    pub attribution: String,
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存项总条目数
    pub total_entries: usize,
    /// 仍然处于有效 TTL 内的条目数
    pub active_entries: usize,
    /// 已过期的条目数
    pub expired_entries: usize,
}

/// 外部食品网络缓存数据库引擎
#[derive(Clone)]
pub struct CacheStorageEngine {
    conn: Arc<Mutex<Connection>>,
}

impl CacheStorageEngine {
    /// 打开或新建指定路径的独立缓存 SQLite 数据库文件。
    pub fn open(db_path: &str) -> Result<Self, StorageError> {
        let conn = Connection::open(db_path)?;
        Self::init_connection(conn)
    }

    /// 打开纯内存缓存数据库（主要用于单元测试与快速隔离运行）。
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        Self::init_connection(conn)
    }

    fn init_connection(conn: Connection) -> Result<Self, StorageError> {
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");

        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS remote_food_cache (
                barcode TEXT PRIMARY KEY,
                ean13 TEXT NOT NULL,
                food_name TEXT NOT NULL,
                brand TEXT,
                serving_quantity REAL,
                serving_unit TEXT,
                nutriments_json TEXT NOT NULL,
                raw_json TEXT,
                cached_at TEXT NOT NULL,
                ttl_seconds INTEGER NOT NULL DEFAULT 2592000,
                attribution TEXT NOT NULL DEFAULT 'Open Food Facts (ODbL)'
            );

            CREATE INDEX IF NOT EXISTS idx_cache_ean13 ON remote_food_cache(ean13);
            CREATE INDEX IF NOT EXISTS idx_cache_cached_at ON remote_food_cache(cached_at);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 根据条形码（或 EAN-13）查询未过期的本地缓存食品记录。
    /// 若命中但已超过 TTL 有效期，则返回 `None`。
    pub fn get_cached_food(&self, barcode_query: &str) -> Result<Option<CachedFoodRecord>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT barcode, ean13, food_name, brand, serving_quantity, serving_unit,
                   nutriments_json, raw_json, cached_at, ttl_seconds, attribution
            FROM remote_food_cache
            WHERE barcode = ?1 OR ean13 = ?1
            LIMIT 1
            "#,
        )?;

        let mut rows = stmt.query(params![barcode_query])?;
        if let Some(row) = rows.next()? {
            let cached_at_str: String = row.get(8)?;
            let ttl_seconds: i64 = row.get(9)?;

            // 校验是否在 TTL 有效期内
            if let Ok(cached_time) = DateTime::parse_from_rfc3339(&cached_at_str) {
                let now = Utc::now();
                let age_seconds = (now - cached_time.with_timezone(&Utc)).num_seconds();
                if age_seconds > ttl_seconds {
                    // 已过期，返回 None 触发上层联网重新获取
                    return Ok(None);
                }
            }

            let nutriments_json: String = row.get(6)?;
            let nutriments: Nutriments100g = serde_json::from_str(&nutriments_json)
                .map_err(|e| StorageError::Conversion(format!("解析营养素 JSON 失败: {}", e)))?;

            Ok(Some(CachedFoodRecord {
                barcode: row.get(0)?,
                ean13: row.get(1)?,
                food_name: row.get(2)?,
                brand: row.get(3)?,
                serving_quantity: row.get(4)?,
                serving_unit: row.get(5)?,
                nutriments,
                raw_json: row.get(7)?,
                cached_at: cached_at_str,
                ttl_seconds,
                attribution: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 将远程查询结果持久化写入本地缓存库中。
    pub fn save_cached_food(&self, record: &CachedFoodRecord) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let nutriments_json = serde_json::to_string(&record.nutriments)
            .map_err(|e| StorageError::Conversion(format!("序列化营养素 JSON 失败: {}", e)))?;

        conn.execute(
            r#"
            INSERT INTO remote_food_cache (
                barcode, ean13, food_name, brand, serving_quantity, serving_unit,
                nutriments_json, raw_json, cached_at, ttl_seconds, attribution
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(barcode) DO UPDATE SET
                ean13 = excluded.ean13,
                food_name = excluded.food_name,
                brand = excluded.brand,
                serving_quantity = excluded.serving_quantity,
                serving_unit = excluded.serving_unit,
                nutriments_json = excluded.nutriments_json,
                raw_json = excluded.raw_json,
                cached_at = excluded.cached_at,
                ttl_seconds = excluded.ttl_seconds,
                attribution = excluded.attribution
            "#,
            params![
                record.barcode,
                record.ean13,
                record.food_name,
                record.brand,
                record.serving_quantity,
                record.serving_unit,
                nutriments_json,
                record.raw_json,
                record.cached_at,
                record.ttl_seconds,
                record.attribution,
            ],
        )?;

        Ok(())
    }

    /// 驱逐并物理删除所有已过期的缓存条目。
    pub fn evict_expired(&self) -> Result<usize, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT barcode, cached_at, ttl_seconds FROM remote_food_cache")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        let now = Utc::now();
        let mut to_delete = Vec::new();
        for item in rows.flatten() {
            let (barcode, cached_at_str, ttl) = item;
            if let Ok(cached_time) = DateTime::parse_from_rfc3339(&cached_at_str)
                && (now - cached_time.with_timezone(&Utc)).num_seconds() > ttl
            {
                to_delete.push(barcode);
            }
        }

        let count = to_delete.len();
        for barcode in to_delete {
            conn.execute("DELETE FROM remote_food_cache WHERE barcode = ?1", params![barcode])?;
        }

        Ok(count)
    }

    /// 清空全部网络缓存数据（不影响任何用户核心私有数据）。
    pub fn clear_all(&self) -> Result<usize, StorageError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM remote_food_cache", [])?;
        Ok(count)
    }

    /// 获取当前外部缓存数据库的运行指标。
    pub fn get_cache_stats(&self) -> Result<CacheStats, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT cached_at, ttl_seconds FROM remote_food_cache")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        let now = Utc::now();
        let mut total = 0;
        let mut active = 0;
        let mut expired = 0;

        for item in rows.flatten() {
            total += 1;
            let (cached_at_str, ttl) = item;
            let is_expired = match DateTime::parse_from_rfc3339(&cached_at_str) {
                Ok(cached_time) => (now - cached_time.with_timezone(&Utc)).num_seconds() > ttl,
                Err(_) => true,
            };

            if is_expired {
                expired += 1;
            } else {
                active += 1;
            }
        }

        Ok(CacheStats {
            total_entries: total,
            active_entries: active,
            expired_entries: expired,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_storage_lifecycle_and_ttl() {
        let storage = CacheStorageEngine::open_in_memory().unwrap();

        let record = CachedFoodRecord {
            barcode: "3079970000058".to_string(),
            ean13: "3079970000058".to_string(),
            food_name: "Evian Natural Mineral Water".to_string(),
            brand: Some("Evian 依云".to_string()),
            serving_quantity: Some(1000.0),
            serving_unit: Some("ml".to_string()),
            nutriments: Nutriments100g::simple(0.0, 0.0, 0.0, 0.0),
            raw_json: Some(r#"{"status":1}"#.to_string()),
            cached_at: Utc::now().to_rfc3339(),
            ttl_seconds: 3600,
            attribution: "Open Food Facts (ODbL)".to_string(),
        };

        // 1. 保存缓存
        storage.save_cached_food(&record).unwrap();

        // 2. 查询缓存
        let found = storage.get_cached_food("3079970000058").unwrap();
        assert!(found.is_some());
        let item = found.unwrap();
        assert_eq!(item.food_name, "Evian Natural Mineral Water");
        assert_eq!(item.attribution, "Open Food Facts (ODbL)");

        // 3. 统计检查
        let stats = storage.get_cache_stats().unwrap();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.active_entries, 1);
        assert_eq!(stats.expired_entries, 0);

        // 4. 模拟已过期条目
        let expired_record = CachedFoodRecord {
            barcode: "0000000000000".to_string(),
            ean13: "0000000000000".to_string(),
            food_name: "Expired Snack".to_string(),
            brand: None,
            serving_quantity: None,
            serving_unit: None,
            nutriments: Nutriments100g::simple(100.0, 10.0, 2.0, 5.0),
            raw_json: None,
            cached_at: "2020-01-01T00:00:00Z".to_string(),
            ttl_seconds: 60,
            attribution: "Open Food Facts (ODbL)".to_string(),
        };
        storage.save_cached_food(&expired_record).unwrap();

        // 过期条目不可被 get_cached_food 读取
        assert!(storage.get_cached_food("0000000000000").unwrap().is_none());

        // 5. 驱逐过期数据
        let evicted = storage.evict_expired().unwrap();
        assert_eq!(evicted, 1);

        // 6. 清空全部
        let cleared = storage.clear_all().unwrap();
        assert_eq!(cleared, 1);
        assert_eq!(storage.get_cache_stats().unwrap().total_entries, 0);
    }
}