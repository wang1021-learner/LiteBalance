# Walkthrough: Data Migration, Clinical DRI Radar, and Periodic Analytics

We have completed the implementation, verification, and CLI integration for the three requested non-AI core subsystems in `nutritracker-core`:
1. **Data Import & Export Layer** (`export_import.rs`)
2. **Clinical Micronutrient DRI Evaluation Engine** (`micronutrient_eval.rs`)
3. **Periodic Analytics & Metabolic Reporting Engine** (`analytics_reporting.rs`)

---

## 1. What Was Built

```
nutritracker-core/
├── Cargo.toml                              [Added csv 1.3]
└── src/
    ├── calc/
    │   ├── micronutrient_eval.rs          [NEW: Clinical NASEM DRI Evaluation & Anti-False-Alarm]
    │   ├── analytics_reporting.rs         [NEW: Periodic Deficit, Adherence, & Metabolic Divergence]
    │   └── mod.rs                         [Exposed new calc modules & structs]
    ├── storage/
    │   ├── export_import.rs               [NEW: Native Backup & JSON/CSV Import/Export]
    │   ├── models.rs                      [Extended records: UserProfile, Weight, Water, Fasting, FoodWithNutriments]
    │   ├── db.rs                          [Added history queries, raw inserts, ensure_user_exists, list helpers]
    │   └── mod.rs                         [Exposed export_import module and records]
    ├── lib.rs                             [Re-exported all public APIs]
    └── main.rs                            [Added export, import, dri, and report CLI commands]
```

---

## 2. Technical Highlights & Scientific Integrity

### A. Data Import & Export Layer (`export_import.rs`)
- **Import formats**: Supports JSON/CSV intake, weight, and recipe import formats (`user_intake.json`, `weight_log.json`, `user_recipes.json`, and RFC-4180 `user_intake.csv`).
- **Native Complete Backup**: `NutriTrackerBackup` serializes the full local SQLite database state (User Profile, Foods with 24-column `Nutriments100g`, Intakes, Body Weight scale entries, Water logs, Fasting sessions, and Recipes).
- **Relational Integrity**: Automatically ensures `user_id` exists before inserting child records, avoiding SQLite foreign key cascade issues.

### B. Clinical Micronutrient DRI Evaluation Engine (`micronutrient_eval.rs`)
- **Authoritative Reference Matrix**: Encodes official NASEM Recommended Dietary Allowances (RDA), Adequate Intakes (AI), and Tolerable Upper Intake Levels (UL) / CDRR for 14 essential vitamins and minerals across sex and age groups:
  - *Sex/Age Differentiated*: Pre-menopausal females (18 mg Iron) vs Males/Post-menopausal females (8 mg Iron); Calcium target transitions at 50/70 years.
  - *Safety Guardrails*: Tolerable Upper Intake Levels (UL) for Sodium (2300 mg), Calcium (2500 mg), Iron (45 mg), Zinc (40 mg), Vitamin A (3000 µg), Vitamin C (2000 mg), Niacin (35 mg).
- **Anti-False-Alarm Guardrail**: Resolves the classic nutrition app pitfall where missing nutrient data in commercial food databases is mistakenly treated as zero intake, causing false user alarm. If $<50\%$ of consumed food mass reports a given micronutrient, it is classified as `LowDataCoverage` instead of sounding a false `Deficient` warning.

### C. Periodic Analytics & Metabolic Reporting Engine (`analytics_reporting.rs`)
- **Energy Deficit & Scale Convergence**: Integrates cumulative caloric intake against target TDEE ($\Delta kcal / 7700$).
- **Metabolic Divergence**: Compares the ordinary least squares (OLS) linear regression of actual weight scale history against the theoretical deficit model ($\Delta kg_{\text{actual}} - \Delta kg_{\text{theory}}$), diagnosing metabolic adaptation, water retention, or untracked condiments.
- **Macronutrient Adherence**: Calculates actual energy percentage splits (% protein, % carbs, % fat) and protein sufficiency index ($\ge 90\%$ of FFM target).

---

## 3. Verification Results

