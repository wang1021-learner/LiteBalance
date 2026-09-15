//! SQLite 数据库 DDL 建表语句与模式定义。

pub const CREATE_SCHEMA_SQL: &str = r#"
-- 1. 用户生理档案表 (users)
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    birthday TEXT NOT NULL,
    height_cm REAL NOT NULL,
    weight_kg REAL NOT NULL,
    gender TEXT NOT NULL,
    hormone_profile TEXT,
    activity_level TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 2. 食品基础元数据表 (foods)
CREATE TABLE IF NOT EXISTS foods (
    id TEXT PRIMARY KEY,               -- 条形码 (如 EAN-13) 或 UUID
    name TEXT NOT NULL,
    brand TEXT,
    source TEXT NOT NULL,              -- 'off' (Open Food Facts), 'fdc' (USDA), 'custom', 'recipe'
    serving_quantity REAL,             -- 份量数值，如 30.0
    serving_unit TEXT,                 -- 单位，如 'g', 'ml', 'serving', 'oz'
    image_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 3. 食品每100g/100ml标准营养素表 (food_nutriments)
CREATE TABLE IF NOT EXISTS food_nutriments (
    food_id TEXT PRIMARY KEY REFERENCES foods(id) ON DELETE CASCADE,
    energy_kcal_100 REAL NOT NULL,
    carbohydrates_100 REAL NOT NULL DEFAULT 0.0,
    proteins_100 REAL NOT NULL DEFAULT 0.0,
    fat_100 REAL NOT NULL DEFAULT 0.0,
    sugars_100 REAL,
    saturated_fat_100 REAL,
    fiber_100 REAL,
    monounsaturated_fat_100 REAL,
    polyunsaturated_fat_100 REAL,
    trans_fat_100 REAL,
    cholesterol_mg_100 REAL,
    sodium_mg_100 REAL,
    potassium_mg_100 REAL,
    magnesium_mg_100 REAL,
    calcium_mg_100 REAL,
    iron_mg_100 REAL,
    zinc_mg_100 REAL,
    phosphorus_mg_100 REAL,
    vitamin_a_ug_100 REAL,
    vitamin_c_mg_100 REAL,
    vitamin_d_ug_100 REAL,
    vitamin_b6_mg_100 REAL,
    vitamin_b12_ug_100 REAL,
    niacin_mg_100 REAL
);

-- 4. 食品全文检索 FTS5 虚表 (foods_fts)
CREATE VIRTUAL TABLE IF NOT EXISTS foods_fts USING fts5(
    id UNINDEXED,
    name,
    brand,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- 保持 FTS5 全文检索引擎与 foods 基础表同步的触发器
CREATE TRIGGER IF NOT EXISTS foods_ai AFTER INSERT ON foods BEGIN
    INSERT INTO foods_fts(id, name, brand) VALUES (new.id, new.name, new.brand);
END;

CREATE TRIGGER IF NOT EXISTS foods_ad AFTER DELETE ON foods BEGIN
    DELETE FROM foods_fts WHERE id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS foods_au AFTER UPDATE ON foods BEGIN
    DELETE FROM foods_fts WHERE id = old.id;
    INSERT INTO foods_fts(id, name, brand) VALUES (new.id, new.name, new.brand);
END;

-- 5. 每日食物摄入打卡日志表 (intake_logs)
CREATE TABLE IF NOT EXISTS intake_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    food_id TEXT NOT NULL REFERENCES foods(id),
    amount REAL NOT NULL,              -- 摄入数量（克或毫升）
    unit TEXT NOT NULL,                -- 'g', 'ml', 'serving'
    meal_type TEXT NOT NULL,           -- 'breakfast', 'lunch', 'dinner', 'snack'
    consumed_at TEXT NOT NULL,         -- ISO-8601 日期字符串 YYYY-MM-DDTHH:MM:SS
    -- 摄入瞬间锁定的历史营养素不可变快照
    snapshot_energy_kcal REAL NOT NULL,
    snapshot_protein_g REAL NOT NULL,
    snapshot_carbs_g REAL NOT NULL,
    snapshot_fat_g REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_intakes_user_date ON intake_logs(user_id, consumed_at);

-- 6. 体重与体成分打卡历史表 (weight_logs)
CREATE TABLE IF NOT EXISTS weight_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    logged_at TEXT NOT NULL,           -- ISO-8601
    weight_kg REAL NOT NULL,
    body_fat_pct REAL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_weight_user_date ON weight_logs(user_id, logged_at);

-- 7. 间歇性断食打卡记录表 (fasting_sessions)
CREATE TABLE IF NOT EXISTS fasting_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL,          -- ISO-8601
    target_duration_minutes INTEGER NOT NULL,
    completed_at TEXT,                 -- ISO-8601
    cancelled_at TEXT,                 -- ISO-8601
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_fasting_user ON fasting_sessions(user_id, started_at);

-- 8. 每日饮水打卡日志表 (water_logs)
CREATE TABLE IF NOT EXISTS water_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    logged_at TEXT NOT NULL,           -- ISO-8601
    amount_ml INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_water_user_date ON water_logs(user_id, logged_at);

-- 9. 用户自建烹饪食谱表 (recipes)
CREATE TABLE IF NOT EXISTS recipes (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    servings REAL NOT NULL DEFAULT 1.0,
    total_weight_g REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_recipes_user ON recipes(user_id);

-- 10. 食谱原料明细表 (recipe_ingredients)
CREATE TABLE IF NOT EXISTS recipe_ingredients (
    id TEXT PRIMARY KEY,
    recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    food_id TEXT NOT NULL REFERENCES foods(id),
    amount REAL NOT NULL,
    unit TEXT NOT NULL,
    converted_amount_g REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_recipe_ingredients ON recipe_ingredients(recipe_id);

-- 11. 运动与体力活动打卡日志表 (activity_logs)
CREATE TABLE IF NOT EXISTS activity_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_code TEXT NOT NULL,
    activity_name TEXT NOT NULL,
    category TEXT NOT NULL,
    duration_minutes REAL NOT NULL,
    met_value REAL NOT NULL,
    gross_burned_kcal REAL NOT NULL,
    net_credited_kcal REAL NOT NULL,
    compensation_ratio REAL NOT NULL,
    date TEXT NOT NULL,                -- YYYY-MM-DD
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    external_id TEXT UNIQUE            -- 外部健康平台同步防重 ID (Apple HealthKit / Health Connect)
);

CREATE INDEX IF NOT EXISTS idx_activity_user_date ON activity_logs(user_id, date);
CREATE INDEX IF NOT EXISTS idx_activity_external_id ON activity_logs(external_id);

-- 12. 用户目标与自适应卡路里预算配置表 (user_goals)
CREATE TABLE IF NOT EXISTS user_goals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,                -- 'lose_weight', 'maintain_weight', 'gain_weight'
    target_weight_kg REAL,
    weekly_rate_kg REAL NOT NULL DEFAULT 0.0,
    taper_enabled INTEGER NOT NULL DEFAULT 1,
    adaptive_enabled INTEGER NOT NULL DEFAULT 1,
    manual_calorie_offset REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_user_goals_user ON user_goals(user_id);
"#;
