use chrono::NaiveDate;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::error::StorageError;
use super::models::{
    ActivityLogRecord, DailySummary, FastingSessionRecord, FoodRecord, FoodSource, FoodWithNutriments, IntakeLogRecord,
    MealType, Nutriments100g, UserGoalRecord, UserProfileRecord, WaterLogRecord, WeightLogRecord,
};
use super::schema::CREATE_SCHEMA_SQL;

/// 线程安全的本地 SQLite 数据库操作引擎。
#[derive(Clone)]
pub struct StorageEngine {
    conn: Arc<Mutex<Connection>>,
}

impl StorageEngine {
    /// 安全获取数据库连接互斥锁。
    ///
    /// 相比 `lock().unwrap()`，锁中毒时返回 [`StorageError::LockPoisoned`] 而非 panic，
    /// 避免 panic 跨越移动端 UniFFI 边界导致宿主 App 进程整体崩溃。
    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StorageError> {
        self.conn.lock().map_err(|e| StorageError::LockPoisoned(e.to_string()))
    }

    /// 打开内存 SQLite 数据库（单元测试与临时调试专用）。
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let _ = conn.query_row("PRAGMA foreign_keys = ON;", [], |_| Ok(()));
        let engine = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// 在指定路径打开或创建持久化 SQLite 数据库文件。
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let _ = conn.query_row("PRAGMA journal_mode = WAL;", [], |_| Ok(()));
        // WAL 模式下配合 synchronous = NORMAL：官方推荐组合，仍可防止崩溃与断电导致的数据库损坏
        // （最坏情况仅丢失最近若干已提交事务），显著减少批量写入（导入备份、批量打卡）时的 fsync 次数，
        // 对移动端闪存寿命与写入延迟均有明显收益。
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.query_row("PRAGMA foreign_keys = ON;", [], |_| Ok(()));
        let engine = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// 初始化数据库架构、建表、索引与 FTS5 触发器。
    pub fn init_schema(&self) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute_batch(CREATE_SCHEMA_SQL)?;
        Ok(())
    }

    /// 插入或更新用户生理档案记录。
    pub fn insert_user(
        &self,
        id: &str,
        name: &str,
        birthday: &str,
        profile: &crate::models::UserProfile,
    ) -> Result<(), StorageError> {
        let conn = self.conn()?;
        let (gender_str, hormone_str) = match profile.gender {
            crate::models::Gender::Male => ("male", None),
            crate::models::Gender::Female => ("female", None),
            crate::models::Gender::NonBinary(h) => {
                let h_str = match h {
                    crate::models::HormoneProfile::Averaged => "averaged",
                    crate::models::HormoneProfile::EstrogenTypical => "estrogen_typical",
                    crate::models::HormoneProfile::TestosteroneTypical => "testosterone_typical",
                };
                ("non_binary", Some(h_str))
            }
        };

        let activity_str = match profile.activity_level {
            crate::models::ActivityLevel::Inactive => "inactive",
            crate::models::ActivityLevel::LowActive => "low_active",
            crate::models::ActivityLevel::Active => "active",
            crate::models::ActivityLevel::VeryActive => "very_active",
        };

        conn.execute(
            r#"
            INSERT INTO users (id, name, birthday, height_cm, weight_kg, gender, hormone_profile, activity_level)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                birthday = excluded.birthday,
                height_cm = excluded.height_cm,
                weight_kg = excluded.weight_kg,
                gender = excluded.gender,
                hormone_profile = excluded.hormone_profile,
                activity_level = excluded.activity_level,
                updated_at = datetime('now')
            "#,
            params![
                id,
                name,
                birthday,
                profile.height_cm,
                profile.weight_kg,
                gender_str,
                hormone_str,
                activity_str,
            ],
        )?;
        Ok(())
    }

    /// 插入或更新食品元数据及其每100g标准营养素。
    pub fn insert_food(&self, food: &FoodRecord, nutriments: &Nutriments100g) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;

        // 1. 插入或更新食品基础元数据
        tx.execute(
            r#"
            INSERT OR REPLACE INTO foods (id, name, brand, source, serving_quantity, serving_unit, image_url)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                food.id,
                food.name,
                food.brand,
                food.source.as_str(),
                food.serving_quantity,
                food.serving_unit,
                food.image_url,
            ],
        )?;

        // 2. 插入或更新每100g归一化营养素
        tx.execute(
            r#"
            INSERT OR REPLACE INTO food_nutriments (
                food_id, energy_kcal_100, carbohydrates_100, proteins_100, fat_100,
                sugars_100, saturated_fat_100, fiber_100, monounsaturated_fat_100,
                polyunsaturated_fat_100, trans_fat_100, cholesterol_mg_100,
                sodium_mg_100, potassium_mg_100, magnesium_mg_100,
                calcium_mg_100, iron_mg_100, zinc_mg_100, phosphorus_mg_100,
                vitamin_a_ug_100, vitamin_c_mg_100, vitamin_d_ug_100,
                vitamin_b6_mg_100, vitamin_b12_ug_100, niacin_mg_100
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
            )
            "#,
            params![
                food.id,
                nutriments.energy_kcal_100,
                nutriments.carbohydrates_100,
                nutriments.proteins_100,
                nutriments.fat_100,
                nutriments.sugars_100,
                nutriments.saturated_fat_100,
                nutriments.fiber_100,
                nutriments.monounsaturated_fat_100,
                nutriments.polyunsaturated_fat_100,
                nutriments.trans_fat_100,
                nutriments.cholesterol_mg_100,
                nutriments.sodium_mg_100,
                nutriments.potassium_mg_100,
                nutriments.magnesium_mg_100,
                nutriments.calcium_mg_100,
                nutriments.iron_mg_100,
                nutriments.zinc_mg_100,
                nutriments.phosphorus_mg_100,
                nutriments.vitamin_a_ug_100,
                nutriments.vitamin_c_mg_100,
                nutriments.vitamin_d_ug_100,
                nutriments.vitamin_b6_mg_100,
                nutriments.vitamin_b12_ug_100,
                nutriments.niacin_mg_100,
            ],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 基于 FTS5 引擎及多词子串匹配对食品名称和品牌执行高性能全文与模糊检索。
    ///
    /// 检索策略：
    /// 1. 优先使用 SQLite FTS5 全文索引执行分词与前缀匹配（英文字根与前缀毫秒级响应并按 rank 排序）；
    /// 2. 当无空格中文词或子串未完全覆盖 limit 时，自动降级并联多词子串匹配，完美支持“去皮无骨鸡胸肉”中匹配“鸡胸肉”。
    pub fn search_foods_fts(&self, query: &str, limit: usize) -> Result<Vec<FoodRecord>, StorageError> {
        let conn = self.conn()?;
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        // 1. 优先尝试 FTS5 全文索引
        let fts_query = trimmed
            .split_whitespace()
            .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" ");

        if let Ok(mut stmt) = conn.prepare(
            r#"
            SELECT f.id, f.name, f.brand, f.source, f.serving_quantity, f.serving_unit, f.image_url
            FROM foods_fts fts
            JOIN foods f ON f.id = fts.id
            WHERE foods_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        ) && let Ok(rows) = stmt.query_map(params![fts_query, limit as i64], |row| {
            let source_str: String = row.get(3)?;
            Ok(FoodRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                source: FoodSource::from_str(&source_str),
                serving_quantity: row.get(4)?,
                serving_unit: row.get(5)?,
                image_url: row.get(6)?,
            })
        }) {
            for item in rows.flatten() {
                seen_ids.insert(item.id.clone());
                results.push(item);
            }
        }

        // 2. 若命中未满 limit，启动多词包含匹配补充未分词的中文子串
        if results.len() < limit {
            let remaining = limit - results.len();
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            if !words.is_empty() {
                let mut where_clauses = Vec::new();
                let mut sql_params: Vec<String> = Vec::new();

                for word in &words {
                    where_clauses.push("(f.name LIKE ? OR (f.brand IS NOT NULL AND f.brand LIKE ?))");
                    let pat = format!("%{}%", word.replace('%', "\\%").replace('_', "\\_"));
                    sql_params.push(pat.clone());
                    sql_params.push(pat);
                }

                let where_str = where_clauses.join(" AND ");
                let sql = format!(
                    r#"
                    SELECT f.id, f.name, f.brand, f.source, f.serving_quantity, f.serving_unit, f.image_url
                    FROM foods f
                    WHERE {}
                    LIMIT ?
                    "#,
                    where_str
                );

                if let Ok(mut stmt) = conn.prepare(&sql) {
                    let mut query_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
                    for p in &sql_params {
                        query_params.push(p);
                    }
                    let rem_i64 = remaining as i64;
                    query_params.push(&rem_i64);

                    if let Ok(rows) = stmt.query_map(rusqlite::params_from_iter(query_params), |row| {
                        let source_str: String = row.get(3)?;
                        Ok(FoodRecord {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            brand: row.get(2)?,
                            source: FoodSource::from_str(&source_str),
                            serving_quantity: row.get(4)?,
                            serving_unit: row.get(5)?,
                            image_url: row.get(6)?,
                        })
                    }) {
                        for item in rows.flatten() {
                            if !seen_ids.contains(&item.id) {
                                seen_ids.insert(item.id.clone());
                                results.push(item);
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// 查询食品基础元数据及其每100g标准营养素明细。
    pub fn get_food_with_nutriments(&self, id: &str) -> Result<Option<(FoodRecord, Nutriments100g)>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT f.id, f.name, f.brand, f.source, f.serving_quantity, f.serving_unit, f.image_url,
                   n.energy_kcal_100, n.carbohydrates_100, n.proteins_100, n.fat_100,
                   n.sugars_100, n.saturated_fat_100, n.fiber_100,
                   n.monounsaturated_fat_100, n.polyunsaturated_fat_100, n.trans_fat_100,
                   n.cholesterol_mg_100, n.sodium_mg_100, n.potassium_mg_100, n.magnesium_mg_100,
                   n.calcium_mg_100, n.iron_mg_100, n.zinc_mg_100, n.phosphorus_mg_100,
                   n.vitamin_a_ug_100, n.vitamin_c_mg_100, n.vitamin_d_ug_100,
                   n.vitamin_b6_mg_100, n.vitamin_b12_ug_100, n.niacin_mg_100
            FROM foods f
            JOIN food_nutriments n ON f.id = n.food_id
            WHERE f.id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let source_str: String = row.get(3)?;
            let food = FoodRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                source: FoodSource::from_str(&source_str),
                serving_quantity: row.get(4)?,
                serving_unit: row.get(5)?,
                image_url: row.get(6)?,
            };
            let nutriments = Nutriments100g {
                energy_kcal_100: row.get(7)?,
                carbohydrates_100: row.get(8)?,
                proteins_100: row.get(9)?,
                fat_100: row.get(10)?,
                sugars_100: row.get(11)?,
                saturated_fat_100: row.get(12)?,
                fiber_100: row.get(13)?,
                monounsaturated_fat_100: row.get(14)?,
                polyunsaturated_fat_100: row.get(15)?,
                trans_fat_100: row.get(16)?,
                cholesterol_mg_100: row.get(17)?,
                sodium_mg_100: row.get(18)?,
                potassium_mg_100: row.get(19)?,
                magnesium_mg_100: row.get(20)?,
                calcium_mg_100: row.get(21)?,
                iron_mg_100: row.get(22)?,
                zinc_mg_100: row.get(23)?,
                phosphorus_mg_100: row.get(24)?,
                vitamin_a_ug_100: row.get(25)?,
                vitamin_c_mg_100: row.get(26)?,
                vitamin_d_ug_100: row.get(27)?,
                vitamin_b6_mg_100: row.get(28)?,
                vitamin_b12_ug_100: row.get(29)?,
                niacin_mg_100: row.get(30)?,
            };
            Ok(Some((food, nutriments)))
        } else {
            Ok(None)
        }
    }

    /// 记录一次食物摄入打卡事件，基于摄入量 (克数 / 100.0) 锁定并快照热量及宏量营养素。
    pub fn log_intake(
        &self,
        user_id: &str,
        food_id: &str,
        amount: f64,
        unit: &str,
        meal_type: MealType,
        consumed_at: &str,
    ) -> Result<IntakeLogRecord, StorageError> {
        let (food, nutriments) = self
            .get_food_with_nutriments(food_id)?
            .ok_or_else(|| StorageError::NotFound(format!("Food not found: {}", food_id)))?;

        let ratio = amount / 100.0;
        let snapshot_kcal = ((nutriments.energy_kcal_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_p = ((nutriments.proteins_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_c = ((nutriments.carbohydrates_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_f = ((nutriments.fat_100 * ratio) * 10.0).round() / 10.0;

        let intake_id = Uuid::new_v4().to_string();

        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO intake_logs (
                id, user_id, food_id, amount, unit, meal_type, consumed_at,
                snapshot_energy_kcal, snapshot_protein_g, snapshot_carbs_g, snapshot_fat_g
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                intake_id,
                user_id,
                food_id,
                amount,
                unit,
                meal_type.as_str(),
                consumed_at,
                snapshot_kcal,
                snapshot_p,
                snapshot_c,
                snapshot_f,
            ],
        )?;

        Ok(IntakeLogRecord {
            id: intake_id,
            user_id: user_id.to_string(),
            food_id: food_id.to_string(),
            food_name: food.name,
            amount,
            unit: unit.to_string(),
            meal_type,
            consumed_at: consumed_at.to_string(),
            snapshot_energy_kcal: snapshot_kcal,
            snapshot_protein_g: snapshot_p,
            snapshot_carbs_g: snapshot_c,
            snapshot_fat_g: snapshot_f,
        })
    }

    /// 删除指定用户的某笔食物摄入打卡日志。
    pub fn delete_intake(&self, user_id: &str, intake_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM intake_logs WHERE id = ?1 AND user_id = ?2",
            params![intake_id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// 更新指定食物打卡记录的摄入量（克数）或所属餐别，并根据食品原营养素自动原子重算营养快照。
    pub fn update_intake(
        &self,
        user_id: &str,
        intake_id: &str,
        new_amount: f64,
        new_meal_type: Option<MealType>,
    ) -> Result<bool, StorageError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;

        // 1. 获取原记录的 food_id
        let food_id: String = match tx.query_row(
            "SELECT food_id FROM intake_logs WHERE id = ?1 AND user_id = ?2",
            params![intake_id, user_id],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        // 2. 获取该食品的每100g标准营养素
        let (energy_100, prot_100, carbs_100, fat_100): (f64, f64, f64, f64) = tx.query_row(
            r#"
            SELECT energy_kcal_100, proteins_100, carbohydrates_100, fat_100
            FROM food_nutriments
            WHERE food_id = ?1
            "#,
            params![food_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        // 3. 计算新快照
        let ratio = new_amount / 100.0;
        let snapshot_kcal = ((energy_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_p = ((prot_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_c = ((carbs_100 * ratio) * 10.0).round() / 10.0;
        let snapshot_f = ((fat_100 * ratio) * 10.0).round() / 10.0;

        // 4. 执行更新
        let affected = if let Some(meal) = new_meal_type {
            tx.execute(
                r#"
                UPDATE intake_logs
                SET amount = ?1, meal_type = ?2,
                    snapshot_energy_kcal = ?3, snapshot_protein_g = ?4,
                    snapshot_carbs_g = ?5, snapshot_fat_g = ?6
                WHERE id = ?7 AND user_id = ?8
                "#,
                params![
                    new_amount,
                    meal.as_str(),
                    snapshot_kcal,
                    snapshot_p,
                    snapshot_c,
                    snapshot_f,
                    intake_id,
                    user_id,
                ],
            )?
        } else {
            tx.execute(
                r#"
                UPDATE intake_logs
                SET amount = ?1,
                    snapshot_energy_kcal = ?2, snapshot_protein_g = ?3,
                    snapshot_carbs_g = ?4, snapshot_fat_g = ?5
                WHERE id = ?6 AND user_id = ?7
                "#,
                params![
                    new_amount,
                    snapshot_kcal,
                    snapshot_p,
                    snapshot_c,
                    snapshot_f,
                    intake_id,
                    user_id,
                ],
            )?
        };

        tx.commit()?;
        Ok(affected > 0)
    }

    /// 获取指定用户在特定日期（匹配 ISO 日期前缀 YYYY-MM-DD）的所有摄入打卡日志并汇总每日统计。
    pub fn get_daily_summary(
        &self,
        user_id: &str,
        date_prefix: &str, // 例如 "2026-09-14"
    ) -> Result<DailySummary, StorageError> {
        let conn = self.conn()?;
        let pattern = format!("{}%", date_prefix);

        let mut stmt = conn.prepare(
            r#"
            SELECT l.id, l.user_id, l.food_id, f.name, l.amount, l.unit, l.meal_type, l.consumed_at,
                   l.snapshot_energy_kcal, l.snapshot_protein_g, l.snapshot_carbs_g, l.snapshot_fat_g
            FROM intake_logs l
            JOIN foods f ON f.id = l.food_id
            WHERE l.user_id = ?1 AND l.consumed_at LIKE ?2
            ORDER BY l.consumed_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, pattern], |row| {
            let meal_str: String = row.get(6)?;
            Ok(IntakeLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                food_id: row.get(2)?,
                food_name: row.get(3)?,
                amount: row.get(4)?,
                unit: row.get(5)?,
                meal_type: MealType::from_str(&meal_str),
                consumed_at: row.get(7)?,
                snapshot_energy_kcal: row.get(8)?,
                snapshot_protein_g: row.get(9)?,
                snapshot_carbs_g: row.get(10)?,
                snapshot_fat_g: row.get(11)?,
            })
        })?;

        let mut total_kcal = 0.0;
        let mut total_p = 0.0;
        let mut total_c = 0.0;
        let mut total_f = 0.0;
        let mut logs = Vec::new();

        for item in rows {
            let log = item?;
            total_kcal += log.snapshot_energy_kcal;
            total_p += log.snapshot_protein_g;
            total_c += log.snapshot_carbs_g;
            total_f += log.snapshot_fat_g;
            logs.push(log);
        }

        Ok(DailySummary {
            date: date_prefix.to_string(),
            total_energy_kcal: (total_kcal * 10.0).round() / 10.0,
            total_protein_g: (total_p * 10.0).round() / 10.0,
            total_carbs_g: (total_c * 10.0).round() / 10.0,
            total_fat_g: (total_f * 10.0).round() / 10.0,
            items_count: logs.len(),
            logs,
        })
    }

    /// 获取指定用户在生理日界线（Physiological Day Boundary）下的逻辑日摄入汇总。
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `logical_date`: 逻辑日期字符串（格式 `YYYY-MM-DD`）
    /// - `boundary_offset_minutes`: 生理日界线偏离午夜的分钟数（如凌晨 04:00 为 240；若为 0 则回退至标准自然日汇总）。
    pub fn get_daily_summary_with_boundary(
        &self,
        user_id: &str,
        logical_date: &str,
        boundary_offset_minutes: u32,
    ) -> Result<DailySummary, StorageError> {
        if boundary_offset_minutes == 0 {
            return self.get_daily_summary(user_id, logical_date);
        }

        let date = NaiveDate::parse_from_str(logical_date, "%Y-%m-%d")
            .map_err(|e| StorageError::Conversion(format!("无效的逻辑日期格式 (YYYY-MM-DD): {}", e)))?;

        let (start_iso, end_iso) =
            crate::calc::DayBoundaryEngine::get_logical_day_sql_range(date, boundary_offset_minutes);

        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT l.id, l.user_id, l.food_id, f.name, l.amount, l.unit, l.meal_type, l.consumed_at,
                   l.snapshot_energy_kcal, l.snapshot_protein_g, l.snapshot_carbs_g, l.snapshot_fat_g
            FROM intake_logs l
            JOIN foods f ON f.id = l.food_id
            WHERE l.user_id = ?1 AND l.consumed_at >= ?2 AND l.consumed_at < ?3
            ORDER BY l.consumed_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, start_iso, end_iso], |row| {
            let meal_str: String = row.get(6)?;
            Ok(IntakeLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                food_id: row.get(2)?,
                food_name: row.get(3)?,
                amount: row.get(4)?,
                unit: row.get(5)?,
                meal_type: MealType::from_str(&meal_str),
                consumed_at: row.get(7)?,
                snapshot_energy_kcal: row.get(8)?,
                snapshot_protein_g: row.get(9)?,
                snapshot_carbs_g: row.get(10)?,
                snapshot_fat_g: row.get(11)?,
            })
        })?;

        let mut total_kcal = 0.0;
        let mut total_p = 0.0;
        let mut total_c = 0.0;
        let mut total_f = 0.0;
        let mut logs = Vec::new();

        for item in rows {
            let log = item?;
            total_kcal += log.snapshot_energy_kcal;
            total_p += log.snapshot_protein_g;
            total_c += log.snapshot_carbs_g;
            total_f += log.snapshot_fat_g;
            logs.push(log);
        }

        Ok(DailySummary {
            date: logical_date.to_string(),
            total_energy_kcal: (total_kcal * 10.0).round() / 10.0,
            total_protein_g: (total_p * 10.0).round() / 10.0,
            total_carbs_g: (total_c * 10.0).round() / 10.0,
            total_fat_g: (total_f * 10.0).round() / 10.0,
            items_count: logs.len(),
            logs,
        })
    }

    /// 记录一次饮水打卡事件（单位：毫升 ml）。
    pub fn log_water(
        &self,
        user_id: &str,
        amount_ml: u32,
        logged_at: &str, // ISO-8601 格式 YYYY-MM-DDTHH:MM:SS
    ) -> Result<String, StorageError> {
        let conn = self.conn()?;
        let log_id = Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO water_logs (id, user_id, logged_at, amount_ml)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![log_id, user_id, logged_at, amount_ml as i64],
        )?;
        Ok(log_id)
    }

    /// 聚合指定日期的每日总饮水量，并与设定的 `goal_ml` 进行对比。
    pub fn get_daily_water(
        &self,
        user_id: &str,
        date_prefix: &str, // 例如 "2026-09-14"
        goal_ml: u32,
    ) -> Result<crate::calc::water::DailyWaterSummary, StorageError> {
        let conn = self.conn()?;
        let pattern = format!("{}%", date_prefix);

        let mut stmt = conn.prepare(
            r#"
            SELECT COALESCE(SUM(amount_ml), 0)
            FROM water_logs
            WHERE user_id = ?1 AND logged_at LIKE ?2
            "#,
        )?;

        let total_ml: i64 = stmt.query_row(params![user_id, pattern], |row| row.get(0))?;
        Ok(crate::calc::water::DailyWaterSummary::new(
            date_prefix,
            total_ml as u32,
            goal_ml,
        ))
    }

    /// 删除指定用户的某笔饮水打卡记录。
    pub fn delete_water_log(&self, user_id: &str, log_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM water_logs WHERE id = ?1 AND user_id = ?2",
            params![log_id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// 开启一段新的间歇性断食会话。
    pub fn start_fasting(&self, user_id: &str, started_at: &str, target_minutes: u32) -> Result<String, StorageError> {
        let conn = self.conn()?;
        let session_id = Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO fasting_sessions (id, user_id, started_at, target_duration_minutes)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![session_id, user_id, started_at, target_minutes as i64],
        )?;
        Ok(session_id)
    }

    /// 查询用户当前正在进行中的活跃断食会话（若存在）。
    pub fn get_active_fasting(
        &self,
        user_id: &str,
    ) -> Result<Option<(String, chrono::DateTime<chrono::Utc>, u32)>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, started_at, target_duration_minutes
            FROM fasting_sessions
            WHERE user_id = ?1 AND completed_at IS NULL AND cancelled_at IS NULL
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )?;

        let mut rows = stmt.query(params![user_id])?;
        if let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let started_str: String = row.get(1)?;
            let target: i64 = row.get(2)?;
            let started_at = started_str
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map_err(|e| StorageError::Conversion(e.to_string()))?;
            Ok(Some((id, started_at, target as u32)))
        } else {
            Ok(None)
        }
    }

    /// 将活跃断食会话标记为正常圆满结束。
    pub fn complete_fasting(&self, session_id: &str, completed_at: &str) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE fasting_sessions SET completed_at = ?1 WHERE id = ?2",
            params![completed_at, session_id],
        )?;
        Ok(())
    }

    /// 将活跃断食会话标记为已取消/放弃。
    pub fn cancel_fasting(&self, session_id: &str, cancelled_at: &str) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE fasting_sessions SET cancelled_at = ?1 WHERE id = ?2",
            params![cancelled_at, session_id],
        )?;
        Ok(())
    }

    /// 删除指定用户的某个断食会话记录。
    pub fn delete_fasting_session(&self, user_id: &str, session_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM fasting_sessions WHERE id = ?1 AND user_id = ?2",
            params![session_id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// 保存自建食谱，计算完整营养素与覆盖率，记录原料明细，并自动将该食谱注册为 source='recipe' 的可检索食物项。
    #[allow(clippy::too_many_arguments)]
    pub fn save_recipe(
        &self,
        user_id: &str,
        recipe_id: Option<&str>,
        name: &str,
        description: Option<&str>,
        servings: f64,
        total_weight_override: Option<f64>,
        ingredients: &[crate::calc::recipe::RecipeIngredientInput],
    ) -> Result<crate::storage::models::RecipeWithDetails, StorageError> {
        let comp = crate::calc::recipe::compute_recipe_nutrition(ingredients, servings, total_weight_override)
            .map_err(StorageError::Conversion)?;

        let actual_recipe_id = recipe_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("recipe_{}", Uuid::new_v4()));

        let now_str = chrono::Utc::now().to_rfc3339();

        let mut conn = self.conn()?;
        let tx = conn.transaction()?;

        // 1. 插入或更新 recipes 主表
        tx.execute(
            r#"
            INSERT INTO recipes (id, user_id, name, description, servings, total_weight_g, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                servings = excluded.servings,
                total_weight_g = excluded.total_weight_g,
                updated_at = excluded.updated_at
            "#,
            params![
                actual_recipe_id,
                user_id,
                name,
                description,
                servings,
                comp.total_weight_g,
                now_str,
            ],
        )?;

        // 2. 清理并重新插入食谱原料明细
        tx.execute(
            "DELETE FROM recipe_ingredients WHERE recipe_id = ?1",
            params![actual_recipe_id],
        )?;

        let mut ingredient_records = Vec::with_capacity(ingredients.len());
        for (i, ing) in ingredients.iter().enumerate() {
            let ing_id = Uuid::new_v4().to_string();
            let converted_g = comp.ingredient_weights_g[i];

            tx.execute(
                r#"
                INSERT INTO recipe_ingredients (id, recipe_id, food_id, amount, unit, converted_amount_g)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![ing_id, actual_recipe_id, ing.food_id, ing.amount, ing.unit, converted_g],
            )?;

            ingredient_records.push(crate::storage::models::RecipeIngredientRecord {
                id: ing_id,
                recipe_id: actual_recipe_id.clone(),
                food_id: ing.food_id.clone(),
                food_name: ing.name.clone(),
                amount: ing.amount,
                unit: ing.unit.clone(),
                converted_amount_g: converted_g,
            });
        }

        // 3. 在 foods 与 food_nutriments 表中同步注册/更新食谱虚拟食物
        let serving_g = comp.total_weight_g / servings;
        tx.execute(
            r#"
            INSERT INTO foods (id, name, brand, source, serving_quantity, serving_unit)
            VALUES (?1, ?2, ?3, 'recipe', ?4, 'g')
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                brand = excluded.brand,
                serving_quantity = excluded.serving_quantity,
                serving_unit = excluded.serving_unit
            "#,
            params![actual_recipe_id, name, "Custom Recipe", serving_g],
        )?;

        let n = &comp.per_100g;
        tx.execute(
            r#"
            INSERT OR REPLACE INTO food_nutriments (
                food_id, energy_kcal_100, carbohydrates_100, proteins_100, fat_100,
                sugars_100, saturated_fat_100, fiber_100, monounsaturated_fat_100,
                polyunsaturated_fat_100, trans_fat_100, cholesterol_mg_100,
                sodium_mg_100, potassium_mg_100, magnesium_mg_100,
                calcium_mg_100, iron_mg_100, zinc_mg_100, phosphorus_mg_100,
                vitamin_a_ug_100, vitamin_c_mg_100, vitamin_d_ug_100,
                vitamin_b6_mg_100, vitamin_b12_ug_100, niacin_mg_100
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20,
                ?21, ?22, ?23, ?24, ?25
            )
            "#,
            params![
                actual_recipe_id,
                n.energy_kcal_100,
                n.carbohydrates_100,
                n.proteins_100,
                n.fat_100,
                n.sugars_100,
                n.saturated_fat_100,
                n.fiber_100,
                n.monounsaturated_fat_100,
                n.polyunsaturated_fat_100,
                n.trans_fat_100,
                n.cholesterol_mg_100,
                n.sodium_mg_100,
                n.potassium_mg_100,
                n.magnesium_mg_100,
                n.calcium_mg_100,
                n.iron_mg_100,
                n.zinc_mg_100,
                n.phosphorus_mg_100,
                n.vitamin_a_ug_100,
                n.vitamin_c_mg_100,
                n.vitamin_d_ug_100,
                n.vitamin_b6_mg_100,
                n.vitamin_b12_ug_100,
                n.niacin_mg_100,
            ],
        )?;

        tx.commit()?;

        Ok(crate::storage::models::RecipeWithDetails {
            recipe: crate::storage::models::RecipeRecord {
                id: actual_recipe_id,
                user_id: user_id.to_string(),
                name: name.to_string(),
                description: description.map(|s| s.to_string()),
                servings,
                total_weight_g: comp.total_weight_g,
                created_at: now_str,
            },
            ingredients: ingredient_records,
            per_100g: comp.per_100g,
            per_serving: comp.per_serving,
        })
    }

    /// 查询食谱详情（包含所有原料明细及其计算所得的每100g和每份营养素）。
    pub fn get_recipe(
        &self,
        recipe_id: &str,
    ) -> Result<Option<crate::storage::models::RecipeWithDetails>, StorageError> {
        let conn = self.conn()?;

        // 1. 获取食谱基本信息
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, name, description, servings, total_weight_g, created_at
            FROM recipes
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![recipe_id])?;
        let recipe = if let Some(row) = rows.next()? {
            crate::storage::models::RecipeRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                servings: row.get(4)?,
                total_weight_g: row.get(5)?,
                created_at: row.get(6)?,
            }
        } else {
            return Ok(None);
        };

        // 2. 获取食谱原料明细列表
        let mut ing_stmt = conn.prepare(
            r#"
            SELECT ri.id, ri.recipe_id, ri.food_id, f.name, ri.amount, ri.unit, ri.converted_amount_g
            FROM recipe_ingredients ri
            JOIN foods f ON f.id = ri.food_id
            WHERE ri.recipe_id = ?1
            "#,
        )?;

        let ing_rows = ing_stmt.query_map(params![recipe_id], |row| {
            Ok(crate::storage::models::RecipeIngredientRecord {
                id: row.get(0)?,
                recipe_id: row.get(1)?,
                food_id: row.get(2)?,
                food_name: row.get(3)?,
                amount: row.get(4)?,
                unit: row.get(5)?,
                converted_amount_g: row.get(6)?,
            })
        })?;

        let mut ingredients = Vec::new();
        for item in ing_rows {
            ingredients.push(item?);
        }

        // 3. 从 food_nutriments 中获取每100g归一化营养素
        let mut nutr_stmt = conn.prepare(
            r#"
            SELECT energy_kcal_100, carbohydrates_100, proteins_100, fat_100,
                   sugars_100, saturated_fat_100, fiber_100,
                   monounsaturated_fat_100, polyunsaturated_fat_100, trans_fat_100,
                   cholesterol_mg_100, sodium_mg_100, potassium_mg_100, magnesium_mg_100,
                   calcium_mg_100, iron_mg_100, zinc_mg_100, phosphorus_mg_100,
                   vitamin_a_ug_100, vitamin_c_mg_100, vitamin_d_ug_100,
                   vitamin_b6_mg_100, vitamin_b12_ug_100, niacin_mg_100
            FROM food_nutriments
            WHERE food_id = ?1
            "#,
        )?;

        let mut nutr_rows = nutr_stmt.query(params![recipe_id])?;
        let per_100g = if let Some(row) = nutr_rows.next()? {
            crate::storage::models::Nutriments100g {
                energy_kcal_100: row.get(0)?,
                carbohydrates_100: row.get(1)?,
                proteins_100: row.get(2)?,
                fat_100: row.get(3)?,
                sugars_100: row.get(4)?,
                saturated_fat_100: row.get(5)?,
                fiber_100: row.get(6)?,
                monounsaturated_fat_100: row.get(7)?,
                polyunsaturated_fat_100: row.get(8)?,
                trans_fat_100: row.get(9)?,
                cholesterol_mg_100: row.get(10)?,
                sodium_mg_100: row.get(11)?,
                potassium_mg_100: row.get(12)?,
                magnesium_mg_100: row.get(13)?,
                calcium_mg_100: row.get(14)?,
                iron_mg_100: row.get(15)?,
                zinc_mg_100: row.get(16)?,
                phosphorus_mg_100: row.get(17)?,
                vitamin_a_ug_100: row.get(18)?,
                vitamin_c_mg_100: row.get(19)?,
                vitamin_d_ug_100: row.get(20)?,
                vitamin_b6_mg_100: row.get(21)?,
                vitamin_b12_ug_100: row.get(22)?,
                niacin_mg_100: row.get(23)?,
            }
        } else {
            crate::storage::models::Nutriments100g::simple(0.0, 0.0, 0.0, 0.0)
        };

        let serving_ratio = recipe.total_weight_g / recipe.servings / 100.0;

        let per_serving = crate::storage::models::Nutriments100g {
            energy_kcal_100: ((per_100g.energy_kcal_100 * serving_ratio) * 100.0).round() / 100.0,
            carbohydrates_100: ((per_100g.carbohydrates_100 * serving_ratio) * 100.0).round() / 100.0,
            proteins_100: ((per_100g.proteins_100 * serving_ratio) * 100.0).round() / 100.0,
            fat_100: ((per_100g.fat_100 * serving_ratio) * 100.0).round() / 100.0,
            sugars_100: per_100g
                .sugars_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            saturated_fat_100: per_100g
                .saturated_fat_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            fiber_100: per_100g
                .fiber_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            monounsaturated_fat_100: per_100g
                .monounsaturated_fat_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            polyunsaturated_fat_100: per_100g
                .polyunsaturated_fat_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            trans_fat_100: per_100g
                .trans_fat_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            cholesterol_mg_100: per_100g
                .cholesterol_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            sodium_mg_100: per_100g
                .sodium_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            potassium_mg_100: per_100g
                .potassium_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            magnesium_mg_100: per_100g
                .magnesium_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            calcium_mg_100: per_100g
                .calcium_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            iron_mg_100: per_100g
                .iron_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            zinc_mg_100: per_100g
                .zinc_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            phosphorus_mg_100: per_100g
                .phosphorus_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            vitamin_a_ug_100: per_100g
                .vitamin_a_ug_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            vitamin_c_mg_100: per_100g
                .vitamin_c_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            vitamin_d_ug_100: per_100g
                .vitamin_d_ug_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            vitamin_b6_mg_100: per_100g
                .vitamin_b6_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            vitamin_b12_ug_100: per_100g
                .vitamin_b12_ug_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
            niacin_mg_100: per_100g
                .niacin_mg_100
                .map(|v| ((v * serving_ratio) * 100.0).round() / 100.0),
        };

        Ok(Some(crate::storage::models::RecipeWithDetails {
            recipe,
            ingredients,
            per_100g,
            per_serving,
        }))
    }

    /// 列出指定用户创建的所有自建食谱基础信息。
    pub fn list_user_recipes(&self, user_id: &str) -> Result<Vec<crate::storage::models::RecipeRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, name, description, servings, total_weight_g, created_at
            FROM recipes
            WHERE user_id = ?1
            ORDER BY name ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id], |row| {
            Ok(crate::storage::models::RecipeRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                servings: row.get(4)?,
                total_weight_g: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// 列出用户的所有食谱及其完整原料与计算营养素详情。
    pub fn list_all_recipes_with_details(
        &self,
        user_id: &str,
    ) -> Result<Vec<crate::storage::models::RecipeWithDetails>, StorageError> {
        let headers = self.list_user_recipes(user_id)?;
        let mut list = Vec::new();
        for r in headers {
            if let Some(details) = self.get_recipe(&r.id)? {
                list.push(details);
            }
        }
        Ok(list)
    }

    /// 删除食谱及其原料明细与关联注册的虚拟食物项。
    pub fn delete_recipe(&self, recipe_id: &str) -> Result<(), StorageError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM recipes WHERE id = ?1", params![recipe_id])?;
        tx.execute("DELETE FROM foods WHERE id = ?1", params![recipe_id])?;
        tx.commit()?;
        Ok(())
    }

    /// 记录一次体重测量打卡。
    pub fn log_weight(
        &self,
        user_id: &str,
        weight_kg: f64,
        body_fat_pct: Option<f64>,
        logged_at: &str, // YYYY-MM-DDTHH:MM:SS
        notes: Option<&str>,
    ) -> Result<String, StorageError> {
        let conn = self.conn()?;
        let log_id = Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO weight_logs (id, user_id, logged_at, weight_kg, body_fat_pct, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![log_id, user_id, logged_at, weight_kg, body_fat_pct, notes],
        )?;
        Ok(log_id)
    }

    /// 删除指定用户的某笔体重打卡记录。
    pub fn delete_weight_log(&self, user_id: &str, log_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM weight_logs WHERE id = ?1 AND user_id = ?2",
            params![log_id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// 获取按时间排序的每日平均体重历史数据，用于体重趋势分析与线性回归投影。
    pub fn get_weight_history(
        &self,
        user_id: &str,
        limit: usize,
    ) -> Result<Vec<(chrono::NaiveDate, f64)>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT SUBSTR(logged_at, 1, 10) as day_str, AVG(weight_kg)
            FROM weight_logs
            WHERE user_id = ?1
            GROUP BY day_str
            ORDER BY day_str ASC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, limit as i64], |row| {
            let day_str: String = row.get(0)?;
            let weight: f64 = row.get(1)?;
            let date = chrono::NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            Ok((date, weight))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// 获取每日摄入总量历史（热量、蛋白质、碳水、脂肪），用于移动平均线计算。
    pub fn get_daily_intakes_history(
        &self,
        user_id: &str,
        limit_days: usize,
    ) -> Result<Vec<(f64, f64, f64, f64)>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT SUBSTR(consumed_at, 1, 10) as day_str,
                   SUM(snapshot_energy_kcal),
                   SUM(snapshot_protein_g),
                   SUM(snapshot_carbs_g),
                   SUM(snapshot_fat_g)
            FROM intake_logs
            WHERE user_id = ?1
            GROUP BY day_str
            ORDER BY day_str ASC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, limit_days as i64], |row| {
            let kcal: f64 = row.get(1)?;
            let p: f64 = row.get(2)?;
            let c: f64 = row.get(3)?;
            let f: f64 = row.get(4)?;
            Ok((kcal, p, c, f))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// 确保指定用户存在于 users 表中；若不存在则自动插入默认用户档案。
    pub fn ensure_user_exists(&self, user_id: &str) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT OR IGNORE INTO users (id, name, birthday, height_cm, weight_kg, gender, activity_level)
            VALUES (?1, ?1, '1990-01-01', 170.0, 70.0, 'male', 'active')
            "#,
            params![user_id],
        )?;
        Ok(())
    }

    /// 根据用户 ID 获取用户档案记录。
    pub fn get_user(&self, user_id: &str) -> Result<Option<UserProfileRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, birthday, height_cm, weight_kg, gender, hormone_profile, activity_level
            FROM users
            WHERE id = ?1
            "#,
        )?;
        let mut rows = stmt.query(params![user_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(UserProfileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                birthday: row.get(2)?,
                height_cm: row.get(3)?,
                weight_kg: row.get(4)?,
                gender: row.get(5)?,
                hormone_profile: row.get(6)?,
                activity_level: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 列出所有已注册用户的生理档案记录。
    pub fn list_all_users(&self) -> Result<Vec<UserProfileRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, birthday, height_cm, weight_kg, gender, hormone_profile, activity_level
            FROM users
            ORDER BY created_at ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(UserProfileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                birthday: row.get(2)?,
                height_cm: row.get(3)?,
                weight_kg: row.get(4)?,
                gender: row.get(5)?,
                hormone_profile: row.get(6)?,
                activity_level: row.get(7)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 级联彻底删除指定用户及其所有打卡数据（摄入、体重、饮水、断食、食谱、运动、目标）。
    pub fn delete_all_user_data(&self, user_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute("DELETE FROM users WHERE id = ?1", params![user_id])?;
        Ok(affected > 0)
    }

    /// 列出数据库中所有食物及其完整每100g营养素明细。
    pub fn list_all_foods(&self) -> Result<Vec<FoodWithNutriments>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT f.id, f.name, f.brand, f.source, f.serving_quantity, f.serving_unit, f.image_url,
                   n.energy_kcal_100, n.carbohydrates_100, n.proteins_100, n.fat_100,
                   n.sugars_100, n.saturated_fat_100, n.fiber_100,
                   n.monounsaturated_fat_100, n.polyunsaturated_fat_100, n.trans_fat_100,
                   n.cholesterol_mg_100, n.sodium_mg_100, n.potassium_mg_100, n.magnesium_mg_100,
                   n.calcium_mg_100, n.iron_mg_100, n.zinc_mg_100, n.phosphorus_mg_100,
                   n.vitamin_a_ug_100, n.vitamin_c_mg_100, n.vitamin_d_ug_100,
                   n.vitamin_b6_mg_100, n.vitamin_b12_ug_100, n.niacin_mg_100
            FROM foods f
            JOIN food_nutriments n ON f.id = n.food_id
            ORDER BY f.name ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get(3)?;
            let food = FoodRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                source: FoodSource::from_str(&source_str),
                serving_quantity: row.get(4)?,
                serving_unit: row.get(5)?,
                image_url: row.get(6)?,
            };
            let nutriments = Nutriments100g {
                energy_kcal_100: row.get(7)?,
                carbohydrates_100: row.get(8)?,
                proteins_100: row.get(9)?,
                fat_100: row.get(10)?,
                sugars_100: row.get(11)?,
                saturated_fat_100: row.get(12)?,
                fiber_100: row.get(13)?,
                monounsaturated_fat_100: row.get(14)?,
                polyunsaturated_fat_100: row.get(15)?,
                trans_fat_100: row.get(16)?,
                cholesterol_mg_100: row.get(17)?,
                sodium_mg_100: row.get(18)?,
                potassium_mg_100: row.get(19)?,
                magnesium_mg_100: row.get(20)?,
                calcium_mg_100: row.get(21)?,
                iron_mg_100: row.get(22)?,
                zinc_mg_100: row.get(23)?,
                phosphorus_mg_100: row.get(24)?,
                vitamin_a_ug_100: row.get(25)?,
                vitamin_c_mg_100: row.get(26)?,
                vitamin_d_ug_100: row.get(27)?,
                vitamin_b6_mg_100: row.get(28)?,
                vitamin_b12_ug_100: row.get(29)?,
                niacin_mg_100: row.get(30)?,
            };
            Ok(FoodWithNutriments { food, nutriments })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 查询所有用户本地自建食物列表。
    pub fn list_custom_foods(&self) -> Result<Vec<FoodWithNutriments>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT f.id, f.name, f.brand, f.source, f.serving_quantity, f.serving_unit, f.image_url,
                   n.energy_kcal_100, n.carbohydrates_100, n.proteins_100, n.fat_100,
                   n.sugars_100, n.saturated_fat_100, n.fiber_100,
                   n.monounsaturated_fat_100, n.polyunsaturated_fat_100, n.trans_fat_100,
                   n.cholesterol_mg_100, n.sodium_mg_100, n.potassium_mg_100, n.magnesium_mg_100,
                   n.calcium_mg_100, n.iron_mg_100, n.zinc_mg_100, n.phosphorus_mg_100,
                   n.vitamin_a_ug_100, n.vitamin_c_mg_100, n.vitamin_d_ug_100,
                   n.vitamin_b6_mg_100, n.vitamin_b12_ug_100, n.niacin_mg_100
            FROM foods f
            JOIN food_nutriments n ON f.id = n.food_id
            WHERE f.source = 'custom'
            ORDER BY f.created_at DESC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get(3)?;
            let food = FoodRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                brand: row.get(2)?,
                source: FoodSource::from_str(&source_str),
                serving_quantity: row.get(4)?,
                serving_unit: row.get(5)?,
                image_url: row.get(6)?,
            };
            let nutriments = Nutriments100g {
                energy_kcal_100: row.get(7)?,
                carbohydrates_100: row.get(8)?,
                proteins_100: row.get(9)?,
                fat_100: row.get(10)?,
                sugars_100: row.get(11)?,
                saturated_fat_100: row.get(12)?,
                fiber_100: row.get(13)?,
                monounsaturated_fat_100: row.get(14)?,
                polyunsaturated_fat_100: row.get(15)?,
                trans_fat_100: row.get(16)?,
                cholesterol_mg_100: row.get(17)?,
                sodium_mg_100: row.get(18)?,
                potassium_mg_100: row.get(19)?,
                magnesium_mg_100: row.get(20)?,
                calcium_mg_100: row.get(21)?,
                iron_mg_100: row.get(22)?,
                zinc_mg_100: row.get(23)?,
                phosphorus_mg_100: row.get(24)?,
                vitamin_a_ug_100: row.get(25)?,
                vitamin_c_mg_100: row.get(26)?,
                vitamin_d_ug_100: row.get(27)?,
                vitamin_b6_mg_100: row.get(28)?,
                vitamin_b12_ug_100: row.get(29)?,
                niacin_mg_100: row.get(30)?,
            };
            Ok(FoodWithNutriments { food, nutriments })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 删除指定的用户自建食物（同时级联清除其营养素与 FTS5 索引）。
    pub fn delete_custom_food(&self, food_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let affected = conn.execute(
            "DELETE FROM foods WHERE id = ?1 AND source = 'custom'",
            params![food_id],
        )?;
        Ok(affected > 0)
    }

    /// 按摄入时间升序列出指定用户的所有饮食摄入打卡日志。
    pub fn list_all_intakes(&self, user_id: &str) -> Result<Vec<IntakeLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT l.id, l.user_id, l.food_id, f.name, l.amount, l.unit, l.meal_type, l.consumed_at,
                   l.snapshot_energy_kcal, l.snapshot_protein_g, l.snapshot_carbs_g, l.snapshot_fat_g
            FROM intake_logs l
            JOIN foods f ON l.food_id = f.id
            WHERE l.user_id = ?1
            ORDER BY l.consumed_at ASC
            "#,
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            let meal_str: String = row.get(6)?;
            Ok(IntakeLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                food_id: row.get(2)?,
                food_name: row.get(3)?,
                amount: row.get(4)?,
                unit: row.get(5)?,
                meal_type: MealType::from_str(&meal_str),
                consumed_at: row.get(7)?,
                snapshot_energy_kcal: row.get(8)?,
                snapshot_protein_g: row.get(9)?,
                snapshot_carbs_g: row.get(10)?,
                snapshot_fat_g: row.get(11)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 按打卡时间升序列出指定用户的所有体重打卡记录。
    pub fn list_all_weights(&self, user_id: &str) -> Result<Vec<WeightLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, logged_at, weight_kg, body_fat_pct, notes
            FROM weight_logs
            WHERE user_id = ?1
            ORDER BY logged_at ASC
            "#,
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(WeightLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                logged_at: row.get(2)?,
                weight_kg: row.get(3)?,
                body_fat_pct: row.get(4)?,
                notes: row.get(5)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 按记录时间升序列出指定用户的所有饮水打卡记录。
    pub fn list_all_waters(&self, user_id: &str) -> Result<Vec<WaterLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, logged_at, amount_ml
            FROM water_logs
            WHERE user_id = ?1
            ORDER BY logged_at ASC
            "#,
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(WaterLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                logged_at: row.get(2)?,
                amount_ml: row.get(3)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 按开始时间升序列出指定用户的所有断食会话记录。
    pub fn list_all_fasting(&self, user_id: &str) -> Result<Vec<FastingSessionRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, started_at, target_duration_minutes, completed_at, cancelled_at
            FROM fasting_sessions
            WHERE user_id = ?1
            ORDER BY started_at ASC
            "#,
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(FastingSessionRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                started_at: row.get(2)?,
                target_duration_minutes: row.get(3)?,
                completed_at: row.get(4)?,
                cancelled_at: row.get(5)?,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 获取指定用户在特定日期的所有摄入食物项（包含摄入克数与完整每100g微量营养素），用于微量元素评估。
    pub fn get_daily_intake_nutriments(
        &self,
        user_id: &str,
        date: &str,
    ) -> Result<Vec<(f64, Nutriments100g)>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT l.amount,
                   n.energy_kcal_100, n.carbohydrates_100, n.proteins_100, n.fat_100,
                   n.sugars_100, n.saturated_fat_100, n.fiber_100,
                   n.monounsaturated_fat_100, n.polyunsaturated_fat_100, n.trans_fat_100,
                   n.cholesterol_mg_100, n.sodium_mg_100, n.potassium_mg_100, n.magnesium_mg_100,
                   n.calcium_mg_100, n.iron_mg_100, n.zinc_mg_100, n.phosphorus_mg_100,
                   n.vitamin_a_ug_100, n.vitamin_c_mg_100, n.vitamin_d_ug_100,
                   n.vitamin_b6_mg_100, n.vitamin_b12_ug_100, n.niacin_mg_100
            FROM intake_logs l
            JOIN food_nutriments n ON l.food_id = n.food_id
            WHERE l.user_id = ?1 AND SUBSTR(l.consumed_at, 1, 10) = ?2
            ORDER BY l.consumed_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, date], |row| {
            let amount: f64 = row.get(0)?;
            let nutriments = Nutriments100g {
                energy_kcal_100: row.get(1)?,
                carbohydrates_100: row.get(2)?,
                proteins_100: row.get(3)?,
                fat_100: row.get(4)?,
                sugars_100: row.get(5)?,
                saturated_fat_100: row.get(6)?,
                fiber_100: row.get(7)?,
                monounsaturated_fat_100: row.get(8)?,
                polyunsaturated_fat_100: row.get(9)?,
                trans_fat_100: row.get(10)?,
                cholesterol_mg_100: row.get(11)?,
                sodium_mg_100: row.get(12)?,
                potassium_mg_100: row.get(13)?,
                magnesium_mg_100: row.get(14)?,
                calcium_mg_100: row.get(15)?,
                iron_mg_100: row.get(16)?,
                zinc_mg_100: row.get(17)?,
                phosphorus_mg_100: row.get(18)?,
                vitamin_a_ug_100: row.get(19)?,
                vitamin_c_mg_100: row.get(20)?,
                vitamin_d_ug_100: row.get(21)?,
                vitamin_b6_mg_100: row.get(22)?,
                vitamin_b12_ug_100: row.get(23)?,
                niacin_mg_100: row.get(24)?,
            };
            Ok((amount, nutriments))
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 插入或更新用户生理档案原始记录。
    pub fn insert_user_raw(&self, user: &UserProfileRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO users (id, name, birthday, height_cm, weight_kg, gender, hormone_profile, activity_level)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                birthday = excluded.birthday,
                height_cm = excluded.height_cm,
                weight_kg = excluded.weight_kg,
                gender = excluded.gender,
                hormone_profile = excluded.hormone_profile,
                activity_level = excluded.activity_level,
                updated_at = datetime('now')
            "#,
            params![
                user.id,
                user.name,
                user.birthday,
                user.height_cm,
                user.weight_kg,
                user.gender,
                user.hormone_profile,
                user.activity_level,
            ],
        )?;
        Ok(())
    }

    /// 直接插入或更新食品记录及其标准营养素。
    pub fn insert_food_raw(&self, food: &FoodRecord, nutriments: &Nutriments100g) -> Result<(), StorageError> {
        self.insert_food(food, nutriments)
    }

    /// 插入或更新饮食摄入打卡原始记录。
    pub fn insert_intake_log_raw(&self, record: &IntakeLogRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO intake_logs (
                id, user_id, food_id, amount, unit, meal_type, consumed_at,
                snapshot_energy_kcal, snapshot_protein_g, snapshot_carbs_g, snapshot_fat_g
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                amount = excluded.amount,
                unit = excluded.unit,
                meal_type = excluded.meal_type,
                consumed_at = excluded.consumed_at,
                snapshot_energy_kcal = excluded.snapshot_energy_kcal,
                snapshot_protein_g = excluded.snapshot_protein_g,
                snapshot_carbs_g = excluded.snapshot_carbs_g,
                snapshot_fat_g = excluded.snapshot_fat_g
            "#,
            params![
                record.id,
                record.user_id,
                record.food_id,
                record.amount,
                record.unit,
                record.meal_type.as_str(),
                record.consumed_at,
                record.snapshot_energy_kcal,
                record.snapshot_protein_g,
                record.snapshot_carbs_g,
                record.snapshot_fat_g,
            ],
        )?;
        Ok(())
    }

    /// 插入或更新体重打卡原始记录。
    pub fn insert_weight_log_raw(&self, record: &WeightLogRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO weight_logs (id, user_id, logged_at, weight_kg, body_fat_pct, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                logged_at = excluded.logged_at,
                weight_kg = excluded.weight_kg,
                body_fat_pct = excluded.body_fat_pct,
                notes = excluded.notes
            "#,
            params![
                record.id,
                record.user_id,
                record.logged_at,
                record.weight_kg,
                record.body_fat_pct,
                record.notes,
            ],
        )?;
        Ok(())
    }

    /// 插入或更新饮水打卡原始记录。
    pub fn insert_water_log_raw(&self, record: &WaterLogRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO water_logs (id, user_id, logged_at, amount_ml)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                amount_ml = excluded.amount_ml
            "#,
            params![record.id, record.user_id, record.logged_at, record.amount_ml,],
        )?;
        Ok(())
    }

    /// 插入或更新断食会话原始记录。
    pub fn insert_fasting_session_raw(&self, record: &FastingSessionRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO fasting_sessions (id, user_id, started_at, target_duration_minutes, completed_at, cancelled_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                target_duration_minutes = excluded.target_duration_minutes,
                completed_at = excluded.completed_at,
                cancelled_at = excluded.cancelled_at
            "#,
            params![
                record.id,
                record.user_id,
                record.started_at,
                record.target_duration_minutes,
                record.completed_at,
                record.cancelled_at,
            ],
        )?;
        Ok(())
    }

    /// 记录一次运动打卡（包含名义消耗与 Pontzer 代偿后净计入消耗）。
    pub fn log_activity(&self, record: &ActivityLogRecord) -> Result<(), StorageError> {
        self.ensure_user_exists(&record.user_id)?;
        self.insert_activity_raw(record)
    }

    /// 插入或更新运动打卡原始记录。
    pub fn insert_activity_raw(&self, record: &ActivityLogRecord) -> Result<(), StorageError> {
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO activity_logs (
                id, user_id, activity_code, activity_name, category,
                duration_minutes, met_value, gross_burned_kcal, net_credited_kcal,
                compensation_ratio, date, created_at, external_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                activity_code = excluded.activity_code,
                activity_name = excluded.activity_name,
                category = excluded.category,
                duration_minutes = excluded.duration_minutes,
                met_value = excluded.met_value,
                gross_burned_kcal = excluded.gross_burned_kcal,
                net_credited_kcal = excluded.net_credited_kcal,
                compensation_ratio = excluded.compensation_ratio,
                date = excluded.date,
                external_id = excluded.external_id
            "#,
            params![
                record.id,
                record.user_id,
                record.activity_code,
                record.activity_name,
                record.category,
                record.duration_minutes,
                record.met_value,
                record.gross_burned_kcal,
                record.net_credited_kcal,
                record.compensation_ratio,
                record.date,
                record.created_at,
                record.external_id,
            ],
        )?;
        Ok(())
    }

    /// 获取用户在指定日期的所有运动打卡记录。
    pub fn get_activities_for_date(&self, user_id: &str, date: &str) -> Result<Vec<ActivityLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, activity_code, activity_name, category,
                   duration_minutes, met_value, gross_burned_kcal, net_credited_kcal,
                   compensation_ratio, date, created_at, external_id
            FROM activity_logs
            WHERE user_id = ?1 AND date = ?2
            ORDER BY created_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id, date], |row| {
            Ok(ActivityLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                activity_code: row.get(2)?,
                activity_name: row.get(3)?,
                category: row.get(4)?,
                duration_minutes: row.get(5)?,
                met_value: row.get(6)?,
                gross_burned_kcal: row.get(7)?,
                net_credited_kcal: row.get(8)?,
                compensation_ratio: row.get(9)?,
                date: row.get(10)?,
                created_at: row.get(11)?,
                external_id: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 列出指定用户的所有运动打卡记录（按时间升序）。
    pub fn list_all_activities(&self, user_id: &str) -> Result<Vec<ActivityLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, activity_code, activity_name, category,
                   duration_minutes, met_value, gross_burned_kcal, net_credited_kcal,
                   compensation_ratio, date, created_at, external_id
            FROM activity_logs
            WHERE user_id = ?1
            ORDER BY date ASC, created_at ASC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id], |row| {
            Ok(ActivityLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                activity_code: row.get(2)?,
                activity_name: row.get(3)?,
                category: row.get(4)?,
                duration_minutes: row.get(5)?,
                met_value: row.get(6)?,
                gross_burned_kcal: row.get(7)?,
                net_credited_kcal: row.get(8)?,
                compensation_ratio: row.get(9)?,
                date: row.get(10)?,
                created_at: row.get(11)?,
                external_id: row.get(12)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// 根据外部健康平台同步 ID (Apple HealthKit / Health Connect) 查询打卡记录，用于去重。
    pub fn get_activity_by_external_id(
        &self,
        user_id: &str,
        external_id: &str,
    ) -> Result<Option<ActivityLogRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, activity_code, activity_name, category,
                   duration_minutes, met_value, gross_burned_kcal, net_credited_kcal,
                   compensation_ratio, date, created_at, external_id
            FROM activity_logs
            WHERE user_id = ?1 AND external_id = ?2
            "#,
        )?;

        let mut rows = stmt.query(params![user_id, external_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ActivityLogRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                activity_code: row.get(2)?,
                activity_name: row.get(3)?,
                category: row.get(4)?,
                duration_minutes: row.get(5)?,
                met_value: row.get(6)?,
                gross_burned_kcal: row.get(7)?,
                net_credited_kcal: row.get(8)?,
                compensation_ratio: row.get(9)?,
                date: row.get(10)?,
                created_at: row.get(11)?,
                external_id: row.get(12)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 删除指定运动打卡记录。
    pub fn delete_activity(&self, user_id: &str, activity_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let count = conn.execute(
            "DELETE FROM activity_logs WHERE id = ?1 AND user_id = ?2",
            params![activity_id, user_id],
        )?;
        Ok(count > 0)
    }

    /// 聚合计算指定日期用户运动的名义总消耗与代偿后净计入消耗 (gross_burned, net_credited)。
    pub fn get_daily_activity_totals(&self, user_id: &str, date: &str) -> Result<(f64, f64), StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT COALESCE(SUM(gross_burned_kcal), 0.0), COALESCE(SUM(net_credited_kcal), 0.0)
            FROM activity_logs
            WHERE user_id = ?1 AND date = ?2
            "#,
        )?;

        let (gross, net): (f64, f64) = stmt.query_row(params![user_id, date], |row| Ok((row.get(0)?, row.get(1)?)))?;

        Ok((gross, net))
    }

    /// 设置或更新用户的体态目标配置（每个用户仅有一条生效目标记录）。
    pub fn set_user_goal(&self, goal: &UserGoalRecord) -> Result<(), StorageError> {
        self.ensure_user_exists(&goal.user_id)?;
        let conn = self.conn()?;
        conn.execute(
            r#"
            INSERT INTO user_goals (
                id, user_id, kind, target_weight_kg, weekly_rate_kg,
                taper_enabled, adaptive_enabled, manual_calorie_offset,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(user_id) DO UPDATE SET
                kind = excluded.kind,
                target_weight_kg = excluded.target_weight_kg,
                weekly_rate_kg = excluded.weekly_rate_kg,
                taper_enabled = excluded.taper_enabled,
                adaptive_enabled = excluded.adaptive_enabled,
                manual_calorie_offset = excluded.manual_calorie_offset,
                updated_at = datetime('now')
            "#,
            params![
                goal.id,
                goal.user_id,
                goal.kind,
                goal.target_weight_kg,
                goal.weekly_rate_kg,
                if goal.taper_enabled { 1 } else { 0 },
                if goal.adaptive_enabled { 1 } else { 0 },
                goal.manual_calorie_offset,
                goal.created_at,
                goal.updated_at,
            ],
        )?;
        Ok(())
    }

    /// 获取用户的当前活跃目标配置（若存在）。
    pub fn get_user_goal(&self, user_id: &str) -> Result<Option<UserGoalRecord>, StorageError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, user_id, kind, target_weight_kg, weekly_rate_kg,
                   taper_enabled, adaptive_enabled, manual_calorie_offset,
                   created_at, updated_at
            FROM user_goals
            WHERE user_id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![user_id])?;
        if let Some(row) = rows.next()? {
            let taper_int: i64 = row.get(5)?;
            let adaptive_int: i64 = row.get(6)?;
            Ok(Some(UserGoalRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                kind: row.get(2)?,
                target_weight_kg: row.get(3)?,
                weekly_rate_kg: row.get(4)?,
                taper_enabled: taper_int != 0,
                adaptive_enabled: adaptive_int != 0,
                manual_calorie_offset: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// 删除指定用户的目标配置。
    pub fn delete_user_goal(&self, user_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn()?;
        let count = conn.execute("DELETE FROM user_goals WHERE user_id = ?1", params![user_id])?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_food_insert_and_fts_search() {
        let storage = StorageEngine::open_in_memory().unwrap();

        // 1. 插入鸡胸肉
        let chicken = FoodRecord {
            id: "food_chicken_001".to_string(),
            name: "去皮无骨鸡胸肉".to_string(),
            brand: Some("Kirkland Signature".to_string()),
            source: FoodSource::Custom,
            serving_quantity: Some(100.0),
            serving_unit: Some("g".to_string()),
            image_url: None,
        };
        let chicken_nutriments = Nutriments100g::simple(165.0, 0.0, 31.0, 3.6);
        storage.insert_food(&chicken, &chicken_nutriments).unwrap();

        // 2. 插入燕麦片
        let oatmeal = FoodRecord {
            id: "food_oatmeal_002".to_string(),
            name: "全谷物传统燕麦片".to_string(),
            brand: Some("Quaker 桂格".to_string()),
            source: FoodSource::OpenFoodFacts,
            serving_quantity: Some(40.0),
            serving_unit: Some("g".to_string()),
            image_url: None,
        };
        let oatmeal_nutriments = Nutriments100g::simple(375.0, 68.0, 13.0, 6.5);
        storage.insert_food(&oatmeal, &oatmeal_nutriments).unwrap();

        // 3. FTS5 搜索 "鸡胸肉"
        let search_results = storage.search_foods_fts("鸡胸肉", 10).unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, "food_chicken_001");

        // 4. FTS5 前缀搜索 "桂格 燕麦"
        let search_oats = storage.search_foods_fts("桂格 燕麦", 10).unwrap();
        assert_eq!(search_oats.len(), 1);
        assert_eq!(search_oats[0].id, "food_oatmeal_002");
    }

    #[test]
    fn test_storage_log_intake_and_daily_summary() {
        let storage = StorageEngine::open_in_memory().unwrap();

        // 1. 插入用户
        let user_profile = crate::models::UserProfile::new(
            28,
            175.0,
            75.0,
            crate::models::Gender::Male,
            crate::models::ActivityLevel::Active,
        )
        .unwrap();
        storage
            .insert_user("user_01", "Alex", "1998-05-12", &user_profile)
            .unwrap();

        // 2. 插入测试食品（三文鱼排：每 100g 含 208 kcal, 0g 碳水, 20g 蛋白质, 13g 脂肪）
        let salmon = FoodRecord {
            id: "salmon_wild".to_string(),
            name: "野生大西洋三文鱼".to_string(),
            brand: None,
            source: FoodSource::UsdaFoodDataCentral,
            serving_quantity: Some(150.0),
            serving_unit: Some("g".to_string()),
            image_url: None,
        };
        let salmon_nutr = Nutriments100g::simple(208.0, 0.0, 20.0, 13.0);
        storage.insert_food(&salmon, &salmon_nutr).unwrap();

        // 记录 2026-09-14 午餐摄入 200g 三文鱼
        let log = storage
            .log_intake(
                "user_01",
                "salmon_wild",
                200.0,
                "g",
                MealType::Lunch,
                "2026-09-14T12:30:00",
            )
            .unwrap();

        // 200g = 每 100g 营养素的 2 倍
        assert_eq!(log.snapshot_energy_kcal, 416.0);
        assert_eq!(log.snapshot_protein_g, 40.0);
        assert_eq!(log.snapshot_fat_g, 26.0);

        // 获取当日汇总
        let summary = storage.get_daily_summary("user_01", "2026-09-14").unwrap();
        assert_eq!(summary.items_count, 1);
        assert_eq!(summary.total_energy_kcal, 416.0);
        assert_eq!(summary.total_protein_g, 40.0);
        assert_eq!(summary.total_fat_g, 26.0);
    }

    #[test]
    fn test_storage_water_tracking() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user = crate::models::UserProfile::new(
            30,
            180.0,
            80.0,
            crate::models::Gender::Male,
            crate::models::ActivityLevel::Active,
        )
        .unwrap();
        storage.insert_user("user_water", "Bob", "1994-01-01", &user).unwrap();

        // 分别打卡 500ml 和 750ml
        storage.log_water("user_water", 500, "2026-09-14T09:00:00Z").unwrap();
        storage.log_water("user_water", 750, "2026-09-14T14:30:00Z").unwrap();

        let daily_water = storage.get_daily_water("user_water", "2026-09-14", 2500).unwrap();
        assert_eq!(daily_water.total_consumed_ml, 1250);
        assert_eq!(daily_water.goal_ml, 2500);
        assert_eq!(daily_water.remaining_ml, 1250);
        assert_eq!(daily_water.progress_pct, 50.0);
    }

    #[test]
    fn test_storage_fasting_lifecycle() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user = crate::models::UserProfile::new(
            25,
            165.0,
            55.0,
            crate::models::Gender::Female,
            crate::models::ActivityLevel::Inactive,
        )
        .unwrap();
        storage
            .insert_user("user_fasting", "Clara", "1999-03-20", &user)
            .unwrap();

        // 1. 初始状态下无活跃会话
        let active = storage.get_active_fasting("user_fasting").unwrap();
        assert!(active.is_none());

        // 2. 开启 16:8 断食（960 分钟）
        let session_id = storage
            .start_fasting("user_fasting", "2026-09-14T20:00:00Z", 960)
            .unwrap();

        // 3. 查询当前活跃断食
        let active = storage.get_active_fasting("user_fasting").unwrap();
        assert!(active.is_some());
        let (id, started, target) = active.unwrap();
        assert_eq!(id, session_id);
        assert_eq!(target, 960);
        assert_eq!(started.to_rfc3339(), "2026-09-14T20:00:00+00:00");

        // 4. 完成断食会话
        storage.complete_fasting(&session_id, "2026-09-15T12:00:00Z").unwrap();

        // 5. 再次查询活跃会话 - 应为空
        let active_after = storage.get_active_fasting("user_fasting").unwrap();
        assert!(active_after.is_none());
    }

    #[test]
    fn test_storage_recipe_management() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user = crate::models::UserProfile::new(
            27,
            172.0,
            68.0,
            crate::models::Gender::Female,
            crate::models::ActivityLevel::Active,
        )
        .unwrap();
        storage.insert_user("user_chef", "Diana", "1997-08-15", &user).unwrap();

        // 1. 将原料存入 foods 表
        let oats = FoodRecord {
            id: "food_oats_test".to_string(),
            name: "测试燕麦片".to_string(),
            brand: None,
            source: FoodSource::Custom,
            serving_quantity: Some(40.0),
            serving_unit: Some("g".to_string()),
            image_url: None,
        };
        storage
            .insert_food(&oats, &Nutriments100g::simple(380.0, 66.0, 13.0, 7.0))
            .unwrap();

        let milk = FoodRecord {
            id: "food_milk_test".to_string(),
            name: "全脂鲜牛奶".to_string(),
            brand: None,
            source: FoodSource::Custom,
            serving_quantity: Some(250.0),
            serving_unit: Some("ml".to_string()),
            image_url: None,
        };
        storage
            .insert_food(&milk, &Nutriments100g::simple(62.0, 4.8, 3.2, 3.4))
            .unwrap();

        // 2. 保存食谱：100g 燕麦片 + 200ml 牛奶，分 2 份
        let ingredients = vec![
            crate::calc::recipe::RecipeIngredientInput::simple(
                "food_oats_test",
                "测试燕麦片",
                100.0,
                "g",
                Nutriments100g::simple(380.0, 66.0, 13.0, 7.0),
            ),
            crate::calc::recipe::RecipeIngredientInput::simple(
                "food_milk_test",
                "全脂鲜牛奶",
                200.0,
                "ml",
                Nutriments100g::simple(62.0, 4.8, 3.2, 3.4),
            ),
        ];

        let saved = storage
            .save_recipe(
                "user_chef",
                Some("recipe_oatmeal_bowl"),
                "早餐燕麦碗",
                Some("营养丰盛的快手早餐"),
                2.0,
                None,
                &ingredients,
            )
            .unwrap();

        assert_eq!(saved.recipe.id, "recipe_oatmeal_bowl");
        assert_eq!(saved.recipe.total_weight_g, 300.0);
        assert_eq!(saved.ingredients.len(), 2);

        // 3. 验证可通过 get_recipe 查询食谱详情
        let retrieved = storage.get_recipe("recipe_oatmeal_bowl").unwrap();
        assert!(retrieved.is_some());
        let details = retrieved.unwrap();
        assert_eq!(details.recipe.name, "早餐燕麦碗");
        assert_eq!(details.ingredients.len(), 2);

        // 4. 验证食谱已自动索引至 FTS5 全文检索引擎并可被搜索
        let search = storage.search_foods_fts("燕麦碗", 5).unwrap();
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].id, "recipe_oatmeal_bowl");

        // 5. 验证用户可列出自建食谱
        let user_recipes = storage.list_user_recipes("user_chef").unwrap();
        assert_eq!(user_recipes.len(), 1);

        // 6. 删除食谱
        storage.delete_recipe("recipe_oatmeal_bowl").unwrap();
        assert!(storage.get_recipe("recipe_oatmeal_bowl").unwrap().is_none());
        assert!(storage.search_foods_fts("燕麦碗", 5).unwrap().is_empty());
    }

    #[test]
    fn test_storage_weight_and_intakes_trends() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user = crate::models::UserProfile::new(
            32,
            178.0,
            82.0,
            crate::models::Gender::Male,
            crate::models::ActivityLevel::LowActive,
        )
        .unwrap();
        storage.insert_user("user_trends", "Eric", "1992-11-04", &user).unwrap();

        // 1. 记录体重打卡轨迹
        storage
            .log_weight("user_trends", 82.0, Some(22.0), "2026-09-01T07:00:00", None)
            .unwrap();
        storage
            .log_weight("user_trends", 81.4, Some(21.7), "2026-09-08T07:00:00", None)
            .unwrap();
        storage
            .log_weight("user_trends", 80.9, Some(21.4), "2026-09-15T07:00:00", None)
            .unwrap();

        let weight_history = storage.get_weight_history("user_trends", 10).unwrap();
        assert_eq!(weight_history.len(), 3);
        assert_eq!(weight_history[0].1, 82.0);
        assert_eq!(weight_history[2].1, 80.9);

        // 使用 calc::trends 计算体重预测
        let proj = crate::calc::trends::weight_projection(&weight_history, Some(75.0)).unwrap();
        assert!(proj.rate_per_week_kg < 0.0);
        assert!(proj.weeks_to_target.is_some());
    }

    #[test]
    fn test_storage_activity_logging_and_totals() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user_id = "test_athlete";

        let act1 = ActivityLogRecord {
            id: "act_001".into(),
            user_id: user_id.into(),
            activity_code: "running_10kph".into(),
            activity_name: "常规跑步 (10 km/h)".into(),
            category: "running".into(),
            duration_minutes: 45.0,
            met_value: 9.8,
            gross_burned_kcal: 588.0,
            net_credited_kcal: 205.8,
            compensation_ratio: 0.35,
            date: "2026-09-15".into(),
            created_at: "2026-09-15T08:30:00Z".into(),
            external_id: Some("healthkit_workout_999".into()),
        };

        let act2 = ActivityLogRecord {
            id: "act_002".into(),
            user_id: user_id.into(),
            activity_code: "strength_training_general".into(),
            activity_name: "抗阻力量训练".into(),
            category: "conditioning".into(),
            duration_minutes: 60.0,
            met_value: 5.0,
            gross_burned_kcal: 400.0,
            net_credited_kcal: 340.0,
            compensation_ratio: 0.85,
            date: "2026-09-15".into(),
            created_at: "2026-09-15T17:00:00Z".into(),
            external_id: None,
        };

        storage.log_activity(&act1).unwrap();
        storage.log_activity(&act2).unwrap();

        // 外部健康 ID 查询防重复测试
        let fetched_by_ext = storage
            .get_activity_by_external_id(user_id, "healthkit_workout_999")
            .unwrap();
        assert!(fetched_by_ext.is_some());
        assert_eq!(fetched_by_ext.unwrap().id, "act_001");

        // 按日期查询
        let list = storage.get_activities_for_date(user_id, "2026-09-15").unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].activity_code, "running_10kph");
        assert_eq!(list[0].external_id, Some("healthkit_workout_999".into()));
        assert_eq!(list[1].activity_code, "strength_training_general");
        assert_eq!(list[1].external_id, None);

        // 聚合当日总和
        let (gross, net) = storage.get_daily_activity_totals(user_id, "2026-09-15").unwrap();
        assert!((gross - 988.0).abs() < 0.1);
        assert!((net - 545.8).abs() < 0.1);

        // 删除单项记录
        let deleted = storage.delete_activity(user_id, "act_001").unwrap();
        assert!(deleted);

        let list_after = storage.get_activities_for_date(user_id, "2026-09-15").unwrap();
        assert_eq!(list_after.len(), 1);
        assert_eq!(list_after[0].id, "act_002");
    }

    #[test]
    fn test_storage_user_goal_management() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user_id = "user_goal_test";

        // 1. 初始状态无目标
        assert!(storage.get_user_goal(user_id).unwrap().is_none());

        // 2. 设置减重目标
        let goal = UserGoalRecord {
            id: "goal_001".to_string(),
            user_id: user_id.to_string(),
            kind: "lose_weight".to_string(),
            target_weight_kg: Some(72.5),
            weekly_rate_kg: -0.5,
            taper_enabled: true,
            adaptive_enabled: true,
            manual_calorie_offset: -50.0,
            created_at: "2026-09-15T10:00:00Z".to_string(),
            updated_at: "2026-09-15T10:00:00Z".to_string(),
        };

        storage.set_user_goal(&goal).unwrap();

        // 3. 读取并验证
        let retrieved = storage.get_user_goal(user_id).unwrap();
        assert!(retrieved.is_some());
        let g = retrieved.unwrap();
        assert_eq!(g.kind, "lose_weight");
        assert_eq!(g.target_weight_kg, Some(72.5));
        assert_eq!(g.weekly_rate_kg, -0.5);
        assert!(g.taper_enabled);
        assert!(g.adaptive_enabled);
        assert_eq!(g.manual_calorie_offset, -50.0);

        // 4. 更新目标为维持
        let mut updated_goal = g;
        updated_goal.kind = "maintain_weight".to_string();
        updated_goal.weekly_rate_kg = 0.0;
        storage.set_user_goal(&updated_goal).unwrap();

        let g_updated = storage.get_user_goal(user_id).unwrap().unwrap();
        assert_eq!(g_updated.kind, "maintain_weight");
        assert_eq!(g_updated.weekly_rate_kg, 0.0);

        // 5. 删除目标
        let deleted = storage.delete_user_goal(user_id).unwrap();
        assert!(deleted);
        assert!(storage.get_user_goal(user_id).unwrap().is_none());
    }

    #[test]
    fn test_storage_day_boundary_daily_summary() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user_id = "test_night_owl";

        let user = crate::models::UserProfile::new(
            26,
            175.0,
            70.0,
            crate::models::Gender::Male,
            crate::models::ActivityLevel::Active,
        )
        .unwrap();
        storage.insert_user(user_id, "熬夜测试员", "1998-05-10", &user).unwrap();

        let food = FoodRecord {
            id: "food_test_snack".into(),
            name: "夜宵牛肉干".into(),
            brand: None,
            source: FoodSource::Custom,
            serving_quantity: Some(100.0),
            serving_unit: Some("g".into()),
            image_url: None,
        };
        let nutriments = Nutriments100g {
            energy_kcal_100: 300.0,
            carbohydrates_100: 5.0,
            proteins_100: 40.0,
            fat_100: 10.0,
            ..Default::default()
        };
        storage.insert_food(&food, &nutriments).unwrap();

        // 晚餐打卡：2026-09-14 20:00 (500 kcal)
        storage
            .log_intake(
                user_id,
                "food_test_snack",
                166.666, // ~500 kcal
                "g",
                MealType::Dinner,
                "2026-09-14T20:00:00",
            )
            .unwrap();

        // 跨午夜夜宵打卡：2026-09-15 02:30 (300 kcal)
        storage
            .log_intake(
                user_id,
                "food_test_snack",
                100.0, // 300 kcal
                "g",
                MealType::Snack,
                "2026-09-15T02:30:00",
            )
            .unwrap();

        // 1. 标准自然日（午夜 00:00 界线）
        let standard_14 = storage
            .get_daily_summary_with_boundary(user_id, "2026-09-14", 0)
            .unwrap();
        assert_eq!(standard_14.items_count, 1);
        assert_eq!(standard_14.total_energy_kcal, 500.0);

        let standard_15 = storage
            .get_daily_summary_with_boundary(user_id, "2026-09-15", 0)
            .unwrap();
        assert_eq!(standard_15.items_count, 1);
        assert_eq!(standard_15.total_energy_kcal, 300.0);

        // 2. 生理日界线（凌晨 04:00 界线，即 240 分钟偏移）
        // 9月14日的生理日覆盖：2026-09-14 04:00:00 至 2026-09-15 04:00:00
        let bio_14 = storage
            .get_daily_summary_with_boundary(user_id, "2026-09-14", 240)
            .unwrap();
        assert_eq!(bio_14.items_count, 2);
        assert_eq!(bio_14.total_energy_kcal, 800.0);

        // 9月15日的生理日覆盖：2026-09-15 04:00:00 至 2026-09-16 04:00:00
        let bio_15 = storage
            .get_daily_summary_with_boundary(user_id, "2026-09-15", 240)
            .unwrap();
        assert_eq!(bio_15.items_count, 0);
        assert_eq!(bio_15.total_energy_kcal, 0.0);
    }

    #[test]
    fn test_storage_custom_food_lifecycle() {
        let storage = StorageEngine::open_in_memory().unwrap();

        let draft = crate::calc::CustomFoodDraft {
            name: "自制减脂鹰嘴豆泥".to_string(),
            brand: Some("家庭厨房".to_string()),
            barcode: None,
            basis: Some(crate::calc::InputBasis::PerServing {
                serving_amount: 50.0,
                unit: "g".to_string(),
            }),
            energy_kcal: 85.0,
            proteins_g: 4.5,
            carbs_g: 10.0,
            fat_g: 3.2,
            fiber_g: Some(3.0),
            sodium_mg: Some(150.0),
            ..Default::default()
        };

        let norm = crate::calc::CustomFoodEngine::normalize_draft(&draft).unwrap();
        storage.insert_food(&norm.food, &norm.nutriments_100g).unwrap();

        // 1. 验证自建食物列表包含该项
        let custom_list = storage.list_custom_foods().unwrap();
        assert_eq!(custom_list.len(), 1);
        assert_eq!(custom_list[0].food.name, "自制减脂鹰嘴豆泥");
        assert_eq!(custom_list[0].nutriments.energy_kcal_100, 170.0);

        // 2. 验证 FTS 全文检索
        let search = storage.search_foods_fts("鹰嘴豆泥", 5).unwrap();
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].id, norm.food.id);

        // 3. 删除自建食物
        let deleted = storage.delete_custom_food(&norm.food.id).unwrap();
        assert!(deleted);
        assert!(storage.list_custom_foods().unwrap().is_empty());
        assert!(storage.get_food_with_nutriments(&norm.food.id).unwrap().is_none());
    }

    #[test]
    fn test_storage_full_crud_and_cascade_delete() {
        let storage = StorageEngine::open_in_memory().unwrap();
        let user_id = "user_crud_test";

        // 1. 创建用户并验证 list_all_users
        let profile = crate::models::UserProfile::new(
            25,
            175.0,
            70.0,
            crate::models::Gender::Male,
            crate::models::ActivityLevel::Active,
        )
        .unwrap();
        storage
            .insert_user(user_id, "测试全能用户", "2000-01-01", &profile)
            .unwrap();

        let users = storage.list_all_users().unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, user_id);

        // 2. 插入食品
        let food = FoodRecord {
            id: "food_bread".into(),
            name: "全麦吐司".into(),
            brand: None,
            source: FoodSource::Custom,
            serving_quantity: Some(50.0),
            serving_unit: Some("g".into()),
            image_url: None,
        };
        let nutriments = Nutriments100g::simple(250.0, 45.0, 10.0, 3.0);
        storage.insert_food(&food, &nutriments).unwrap();

        // 3. 测试 log_intake -> update_intake -> delete_intake
        let intake = storage
            .log_intake(
                user_id,
                "food_bread",
                100.0,
                "g",
                MealType::Breakfast,
                "2026-09-15T08:00:00",
            )
            .unwrap();
        assert_eq!(intake.snapshot_energy_kcal, 250.0);

        // 更新摄入量从 100g 到 150g，餐别改为 Snack
        let updated = storage
            .update_intake(user_id, &intake.id, 150.0, Some(MealType::Snack))
            .unwrap();
        assert!(updated);

        let summary = storage.get_daily_summary(user_id, "2026-09-15").unwrap();
        assert_eq!(summary.items_count, 1);
        assert_eq!(summary.logs[0].amount, 150.0);
        assert_eq!(summary.logs[0].meal_type, MealType::Snack);
        assert_eq!(summary.logs[0].snapshot_energy_kcal, 375.0); // 250 * 1.5

        // 删除摄入
        let intake_deleted = storage.delete_intake(user_id, &intake.id).unwrap();
        assert!(intake_deleted);
        assert_eq!(storage.get_daily_summary(user_id, "2026-09-15").unwrap().items_count, 0);

        // 4. 测试 log_weight -> delete_weight_log
        let weight_id = storage
            .log_weight(user_id, 69.5, Some(15.0), "2026-09-15T07:00:00", None)
            .unwrap();
        assert_eq!(storage.list_all_weights(user_id).unwrap().len(), 1);
        let weight_deleted = storage.delete_weight_log(user_id, &weight_id).unwrap();
        assert!(weight_deleted);
        assert_eq!(storage.list_all_weights(user_id).unwrap().len(), 0);

        // 5. 测试 log_water -> delete_water_log
        let water_id = storage.log_water(user_id, 350, "2026-09-15T09:00:00").unwrap();
        assert_eq!(storage.list_all_waters(user_id).unwrap().len(), 1);
        let water_deleted = storage.delete_water_log(user_id, &water_id).unwrap();
        assert!(water_deleted);
        assert_eq!(storage.list_all_waters(user_id).unwrap().len(), 0);

        // 6. 测试 start_fasting -> delete_fasting_session
        let fast_id = storage.start_fasting(user_id, "2026-09-15T20:00:00Z", 960).unwrap();
        assert_eq!(storage.list_all_fasting(user_id).unwrap().len(), 1);
        let fast_deleted = storage.delete_fasting_session(user_id, &fast_id).unwrap();
        assert!(fast_deleted);
        assert_eq!(storage.list_all_fasting(user_id).unwrap().len(), 0);

        // 7. 测试 delete_all_user_data 级联清空整个用户的所有关联数据
        // 重新灌入一些数据
        storage
            .log_weight(user_id, 70.0, None, "2026-09-15T07:00:00", None)
            .unwrap();
        storage.log_water(user_id, 500, "2026-09-15T09:00:00").unwrap();
        let user_deleted = storage.delete_all_user_data(user_id).unwrap();
        assert!(user_deleted);

        // 验证用户已被删除，外键级联也清空了子表
        assert!(storage.get_user(user_id).unwrap().is_none());
        assert_eq!(storage.list_all_users().unwrap().len(), 0);
        assert_eq!(storage.list_all_weights(user_id).unwrap().len(), 0);
        assert_eq!(storage.list_all_waters(user_id).unwrap().len(), 0);
    }
}