### Automated Tests (43/43 Passing, 0 Failures)
```powershell
cargo test
```
```
running 43 tests
test calc::energy::tests::test_nasem_2023_female_low_active ... ok
test calc::dynamic_weight::tests::test_dynamic_weight_loss_plateau ... ok
test calc::analytics_reporting::tests::test_macro_split_and_protein_compliance ... ok
test calc::analytics_reporting::tests::test_metabolic_divergence_calculation ... ok
test calc::energy::tests::test_comparison_shows_refinement ... ok
test calc::energy::tests::test_nasem_2023_male_very_active ... ok
test calc::fasting::tests::test_fasting_complete ... ok
test calc::fasting::tests::test_recurring_24h_cycle_modulo ... ok
test calc::micronutrient_eval::tests::test_excessive_sodium_triggers_ul_warning ... ok
test calc::micronutrient_eval::tests::test_iron_target_gender_and_age_differentiation ... ok
test calc::micronutrient_eval::tests::test_low_data_coverage_suppresses_false_deficiency ... ok
test calc::recipe::tests::test_density_conversion_olive_oil ... ok
test calc::recipe::tests::test_nested_recipe_coverage_propagation ... ok
test calc::recipe::tests::test_partial_coverage_zinc_not_diluted ... ok
test storage::export_import::tests::test_native_backup_and_restore ... ok
test storage::export_import::tests::test_csv_export_and_reimport_roundtrip ... ok
...
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### Static Analysis (0 Warnings)
```powershell
cargo clippy -- -D warnings
```
Result: **Zero compiler errors and zero clippy warnings.**

### Interactive CLI Verification
- `cargo run -- dri`: Displays clinical micronutrient radar with coverage percentages and safety warnings.
- `cargo run -- report --days 7`: Generates 7-day metabolic divergence and macro adherence analysis.
- `cargo run -- export --format json`: Successfully created complete native backup JSON.
- `cargo run -- export --format csv`: Successfully exported RFC-4180 `user_intake.csv`.

---

## 4. 全工程中文注释与终端交互本地化 (Codebase-wide Chinese Documentation)

根据用户指令 **“全部改成中文注释”**，已完成对整个 `nutritracker-core` 核心代码库 23 个源文件的全量中文注释、临床医学术语规范化与 CLI 终端交互汉化：

1. **核心计算层 (`src/calc/`)**：
   - `energy.rs`: NASEM 2023 成人 8 分组能量回归矩阵、IAEA 双标水数据库、老版 IOM 2005 方程对比。
   - `dynamic_weight.rs`: NIH Kevin Hall 动态常微分方程 (ODE)、Forbes 体成分动态划分、逆向二分求解器。
   - `workout_compensation.rs`: Pontzer-Trexler 2026、Careau 2021、Flanagan 2024、Howard 2025 运动补偿模型。
   - `macro_engine.rs`: 宏量营养素三大协议（高蛋白均衡/低碳生酮/地中海）、去脂体重 (FFM) 锚定、碳循环周期编排。
   - `micronutrient_eval.rs`: NASEM DRI（RDA/AI/UL/CDRR）临床达标雷达、防虚假报警（缺失数据不按 0 计算）。
   - `recipe.rs`: 单次累加器、液体密度换算（如橄榄油 0.918g/ml）、未知微量元素覆盖率独立计算。
   - `water.rs`: 临床基准水分摄入计算（NASEM AI 对比 35ml/kg 教练经验法则）。
   - `fasting.rs`: 16:8 / 18:6 / 20:4 间歇性断食状态机、无状态纯函数计算、防系统休眠漂移。
   - `trends.rs`: 连续打卡 Streak 统计、最小二乘法 (OLS) 体重趋势回归预测、30 天营养移动均值。
   - `analytics_reporting.rs`: 周期性代谢平衡偏离度、热量缺口理论与实测收敛度、宏量依从度分析。

2. **存储与持久化层 (`src/storage/`)**：
   - `db.rs`: SQLite 连接池互斥锁、FTS5 全文检索及多词包含降级兼容、全部实体 CRUD。
   - `schema.rs`: 完整的 SQL DDL 模式定义、FTS5 虚表与触发器。
   - `models.rs`: 数据库不可变快照结构体、打卡记录、食谱明细。
   - `seed.rs`: 离线基础食物库中英文双语命名与标准微量元素预植入。
   - `error.rs`: 存储与数据库异常定义。

3. **模型与入口层 (`src/models/`, `src/lib.rs`, `src/main.rs`)**：
   - `lib.rs`: 库根级模块架构说明与公共符号导出。
   - `user.rs`: 用户生理画像、内分泌表型与非二元性别计算模式。
   - `main.rs`: 命令行工作台 (Workbench) 帮助手册、横幅、指令解析与交互输出全汉化。

4. **质量验证保障**：
   - **52 / 52 项自动化单元测试全部通过**。
   - **`cargo clippy -- -D warnings` 零警告通过**。

---

## 5. 外部食品数据库与条形码查询客户端 (OpenFoodFacts & Barcode Client)

已严格按照要求与临床级防污染规范，实现外部食品条码查询与网络数据处理模块：

```
nutritracker-core/
├── Cargo.toml                          [新增 reqwest 0.12 (json, blocking)]
└── src/
    ├── client/
    │   ├── mod.rs                      [客户端模块注册与符号重导出]
    │   ├── barcode.rs                  [NEW: GS1 模 10 校验、UPC-A 补零升格为 EAN-13、EAN-8 验证]
    │   └── remote_food_client.rs       [NEW: 强制合规 UA、滑动窗口限速器、零污染清洗器、OFF API v2 客户端]
    ├── storage/
    │   ├── cache_db.rs                 [NEW: 独立网络缓存库 food_cache.db、TTL 校验、自动驱逐、ODbL 隔离]
    │   └── mod.rs                      [导出 CacheStorageEngine, CachedFoodRecord, CacheStats]
    └── main.rs                         [新增 barcode 与 cache 指令，支持离线命中、--save 归档入库与缓存管理]
```

### A. 核心技术规范与合规实现

1. **GS1 模 10 (Modulo 10) 严格校验位算法 (`barcode.rs`)**：
   - 支持 EAN-13、UPC-A、EAN-8 三类条码标准。
   - 输入预清洗：自动剔除首尾空白、内部空格以及常见扫码枪产生的短横线（`-`）。
   - 长度与数字完整性校验：非法非数字或非 8/12/13 位长度立即返回明确的中文错误。
   - 奇偶权值校验：严格按照 GS1 规范从右至左交替加权（UPC-A 与 EAN-13 奇位乘 3、偶位乘 1，EAN-8 反向）。
   - 自动规范化：北美 12 位 UPC-A 自动前缀补 `0` 升格为国际标准 13 位 EAN-13（如 `737628064502` 规范化为 `0737628064502`），确保底层存储与缓存索引全局唯一。

2. **强制合规 User-Agent 头（非爬虫标识）(`remote_food_client.rs`)**：
   - 绝不使用 reqwest 默认 UA，强制全局默认注入：
     `NutriTracker-Desktop/1.0 (https://github.com/nutritracker; contact@nutritracker.org)`
   - 彻底杜绝因缺少有效 UA 被 OpenFoodFacts 官方 WAF 拦截或封禁 IP 的风险。
   - 支持开发者在初始化或命令行通过 `--ua` 覆盖自定义标识。

3. **客户端滑动窗口安全速率限制器 (`SlidingWindowRateLimiter`)**：
   - 针对 OpenFoodFacts 官方未认证 IP 限制（普通请求 15 次/分），在客户端本地实现并发安全的滑动窗口计数器。
   - 设置最大 14 次/分钟的硬性本地安全保护护栏。
   - 突发请求超限时，计算并返回精确至秒级的需等待倒计时，防止 IP 触发服务端 429 降级惩罚。

4. **物理分库与 ODbL 数据版权隔离架构 (`cache_db.rs`)**：
   - **严格物理双库分离**：
     - 用户私有主库：`~/.nutritracker/nutritracker.db`（个人档案、打卡、体重、私有食谱）。
     - 独立网络缓存库：`~/.nutritracker/food_cache.db`（临时网络响应、TTL 记录、ODbL 署名）。
   - **零 ODbL 协议传染**：外部开源食品数据完全物理隔离存储，绝不污染用户私有数据。
   - **纯净轻量备份**：全量数据备份（`nutritracker_backup.json`）仅导出主库，不携带网络爬虫缓存。
   - **TTL 生命周期管理**：支持 30 天自动过期逻辑，提供 `evict_expired()` 定期清理与 `clear_all()` 一键重置。

5. **零污染营养素解析器 (`Nutriments100g`)**：
   - 严格遵循临床级医学准则：对于 OpenFoodFacts 响应中缺失或未填写的微量元素（如锌、镁、钾、维生素 D 等），解析器保持 `None`，**绝不将缺失数据自动填为 0.0**。
   - 彻底根除“因外部食品未标注微量元素导致计算时稀释用户全天平均摄入或诱发虚假缺乏症警告”的核心行业痛点。

---

### B. 交互与验证结果

#### 1. 自动化单元测试（52 项全部通过）
```powershell
cargo test
# running 52 tests ... 52 passed; 0 failed; 0 ignored; finished in 0.04s
```
- 新增 `barcode` GS1 模 10 校验测试（有效 EAN-13、UPC-A 补零升格、EAN-8、校验位错误拦截、非法字符拦截）。
- 新增 `remote_food_client` UA 校验测试、零污染数据保留测试、滑动窗口限速突发拦截测试。
- 新增 `cache_db` 独立缓存读写、TTL 过期判定、过期驱逐与全量清空测试。

#### 2. 代码规范与静态检查
```powershell
cargo clippy -- -D warnings
# Checking nutritracker-core v0.1.0 ... Finished in 1.32s (0 warnings)
```

#### 3. 命令行实机运行效果
- **GS1 错误校验拦截**：
  ```powershell
  cargo run -- barcode 737628064500
  # [错误] 条形码 GS1 校验失败: 条形码校验位错误: 声明末位为 '0'，GS1 计算期望为 '2'
  ```
- **实时联网检索与零污染展示**：
  ```powershell
  cargo run -- barcode 737628064502
  # * 原始输入条码:    737628064502
  # * 规范化标准条码:  0737628064502
  # * 条码标准类型:    UpcA
  # * GS1 校验状态:    通过 (模 10 校验位: 2)
  # * 客户端 User-Agent: NutriTracker-Desktop/1.0 (https://github.com/nutritracker; contact@nutritracker.org)
  # [命中状态]: [OpenFoodFacts 远程 API 实时拉取 (已自动写入本地缓存)]
  # * 商品名称:  Thai peanut noodle kit includes stir-fry rice noodles & thai peanut seasoning
  # * 生产厂商:  Simply Asia, Thai Kitchen
  # * 锌 (Zinc): 未标注/未知 (保持 None，避免数据稀释)
  # * 维生素 D:  未标注/未知 (保持 None，避免数据稀释)
  ```
- **本地独立离线缓存命中（毫秒级响应）**：
  ```powershell
  cargo run -- barcode 737628064502
  # [食品检索成功 [本地离线缓存命中 (food_cache.db)]]
  ```
- **一键持久化保存至用户主库并在 FTS5 检索**：
  ```powershell
  cargo run -- barcode 737628064502 --save
  # [已保存] 该食品已成功保存至本地用户主库 (nutritracker.db)！
  cargo run -- search Thai
  # 检索关键词: 'Thai' (共命中 1 条食物)
  # 0737628064502  Thai peanut noodle kit includes s...  52 g  385 kcal
  ```
- **网络缓存独立清空与主库数据隔离验证**：
  ```powershell
  cargo run -- cache --clear
  # 已成功清空外部食品网络缓存数据库！清理条目数: 1 条。用户核心数据库完好无损。
  cargo run -- search Thai
  # 用户主库中的食品依然完好可查！
  ```

---

## 6. 运动与体力活动日志追踪引擎 (ActivityTracker & EnergyBalance)

已全面完成【模块 1：运动与体力活动日志追踪引擎】的开发与系统整合，打通了“**摄入 - 基础维持能耗 - 运动名义消耗 - Pontzer 代偿扣除 - 净能量差**”的闭环链路：

```
nutritracker-core/
├── src/
│   ├── calc/
│   │   ├── activity.rs               [NEW: MET 运动库、粗消耗计算、模态映射与代偿联动]
│   │   └── mod.rs                    [导出 ActivityCatalogItem, DailyEnergyBalance, ActivityEnergyCalculator]
│   ├── storage/
│   │   ├── schema.rs                 [新增 activity_logs 关系表与索引]
│   │   ├── models.rs                 [新增 ActivityLogRecord 数据模型]
│   │   ├── db.rs                     [实现 log_activity, get_activities_for_date, delete_activity, get_daily_activity_totals]
│   │   ├── export_import.rs          [原生 JSON 备份完整纳管 activities]
│   │   └── mod.rs                    [导出 ActivityLogRecord]
│   ├── lib.rs                        [导出运动追踪公共 API]
│   └── main.rs                       [新增 activity 与 balance 命令行指令，支持代偿诊断]
```

### A. 核心技术特性

1. **临床标准 MET 运动知识库 (`Herrmann et al. 2024`)**：
   - 涵盖慢跑、常规跑步、进阶跑步、公路骑行、动感单车、抗阻力量训练、大重量力量举、自重健身、HIIT 循环、CrossFit、跳绳、游泳、球类等 18 项常见标准化运动。
   - 每项运动明确标定 MET 当量与所属 `ExerciseModality`（有氧耐力、抗阻力量、间歇综合、低强度日常）。

2. **Pontzer-Trexler 2026 运动能量代偿深度联动**：
   - **抗阻力量训练**：代偿极低（~10-15%），维持 **85%~90% 高计入率**。
   - **有氧耐力跑步/骑行**：代偿显著（约 65%~70%），**仅计入约 35%**，科学扣减基础能耗挤压。
   - 杜绝传统饮食 App 粗暴加和运动消耗导致的“跑完 500 kcal 多吃 500 kcal 进而复胖”的致命缺陷。

3. **每日全量能量闭环仪表盘 (Daily Energy Balance)**：
   $$\text{Adjusted TDEE} = \text{Base TDEE} + \text{Net Credited Activity}$$
   $$\text{Net Caloric Balance} = \text{Total Intake} - \text{Adjusted TDEE}$$
   - 自动判定是否处于热量赤字减脂状态，并基于 $7700 \text{ kcal} = 1 \text{ kg}$ 理论能量守恒预测每周体重变化率。
   - 输出代偿防护诊断，直观告知用户系统阻断了多少额外热量摄入风险。

4. **存储持久化与全量原生备份**：
   - SQLite `activity_logs` 完整记录名义消耗、净计入消耗与代偿折算率。
   - `export` 与 `import` 完整纳管运动日志，实现备份还原无损往返。

### B. 自动化测试与实机验证

- **单元测试**：`cargo test` **56 / 56 项测试全部通过**（耗时 0.04 秒）。
- **静态代码检查**：`cargo clippy -- -D warnings` **零警告通过**。
- **CLI 实机验证**：
  ```powershell
  # 1. 浏览标准 MET 运动库
  cargo run -- activity catalog --category running
  
  # 2. 记录一次 45 分钟常规跑步 (有氧代偿 ~65%)
  cargo run -- activity log --code running_10kph --duration 45
  # * 名义总能量消耗: 551.3 kcal (未经折算的表面能耗)
  # * 科学净计入消耗: 193.0 kcal (实际计入每日摄入预算，折算率 35%)
  # * 代偿抵消挤压:   358.3 kcal (被基础代谢与自发性活动吸收抵消)
  
  # 3. 记录一次 60 分钟力量训练 (抗阻代偿极小，计入率 90%)
  cargo run -- activity log --code strength_training_general --duration 60
  # * 名义总能量消耗: 375.0 kcal
  # * 科学净计入消耗: 337.5 kcal (折算率 90%)
  
  # 4. 查看当日运动列表与汇总
  cargo run -- activity list
  # 当日合计: 名义消耗 926.3 kcal | 净计入 530.5 kcal | 代偿抵消扣除 395.8 kcal
  
  # 5. 查看每日全量能量闭环仪表盘
  cargo run -- balance
  # * 实际饮食摄入总热量:    368 kcal
  # * 静息基准维持消耗 (TDEE): 3014 kcal (NASEM 2023 DRI 标准)
  # * 运动上报名义总消耗:    926 kcal
  # * 代偿吸收扣减能耗:      396 kcal
  # * 科学净有效计入消耗:    530 kcal
  # * 今日调整后真实总能耗:  3545 kcal
  # * 今日最终净能量差:      -3177 kcal  [热量赤字/减脂期: -3177 kcal]
  # * 临床提示: 根据 Pontzer-Trexler 2026 运动代偿模型，系统已科学抵扣 396 kcal 基础能耗挤压，避免了因高估运动消耗多吃而诱发体脂反弹。
  ```

