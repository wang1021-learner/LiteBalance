//! 轻衡 (LiteBalance Core) 移动端共享引擎本地无头测试工作台 (Developer Headless CLI)
//!
//! 提供临床级代谢规划、食物 FTS5 离线检索、饮食打卡、运动与代偿追踪、补水与断食追踪、
//! 微量元素雷达评估、周期性报表分析以及数据备份导入导出的本地无头调试工具（供移动端算法验证与批量回测）。
//!
//! 核心标准集成：
//! - NASEM 2023 DRI 能量方程矩阵
//! - NIH Kevin Hall 动态常微分方程 (ODE) 体重模型
//! - NASEM 微量元素达标雷达（RDA / AI / UL / CDRR）
//! - Pontzer-Trexler 2026 运动能量补偿非线性模型
//! - 本地优先 SQLite + FTS5 离线持久化存储

use std::env;
use std::path::PathBuf;

use litebalance_core::calc::{
    find_activity_by_code, get_standard_activity_catalog, weight_projection, ActivityEnergyCalculator,
    CustomFoodDraft, CustomFoodEngine, DailyEnergyBalance, DailyIntakeData, DailyWeightData,
    DayBoundaryConfig, DietProtocol, DriStatus, DynamicWeightPlanner, EnergyCalc, ExerciseModality,
    FastingProtocol, FastingSession, FastingState, GoalConfig, GoalKind, GoalProfileEngine,
    InputBasis, MacroEngine, MicronutrientEvaluator, NutritionMovingAverage, NutritionState,
    PeriodicAnalyticsEngine, UnitConverter, WaterCalc,
};
use litebalance_core::client::{BarcodeValidator, RemoteFoodClient};
use litebalance_core::models::{ActivityLevel, Gender, UserProfile};
use litebalance_core::storage::{
    seed_default_foods_if_empty, ActivityLogRecord, CacheStorageEngine, ExportImportEngine, NutriTrackerBackup,
    FoodRecord, FoodSource, MealType, StorageEngine, UserGoalRecord,
};

/// 获取本地 SQLite 数据库的存储路径（位于用户主目录下的 `.litebalance/litebalance.db`）。
fn get_db_path() -> PathBuf {
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_else(|_| ".".to_string());
    let data_dir = PathBuf::from(&home).join(".litebalance");
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join("litebalance.db")
}

/// 打开本地 SQLite 数据库存储引擎，并在初次启动为空时自动载入基础临床参考食物库。
fn open_app_storage() -> StorageEngine {
    let path = get_db_path();
    let storage = StorageEngine::open(path.to_str().unwrap()).expect("无法打开本地 SQLite 数据库");
    let _ = seed_default_foods_if_empty(&storage);
    storage
}

/// 获取外部网络食品缓存 SQLite 数据库存储路径（位于用户主目录下的 `.litebalance/food_cache.db`）。
fn get_cache_db_path() -> PathBuf {
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_else(|_| ".".to_string());
    let data_dir = PathBuf::from(&home).join(".litebalance");
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join("food_cache.db")
}

/// 打开外部网络食品缓存 SQLite 存储引擎（与用户核心数据库物理完全隔离）。
fn open_cache_storage() -> CacheStorageEngine {
    let path = get_cache_db_path();
    CacheStorageEngine::open(path.to_str().unwrap()).expect("无法打开外部网络食品缓存数据库")
}

/// 打印工作台欢迎横幅。
fn print_banner() {
    println!(r#"
=============================================================================
  轻衡 (LiteBalance Core) 工作台 (临床级代谢动力学与高精度营养计算引擎)
  - 遵循标准: NASEM 2023 DRI | NIH Kevin Hall ODE | Pontzer-Trexler 2026
  - 本地离线优先 SQLite + FTS5 全文检索 | 纯原生 Rust 实现
============================================================================="#);
}

/// 打印命令行帮助使用说明。
fn print_usage() {
    println!(r#"
用法: litebalance <COMMAND> [OPTIONS]

核心指令列表 (COMMANDS):
  plan        计算临床 TDEE、NIH Kevin Hall 动态体重变化轨迹与宏量分配
  search      使用 FTS5 全文索引在离线数据库中快速检索食物
  barcode     使用 GS1 模 10 校验并基于 OpenFoodFacts 检索条形码食品 (支持 EAN-13/UPC-A)
  cache       管理独立外部网络缓存数据库 (--stats 查看指标 / --clear 清空 / --evict 驱逐过期)
  activity    运动与体力活动打卡 (log 记录 / list 列表 / catalog 运动知识库 / delete 删除)
  balance     每日全量能量闭环仪表盘 (摄入 - 基础能耗 - 运动 - Pontzer 代偿 = 真实净能量差)
  goal        动态卡路里预算与自适应目标调节 (set 设置 / status 查看 / delete 清除)
  food-custom 本地自建食物全流程管理 (create 录入 / list 列表 / delete 删除)
  unit        多单位制高精度双向转换 (kg/lbs/stone, cm/in/ft, kcal/kJ, g/oz, ml/fl_oz)
  log-food    记录单次饮食摄入打卡日志
  summary     查看今日或指定日期的营养素摄入汇总 (支持 --boundary 生理日界线)
  water       记录饮水量或查看基于生理基准的补水达标进度
  fast        间歇性断食状态管理 (start 开启 / status 状态 / complete 结束 / cancel 取消)
  log-weight  记录体重与体成分（体脂率）打卡
  recipe      查看用户自建多原料聚合食谱及归一化营养素明细
  dri         临床 NASEM 微量元素雷达分析与防误报达标评估
  report      生成周期性代谢偏离度、热量缺口达成率与依从性报表
  export      导出全量原生备份 JSON 或标准兼容 CSV
  import      导入历史 JSON/CSV 或原生备份文件
  trends      查看体重回归趋势预测及 30 天营养移动均值
  user        多用户档案与关联数据管理 (list 列表 / delete 清空)
  seed        重新初始化/更新离线临床食物基准数据库

使用示例 (EXAMPLES):
  litebalance plan --age 28 --height 178 --weight 80 --gender male --activity active
  litebalance goal set --type lose --target 75.0 --rate -0.5
  litebalance goal status
  litebalance food-custom create --name "高蛋白乳清曲奇" --serving-size 60 --energy 220 --protein 18 --carbs 24 --fat 5.5
  litebalance food-custom list
  litebalance unit 80 kg
  litebalance unit 175 cm
  litebalance search 鸡胸肉
  litebalance barcode 737628064502
  litebalance activity catalog
  litebalance activity log --code running_10kph --duration 45
  litebalance activity list
  litebalance balance --boundary 04:00
  litebalance summary --boundary 04:00
  litebalance dri
  litebalance report --days 14
  litebalance export --format json --output backup.json
  litebalance import --file user_intake.json
  litebalance user list
"#);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_banner();
        print_usage();
        return;
    }

    let command = args[1].to_lowercase();
    match command.as_str() {
        "plan" => handle_plan(&args[2..]),
        "goal" => handle_goal(&args[2..]),
        "food-custom" => handle_food_custom(&args[2..]),
        "unit" => handle_unit(&args[2..]),
        "search" => handle_search(&args[2..]),
        "barcode" => handle_barcode(&args[2..]),
        "cache" => handle_cache(&args[2..]),
        "activity" => handle_activity(&args[2..]),
        "balance" => handle_balance(&args[2..]),
        "log-food" => handle_log_food(&args[2..]),
        "summary" => handle_summary(&args[2..]),
        "water" => handle_water(&args[2..]),
        "fast" => handle_fast(&args[2..]),
        "log-weight" => handle_log_weight(&args[2..]),
        "recipe" => handle_recipe(&args[2..]),
        "dri" => handle_dri(&args[2..]),
        "report" => handle_report(&args[2..]),
        "export" => handle_export(&args[2..]),
        "import" => handle_import(&args[2..]),
        "trends" => handle_trends(&args[2..]),
        "user" => handle_user(&args[2..]),
        "seed" => handle_seed(),
        "-h" | "--help" | "help" => print_usage(),
        _ => {
            eprintln!("未知指令: '{}'。运行 --help 查看完整使用指南。", command);
        }
    }
}

fn get_arg_val(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

/// 处理 `plan` 指令：执行临床代谢评估、动态体重仿真与宏量目标分配。
fn handle_plan(args: &[String]) {
    let age: u8 = get_arg_val(args, "--age").and_then(|s| s.parse().ok()).unwrap_or(30);
    let height: f64 = get_arg_val(args, "--height").and_then(|s| s.parse().ok()).unwrap_or(175.0);
    let weight: f64 = get_arg_val(args, "--weight").and_then(|s| s.parse().ok()).unwrap_or(80.0);
    let gender_str = get_arg_val(args, "--gender").unwrap_or_else(|| "male".to_string());
    let act_str = get_arg_val(args, "--activity").unwrap_or_else(|| "active".to_string());
    let target_weight: f64 = get_arg_val(args, "--target").and_then(|s| s.parse().ok()).unwrap_or(weight - 5.0);
    let weeks: u32 = get_arg_val(args, "--weeks").and_then(|s| s.parse().ok()).unwrap_or(12);

    let gender = match gender_str.to_lowercase().as_str() {
        "female" | "f" => Gender::Female,
        _ => Gender::Male,
    };

    let activity = match act_str.to_lowercase().as_str() {
        "inactive" | "sedentary" => ActivityLevel::Inactive,
        "low" | "lowactive" => ActivityLevel::LowActive,
        "very" | "veryactive" => ActivityLevel::VeryActive,
        _ => ActivityLevel::Active,
    };

    let profile = UserProfile::new(age, height, weight, gender, activity)
        .expect("无效的用户生理档案参数");

    println!("\n=======================================================");
    println!("  临床代谢评估与 NIH KEVIN HALL 动态体重规划");
    println!("=======================================================");
    println!(
        "用户画像: {} 岁, {:.1} cm, {:.1} kg | 性别: {:?} | 活动水平: {:?}",
        age, height, weight, gender, activity
    );
    println!(
        "减重目标: {:.1} kg（跨度 {} 周，目标平均减重速率: {:+.2} kg/周）",
        target_weight,
        weeks,
        (target_weight - weight) / weeks as f64
    );

    // 1. 能量代谢计算（NASEM 2023 DRI 对比老版 IOM 2005）
    let comp = EnergyCalc::compare(&profile);
    println!("\n--- 1. 每日总能量消耗 (TDEE) 与基础代谢 ---");
    println!("  * NASEM 2023 DRI:  {:.0} kcal/天 (基于 IAEA 双标水数据库更新回归模型)", comp.nasem_2023_kcal);
    println!(
        "  * 老版 IOM 2005:   {:.0} kcal/天 (绝对偏差: {:+.0} kcal, 相对偏差: {:+.1}%)",
        comp.iom_2005_kcal, comp.difference_kcal, comp.difference_pct
    );

    // 2. NIH Kevin Hall 动态常微分方程规划器
    println!("\n--- 2. NIH Kevin Hall 动态常微分方程求解与体成分预测 ---");
    let days = weeks * 7;
    let target_plan = DynamicWeightPlanner::solve_target_intake(
        &profile,
        None,
        target_weight,
        days as usize,
    );

    match &target_plan {
        Ok(daily_intake) => {
            println!(
                "  * 推荐每日摄入热量: {:.0} kcal/天 (每日热量缺口: {:+.0} kcal)",
                daily_intake,
                daily_intake - comp.nasem_2023_kcal
            );
            let sim = DynamicWeightPlanner::simulate(
                &profile,
                None,
                *daily_intake,
                days as usize,
                Some(target_weight),
            );
            println!(
                "  * 预测周期末体重: {:.2} kg (总变化量: {:+.2} kg)",
                sim.final_weight_kg, sim.total_weight_change_kg
            );
            println!(
                "  * 体成分变化划分: 脂肪组织 {:+.2} kg | 瘦体重 (FFM) {:+.2} kg",
                sim.fat_mass_change_kg, sim.fat_free_mass_change_kg
            );
            println!(
                "  * 适应性产热效应: -{:.1} kcal/天 (代谢适应下调)",
                sim.metabolic_adaptation_kcal
            );
            println!(
                "  * 传统 7700kcal 规则偏差: 传统线性规则虚高夸大减重达 {:.2} kg",
                sim.wishnofsky_overestimate_kg
            );
        }
        Err(e) => {
            println!("  * 规划求解器提示: {}", e);
        }
    }

    // 3. 宏量营养素目标分配
    let intake_for_macros = target_plan.unwrap_or(comp.nasem_2023_kcal - 500.0);

    let macros = MacroEngine::calculate(
        intake_for_macros,
        &profile,
        None,
        DietProtocol::HighProteinBalanced,
    );

    println!("\n--- 3. 宏量营养素目标分配 (高蛋白均衡协议) ---");
    println!(
        "  * 蛋白质:  {:.1} g ({:.0} kcal, 占总热量 {:.1}%) [基准: {:.1} g/kg 体重, {:.1} g/kg 瘦体重]",
        macros.protein_g, macros.protein_kcal, macros.protein_pct, macros.protein_per_kg_bw, macros.protein_per_kg_ffm
    );
    println!(
        "  * 碳水:    {:.1} g ({:.0} kcal, 占总热量 {:.1}%)",
        macros.carbs_g, macros.carbs_kcal, macros.carbs_pct
    );
    println!(
        "  * 脂肪:    {:.1} g ({:.0} kcal, 占总热量 {:.1}%) [{:.2} g/kg 体重]",
        macros.fat_g, macros.fat_kcal, macros.fat_pct, macros.fat_per_kg_bw
    );
    println!(
        "  * 膳食纤维: >= {:.1} g/天 (肠道益生菌安全推荐底线)",
        macros.fiber_g_min
    );

    // 4. 补水目标基准
    let water_target = WaterCalc::recommend_daily_target(&profile);
    println!("\n--- 4. 每日临床水分摄入基准 ---");
    println!("  * 基准饮水推荐量: {} ml/天 (NASEM 临床水分转化基准)\n", water_target);
}

/// 处理 `search` 指令：在离线 SQLite FTS5 数据库中检索食品。
fn handle_search(args: &[String]) {
    if args.is_empty() {
        eprintln!("错误: 请提供搜索关键词。例如: litebalance search 鸡胸肉");
        return;
    }

    let query = args.join(" ");
    let storage = open_app_storage();
    let results = storage.search_foods_fts(&query, 20).unwrap_or_default();

    println!("\n检索关键词: '{}' (共命中 {} 条食物)", query, results.len());
    println!("{:<26} {:<38} {:<10} {:<10}", "食物 ID", "食品名称", "标准份量", "热量 (kcal/100g)");
    println!("{:-<88}", "");

    for food in results {
        let nutr = storage.get_food_with_nutriments(&food.id).ok().flatten();
        let kcal_str = nutr
            .map(|(_, n)| format!("{:.0}", n.energy_kcal_100))
            .unwrap_or_else(|| "-".to_string());
        let serv_str = format!(
            "{:.0} {}",
            food.serving_quantity.unwrap_or(100.0),
            food.serving_unit.as_deref().unwrap_or("g")
        );

        let display_name = if food.name.len() > 36 {
            format!("{}...", &food.name[..33])
        } else {
            food.name.clone()
        };

        println!("{:<26} {:<38} {:<10} {:<10}", food.id, display_name, serv_str, kcal_str);
    }
    println!();
}

/// 处理 `log-food` 指令：记录、更新或删除食物摄入日志。
fn handle_log_food(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";

    // 1. 处理删除子命令/参数: --delete <ID>
    if let Some(del_id) = get_arg_val(args, "--delete") {
        match storage.delete_intake(user_id, &del_id) {
            Ok(true) => println!("\n已成功删除摄入打卡记录 ID: {}\n", del_id),
            Ok(false) => eprintln!("\n未找到指定 ID 的摄入打卡记录或无权删除: {}\n", del_id),
            Err(e) => eprintln!("\n删除摄入记录失败: {}\n", e),
        }
        return;
    }

    // 2. 处理修改子命令/参数: --update <ID> --amount <VAL> [--meal <MEAL>]
    if let Some(update_id) = get_arg_val(args, "--update") {
        let amount: f64 = get_arg_val(args, "--amount")
            .and_then(|s| s.parse().ok())
            .expect("修改摄入记录时必须提供新的摄入量: --amount <VAL>");
        let meal_type = get_arg_val(args, "--meal").map(|m| match m.to_lowercase().as_str() {
            "breakfast" => MealType::Breakfast,
            "dinner" => MealType::Dinner,
            "snack" => MealType::Snack,
            _ => MealType::Lunch,
        });

        match storage.update_intake(user_id, &update_id, amount, meal_type) {
            Ok(true) => println!("\n已成功更新摄入记录 ID: {}，新份量: {:.1}g，营养快照已自动重算！\n", update_id, amount),
            Ok(false) => eprintln!("\n未找到指定 ID 的摄入打卡记录: {}\n", update_id),
            Err(e) => eprintln!("\n更新摄入记录失败: {}\n", e),
        }
        return;
    }

    // 3. 正常打卡记录
    let food_id = match get_arg_val(args, "--id") {
        Some(id) => id,
        None => {
            println!(r#"
用法: litebalance log-food [OPTIONS]

选项:
  --id <FOOD_ID>           食物 ID (必填，用于打卡)
  --amount <AMOUNT>        摄入克数 (默认: 100.0)
  --unit <UNIT>            单位 (默认: g)
  --meal <MEAL>            餐别: breakfast | lunch | dinner | snack (默认: lunch)
  --update <INTAKE_ID>     更新指定摄入记录的克数或餐别 (配合 --amount, 可选 --meal)
  --delete <INTAKE_ID>     删除指定的食物摄入记录
"#);
            return;
        }
    };
    let amount: f64 = get_arg_val(args, "--amount").and_then(|s| s.parse().ok()).unwrap_or(100.0);
    let unit = get_arg_val(args, "--unit").unwrap_or_else(|| "g".to_string());
    let meal_str = get_arg_val(args, "--meal").unwrap_or_else(|| "lunch".to_string());

    let meal_type = match meal_str.to_lowercase().as_str() {
        "breakfast" => MealType::Breakfast,
        "dinner" => MealType::Dinner,
        "snack" => MealType::Snack,
        _ => MealType::Lunch,
    };

    // 默认本地主用户
    let dummy_profile = UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::Active).unwrap();
    let _ = storage.insert_user(user_id, "User", "1994-01-01", &dummy_profile);

    let now_str = chrono::Utc::now().to_rfc3339();
    let log = storage
        .log_intake(user_id, &food_id, amount, &unit, meal_type, &now_str)
        .expect("记录饮食日志失败");

    println!("\n成功记录餐食摄入日志!");
    println!("食物项: {} ({:.1} {})", log.food_name, log.amount, log.unit);
    println!("餐别: {:?}", log.meal_type);
    println!(
        "摄入快照: {:.0} kcal | 蛋白质: {:.1}g | 碳水: {:.1}g | 脂肪: {:.1}g\n",
        log.snapshot_energy_kcal, log.snapshot_protein_g, log.snapshot_carbs_g, log.snapshot_fat_g
    );
}

/// 处理 `summary` 指令：查看今日或指定日期的摄入营养素及三大宏量占比（支持 --boundary 生理日界线）。
fn handle_summary(args: &[String]) {
    let date_str = get_arg_val(args, "--date")
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let boundary_offset = get_arg_val(args, "--boundary")
        .map(|b| DayBoundaryConfig::from_str_loose(&b).offset_total_minutes)
        .unwrap_or(0);

    let storage = open_app_storage();
    let summary = storage
        .get_daily_summary_with_boundary("default_user", &date_str, boundary_offset)
        .unwrap_or_default();

    println!("\n=======================================================");
    if boundary_offset > 0 {
        let h = boundary_offset / 60;
        let m = boundary_offset % 60;
        println!("  每日营养摄入汇总 ({}) [生理日界线: {:02}:{:02}]", date_str, h, m);
    } else {
        println!("  每日营养摄入汇总 ({})", date_str);
    }
    println!("=======================================================");
    println!("打卡记录项数:     {} 项", summary.items_count);
    println!("实际摄入总能量:   {:.0} kcal", summary.total_energy_kcal);
    println!("三大宏量营养素摄入:");
    println!("  * 蛋白质:       {:.1} g ({:.0} kcal)", summary.total_protein_g, summary.total_protein_g * 4.0);
    println!("  * 碳水化合物:   {:.1} g ({:.0} kcal)", summary.total_carbs_g, summary.total_carbs_g * 4.0);
    println!("  * 脂肪:         {:.1} g ({:.0} kcal)", summary.total_fat_g, summary.total_fat_g * 9.0);

    let total_macro_kcal = (summary.total_protein_g * 4.0)
        + (summary.total_carbs_g * 4.0)
        + (summary.total_fat_g * 9.0);
    if total_macro_kcal > 0.0 {
        println!("宏量供能比 (Macro Ratio):");
        println!(
            "  * 蛋白质: {:.1}% | 碳水化合物: {:.1}% | 脂肪: {:.1}%",
            summary.total_protein_g * 4.0 / total_macro_kcal * 100.0,
            summary.total_carbs_g * 4.0 / total_macro_kcal * 100.0,
            summary.total_fat_g * 9.0 / total_macro_kcal * 100.0
        );
    }
    println!();
}

/// 处理 `water` 指令：打卡记录饮水量、删除记录与查看今日水合状态。
fn handle_water(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";
    let dummy_profile = UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::Active).unwrap();
    let _ = storage.insert_user(user_id, "User", "1994-01-01", &dummy_profile);

    if let Some(del_id) = get_arg_val(args, "--delete") {
        match storage.delete_water_log(user_id, &del_id) {
            Ok(true) => println!("\n已成功删除饮水打卡记录 ID: {}\n", del_id),
            Ok(false) => eprintln!("\n未找到指定 ID 的饮水打卡记录: {}\n", del_id),
            Err(e) => eprintln!("\n删除饮水记录失败: {}\n", e),
        }
        return;
    }

    if let Some(add_str) = get_arg_val(args, "--add") {
        let amount_ml: u32 = add_str.parse().expect("无效的饮水量 (ml)");
        let now_str = chrono::Utc::now().to_rfc3339();
        storage.log_water(user_id, amount_ml, &now_str).expect("记录饮水失败");
        println!("\n已记录饮水 +{} ml。", amount_ml);
    }

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let daily_water = storage.get_daily_water(user_id, &today, 2500).unwrap();

    println!("\n=======================================================");
    println!("  每日水合与饮水达标状态 ({})", today);
    println!("=======================================================");
    println!(
        "已饮用:   {} ml / 目标 {} ml ({:.1}%)",
        daily_water.total_consumed_ml, daily_water.goal_ml, daily_water.progress_pct
    );
    println!("剩余需喝: {} ml", daily_water.remaining_ml);
    if daily_water.is_goal_reached {
        println!("当前状态: [已达成目标] 水合充足！");
    } else {
        println!("当前状态: [进行中] 保持规律补充水分。");
    }
    println!();
}

/// 处理 `fast` 指令：管理间歇性断食会话（开启、查询、完成、取消、删除）。
fn handle_fast(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";
    let dummy_profile = UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::Active).unwrap();
    let _ = storage.insert_user(user_id, "User", "1994-01-01", &dummy_profile);

    if let Some(del_id) = get_arg_val(args, "--delete") {
        match storage.delete_fasting_session(user_id, &del_id) {
            Ok(true) => println!("\n已成功删除断食会话 ID: {}\n", del_id),
            Ok(false) => eprintln!("\n未找到指定 ID 的断食会话: {}\n", del_id),
            Err(e) => eprintln!("\n删除断食会话失败: {}\n", e),
        }
        return;
    }

    if let Some(proto_str) = get_arg_val(args, "--start") {
        let protocol = match proto_str.as_str() {
            "18:6" => FastingProtocol::F18_6,
            "20:4" => FastingProtocol::F20_4,
            "14:10" => FastingProtocol::Circadian14_10,
            _ => FastingProtocol::F16_8,
        };

        let now = chrono::Utc::now();
        let target_mins = protocol.target_duration_minutes();
        let session_id = storage.start_fasting(user_id, &now.to_rfc3339(), target_mins).unwrap();
        println!(
            "\n已成功开启 {} 断食计划 (会话 ID: {})，起始时间: {}",
            protocol.name(),
            session_id,
            now.format("%H:%M:%S UTC")
        );
        return;
    }

    if args.iter().any(|a| a == "--complete") {
        if let Ok(Some((id, _, _))) = storage.get_active_fasting(user_id) {
            let now_str = chrono::Utc::now().to_rfc3339();
            storage.complete_fasting(&id, &now_str).unwrap();
            println!("\n已成功打卡完成断食会话 {}！", id);
            return;
        } else {
            println!("\n当前没有处于进行中的断食会话。");
            return;
        }
    }

    if args.iter().any(|a| a == "--cancel") {
        if let Ok(Some((id, _, _))) = storage.get_active_fasting(user_id) {
            let now_str = chrono::Utc::now().to_rfc3339();
            storage.cancel_fasting(&id, &now_str).unwrap();
            println!("\n已取消断食会话 {}。", id);
            return;
        } else {
            println!("\n当前没有处于进行中的断食会话。");
            return;
        }
    }

    // 默认展示当前断食状态
    match storage.get_active_fasting(user_id) {
        Ok(Some((id, started_at, target_mins))) => {
            let now = chrono::Utc::now();
            let elapsed_mins = (now - started_at).num_minutes().max(0) as u32;
            let session = FastingSession {
                id: id.clone(),
                user_id: user_id.to_string(),
                started_at,
                target_duration_minutes: target_mins,
                completed_at: None,
                cancelled_at: None,
            };
            let state = session.state_at(now);

            println!("\n=======================================================");
            println!("  进行中的断食会话");
            println!("=======================================================");
            println!("会话 ID:   {}", id);
            println!("开始时间:  {}", started_at.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("目标时长:  {} 小时 ({} 分钟)", target_mins / 60, target_mins);
            println!(
                "已过时长:  {} 分钟 ({:.1}%)",
                elapsed_mins,
                (elapsed_mins as f64 / target_mins as f64 * 100.0).min(100.0)
            );
            match state {
                FastingState::Fasting { remaining_minutes, progress_pct, .. } => {
                    println!("当前状态:  断食进行中 (已完成 {:.1}%, 剩余 {} 分钟)", progress_pct, remaining_minutes);
                }
                FastingState::Overtime { overtime_minutes, .. } => {
                    println!("当前状态:  [目标已达成] 已超时断食 {} 分钟", overtime_minutes);
                }
                _ => {}
            }
            println!();
        }
        Ok(None) => {
            println!("\n当前无活跃的断食计时器。可使用指令开启: litebalance fast --start 16:8\n");
        }
        Err(e) => eprintln!("查询断食状态异常: {}", e),
    }
}

/// 处理 `trends` 指令：体重最小二乘回归投影与营养移动均值分析。
fn handle_trends(_args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";

    let history = storage.get_weight_history(user_id, 30).unwrap_or_default();
    println!("\n=======================================================");
    println!("  历史趋势与代谢投影分析");
    println!("=======================================================");

    if history.len() >= 2 {
        let proj = weight_projection(&history, Some(75.0));
        if let Some(p) = proj {
            println!("体重变化速率: {:+.2} kg / 周", p.rate_per_week_kg);
            if let Some(weeks) = p.weeks_to_target {
                println!("预计达到目标体重 (75.0 kg) 尚需: {} 周", weeks);
            }
        }
    } else {
        println!("体重趋势分析: 至少需要 2 条记录以建立最小二乘回归轨迹。");
    }

    let intake_history = storage.get_daily_intakes_history(user_id, 30).unwrap_or_default();
    if let Some(ma) = NutritionMovingAverage::compute(&intake_history) {
        println!("\n过去 30 天营养摄入移动均值 (基于 {} 天记录数据):", ma.days_count);
        println!("  * 平均每日热量: {:.0} kcal / 天", ma.avg_daily_calories);
        println!("  * 平均蛋白质:   {:.1} g (供能比 {:.1}%)", ma.avg_daily_protein_g, ma.protein_kcal_pct);
        println!("  * 平均碳水:     {:.1} g (供能比 {:.1}%)", ma.avg_daily_carbs_g, ma.carbs_kcal_pct);
        println!("  * 平均脂肪:     {:.1} g (供能比 {:.1}%)", ma.avg_daily_fat_g, ma.fat_kcal_pct);
    } else {
        println!("摄入趋势分析: 未检索到有效历史饮食打卡记录。");
    }
    println!();
}

/// 处理 `seed` 指令：重新载入离线临床食物基准库。
fn handle_seed() {
    let storage = open_app_storage();
    match seed_default_foods_if_empty(&storage) {
        Ok(count) => {
            println!("离线食物数据库初始化完成，已写入 {} 条临床参考食物。", count);
        }
        Err(e) => {
            eprintln!("初始化食物数据库失败: {}", e);
        }
    }
}

/// 处理 `log-weight` 指令：记录体重与体成分，或删除指定体重打卡记录。
fn handle_log_weight(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";

    // 处理删除: log-weight --delete <ID>
    if let Some(del_id) = get_arg_val(args, "--delete") {
        match storage.delete_weight_log(user_id, &del_id) {
            Ok(true) => println!("\n已成功删除体重打卡记录 ID: {}\n", del_id),
            Ok(false) => eprintln!("\n未找到指定 ID 的体重打卡记录: {}\n", del_id),
            Err(e) => eprintln!("\n删除体重记录失败: {}\n", e),
        }
        return;
    }

    let weight_kg: f64 = get_arg_val(args, "--kg")
        .and_then(|s| s.parse().ok())
        .expect("缺少必填参数 --kg <weight>。示例: litebalance log-weight --kg 79.5");
    let body_fat: Option<f64> = get_arg_val(args, "--fat").and_then(|s| s.parse().ok());
    let date_str = get_arg_val(args, "--date")
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string());

    let dummy_profile = UserProfile::new(30, 175.0, weight_kg, Gender::Male, ActivityLevel::Active).unwrap();
    let _ = storage.insert_user(user_id, "User", "1994-01-01", &dummy_profile);

    let log_id = storage.log_weight(user_id, weight_kg, body_fat, &date_str, None).expect("记录体重失败");
    println!(
        "\n成功记录体重: {:.1} kg (体脂率: {:?})，记录时间: {}",
        weight_kg, body_fat, date_str
    );
    println!("记录 ID: {}  (可使用 --delete <ID> 删除此记录)\n", log_id);
}

/// 处理 `recipe` 指令：查看用户自建多原料食谱列表。
fn handle_recipe(_args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";

    let recipes = storage.list_user_recipes(user_id).unwrap_or_default();
    println!("\n=======================================================");
    println!("  用户自定义食谱库 (共 {} 份食谱)", recipes.len());
    println!("=======================================================");
    if recipes.is_empty() {
        println!("本地数据库中暂无自建食谱。");
    } else {
        println!("{:<24} {:<32} {:<10} {:<12}", "食谱 ID", "食谱名称", "份数", "总重量 (g)");
        println!("{:-<80}", "");
        for r in recipes {
            println!("{:<24} {:<32} {:<10.1} {:.0} g", r.id, r.name, r.servings, r.total_weight_g);
        }
    }
    println!();
}

/// 处理 `dri` 指令：执行 NASEM 微量元素达标雷达评估与防虚假报警分析。
fn handle_dri(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let date = get_arg_val(args, "--date").unwrap_or(today);
    let age: u32 = get_arg_val(args, "--age").and_then(|s| s.parse().ok()).unwrap_or(30);
    let gender_str = get_arg_val(args, "--gender").unwrap_or_else(|| "male".into());
    let gender = if gender_str.to_lowercase().starts_with('f') {
        Gender::Female
    } else {
        Gender::Male
    };

    let consumed_nutriments = storage.get_daily_intake_nutriments(user_id, &date).unwrap_or_default();
    let report = MicronutrientEvaluator::evaluate_daily_intake(&date, gender, age, &consumed_nutriments);

    println!("\n==========================================================================================");
    println!("  临床微量元素 DRI 雷达与数据完整度评估: {}", report.date);
    println!("  参考标准: NASEM Dietary Reference Intakes (RDA 推荐摄入 / AI 适宜摄入 / UL 上限)");
    println!("==========================================================================================");
    println!("已记录食物摄入总净重: {:.1} g", report.total_food_mass_g);
    println!("整体微量营养充足度评分: {:.1}%\n", report.adequacy_score);

    println!("{:<24} {:<10} {:<12} {:<10} {:<12} {:<16}", "营养素", "摄入量", "目标值", "标准类型", "数据覆盖率", "临床判定状态");
    println!("{:-<92}", "");

    for a in &report.assessments {
        let status_str = match &a.status {
            DriStatus::LowDataCoverage { coverage_pct } => format!("数据不足 ({:.0}%)", coverage_pct),
            DriStatus::Deficient { percent_of_target } => format!("摄入不足 ({:.0}%)", percent_of_target),
            DriStatus::Adequate { percent_of_target } => format!("达标充足 ({:.0}%)", percent_of_target),
            DriStatus::Optimal { percent_of_target } => format!("最佳状态 ({:.0}%)", percent_of_target),
            DriStatus::Excessive { upper_limit, .. } => format!("超标过量 (> {})", upper_limit),
        };
        let std_str = match a.standard {
            litebalance_core::calc::DriStandard::Rda => "RDA",
            litebalance_core::calc::DriStandard::Ai => "AI",
        };
        println!(
            "{:<24} {:<10.1} {:<12.1} {:<10} {:<12.0}% {:<16}",
            a.nutrient_name,
            a.intake_amount,
            a.target_value,
            std_str,
            a.data_coverage_pct,
            status_str
        );
    }

    if !report.warnings.is_empty() {
        println!("\n临床预警与摄入不足拦截:");
        for w in &report.warnings {
            println!("  [!] {}", w);
        }
    }
    println!();
}

/// 处理 `report` 指令：生成周期性代谢偏离度与行为依从性周报/月报。
fn handle_report(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";
    let days: usize = get_arg_val(args, "--days").and_then(|s| s.parse().ok()).unwrap_or(7);
    let tdee: f64 = get_arg_val(args, "--tdee").and_then(|s| s.parse().ok()).unwrap_or(2400.0);
    let target_kcal: f64 = get_arg_val(args, "--target").and_then(|s| s.parse().ok()).unwrap_or(1900.0);
    let target_protein: f64 = get_arg_val(args, "--protein").and_then(|s| s.parse().ok()).unwrap_or(150.0);
    let target_water: u32 = get_arg_val(args, "--water").and_then(|s| s.parse().ok()).unwrap_or(2500);

    let end_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let start_date = (chrono::Utc::now() - chrono::Duration::days(days as i64 - 1)).format("%Y-%m-%d").to_string();

    let intakes_raw = storage.get_daily_intakes_history(user_id, days).unwrap_or_default();
    let mut daily_intakes = Vec::new();
    for (i, item) in intakes_raw.iter().enumerate() {
        daily_intakes.push(DailyIntakeData {
            date: format!("Day-{}", i + 1),
            intake_kcal: item.0,
            protein_g: item.1,
            carbs_g: item.2,
            fat_g: item.3,
        });
    }

    let weights_raw = storage.get_weight_history(user_id, days).unwrap_or_default();
    let mut weight_logs = Vec::new();
    for (i, w) in weights_raw.iter().enumerate() {
        weight_logs.push(DailyWeightData {
            day_offset: i as f64,
            weight_kg: w.1,
        });
    }

    let report = PeriodicAnalyticsEngine::generate_report(
        &start_date,
        &end_date,
        days,
        tdee,
        target_kcal,
        target_protein,
        target_water,
        &daily_intakes,
        &weight_logs,
        &[],
        &[],
    );

    println!("\n=============================================================================");
    println!(
        "  周期性代谢动力学与依从性分析报告: {} -> {} ({} 天周期)",
        report.start_date, report.end_date, report.period_days
    );
    println!("=============================================================================");
    println!("1. 能量平衡与真实体重轨迹");
    println!(
        "   打卡依从率:                 {}/{} 天 ({:.1}%)",
        report.energy.logged_days, report.energy.total_days, report.energy.logging_adherence_pct
    );
    println!(
        "   平均每日摄入:               {:.0} kcal (目标: {:.0} kcal, TDEE: {:.0} kcal)",
        report.energy.avg_daily_intake_kcal, report.energy.avg_daily_target_kcal, report.energy.avg_daily_tdee_kcal
    );
    println!("   累计热量缺口:               {:+.0} kcal", report.energy.cumulative_caloric_deficit);
    println!("   理论预计体重变化:           {:+.2} kg", report.energy.theoretical_weight_change_kg);
    if let Some(actual) = report.energy.actual_weight_change_kg {
        println!("   秤端实际体重变化:           {:+.2} kg", actual);
        if let Some(div) = report.energy.metabolic_divergence_kg {
            println!("   代谢偏离度 (Divergence):    {:+.2} kg", div);
        }
    } else {
        println!("   秤端实际体重变化:           打卡数据不足 (需至少 2 条体重记录)");
    }

    println!("\n2. 宏量营养素摄入分布");
    println!(
        "   蛋白质: {:.1} g/天 (供能比 {:.1}%) [目标: {:.0} g]",
        report.macros.avg_daily_protein_g, report.macros.actual_protein_energy_pct, report.macros.target_protein_g
    );
    println!(
        "   碳水:   {:.1} g/天 (供能比 {:.1}%)",
        report.macros.avg_daily_carbs_g, report.macros.actual_carbs_energy_pct
    );
    println!(
        "   脂肪:   {:.1} g/天 (供能比 {:.1}%)",
        report.macros.avg_daily_fat_g, report.macros.actual_fat_energy_pct
    );
    println!(
        "   蛋白质达标率:               {}/{} 天 ({:.1}%)",
        report.macros.protein_compliance_days, report.energy.logged_days, report.macros.protein_compliance_rate_pct
    );

    println!("\n3. 临床见解与行为指导");
    for ins in &report.insights {
        println!("   * {}", ins);
    }
    println!();
}

/// 处理 `export` 指令：导出全量原生备份或标准 CSV。
fn handle_export(args: &[String]) {
    let storage = open_app_storage();
    let user_id = "default_user";
    let format = get_arg_val(args, "--format").unwrap_or_else(|| "json".into());
    let output = get_arg_val(args, "--output");

    match format.to_lowercase().as_str() {
        "csv" => {
            let csv_str = ExportImportEngine::export_intakes_csv(&storage, user_id)
                .expect("导出饮食日志为 CSV 失败");
            let file_path = output.unwrap_or_else(|| "user_intake.csv".into());
            std::fs::write(&file_path, csv_str).expect("写入 CSV 文件失败");
            println!("\n已成功导出饮食日志至 CSV: {}", file_path);
        }
        _ => {
            let backup = ExportImportEngine::export_native_backup(&storage, user_id)
                .expect("生成原生备份失败");
            let json_str = serde_json::to_string_pretty(&backup).expect("序列化备份数据失败");
            let file_path = output.unwrap_or_else(|| "litebalance_backup.json".into());
            std::fs::write(&file_path, json_str).expect("写入 JSON 备份文件失败");
            println!("\n已成功导出全量数据库原生备份至 JSON: {}", file_path);
            println!("  食品数据:  {} 项", backup.foods.len());
            println!("  饮食打卡:  {} 条", backup.intakes.len());
            println!("  体重记录:  {} 条", backup.weights.len());
            println!("  饮水记录:  {} 条", backup.waters.len());
            println!("  运动打卡:  {} 条", backup.activities.len());
            println!("  自建食谱:  {} 份", backup.recipes.len());
        }
    }
    println!();
}

/// 处理 `import` 指令：从历史 JSON/CSV 或原生备份还原数据。
fn handle_import(args: &[String]) {
    if args.is_empty() {
        eprintln!("用法: litebalance import <backup.json>");
        eprintln!("仅支持原生备份 JSON（由 export 生成）。");
        return;
    }
    let path = &args[0];
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("读取失败: {}", e));
    let storage = open_app_storage();
    let backup: NutriTrackerBackup =
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("解析原生备份失败: {}", e));
    let stats =
        ExportImportEngine::import_native_backup(&storage, &backup).expect("原生数据还原失败");
    println!(
        "导入完成: foods={}, intakes={}, weights={}, waters={}, errors={}",
        stats.foods_imported,
        stats.intakes_imported,
        stats.weights_imported,
        stats.waters_imported,
        stats.errors.len()
    );
    for e in stats.errors {
        eprintln!("警告: {}", e);
    }
}

/// 处理 `barcode` 指令：GS1 模 10 校验、OpenFoodFacts API 检索、零污染微量解析与本地网络缓存。
fn handle_barcode(args: &[String]) {
    if args.is_empty() {
        eprintln!("错误: 缺少条形码参数。用法: litebalance barcode <BARCODE> [--save] [--ua <custom_ua>]");
        eprintln!("示例: litebalance barcode 737628064502");
        return;
    }

    let raw_code = if args[0].starts_with("--") {
        get_arg_val(args, "--code").unwrap_or_default()
    } else {
        args[0].clone()
    };

    if raw_code.is_empty() {
        eprintln!("错误: 缺少有效的条形码。示例: litebalance barcode 737628064502");
        return;
    }

    let save_to_main = args.iter().any(|a| a == "--save");
    let custom_ua = get_arg_val(args, "--ua");

    println!("\n=======================================================");
    println!("  条形码识别与外部食品数据库 (OpenFoodFacts) 检索");
    println!("=======================================================");

    // 1. GS1 模 10 校验与条码类型识别
    let normalized = match BarcodeValidator::parse_and_normalize(&raw_code) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("\n[错误] 条形码 GS1 校验失败: {}", e);
            eprintln!("支持格式: EAN-13 (13位)、UPC-A (12位，自动前置补零规范化)、EAN-8 (8位)");
            return;
        }
    };

    println!("* 原始输入条码:    {}", normalized.raw);
    println!("* 规范化标准条码:  {}", normalized.standard_code);
    println!("* 条码标准类型:    {:?}", normalized.barcode_type);
    println!(
        "* GS1 校验状态:    通过 (模 10 校验位: {})",
        normalized.standard_code.chars().last().unwrap_or('?')
    );

    // 2. 初始化独立网络缓存库
    let cache_storage = open_cache_storage();

    // 3. 构建客户端并执行查询
    let client = match RemoteFoodClient::new(custom_ua.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("\n[错误] 初始化远程网络客户端失败: {}", e);
            return;
        }
    };

    println!("* 客户端 User-Agent: {}", client.user_agent());
    println!("* 安全速率限制:    最大 14 次/分钟 (滑动窗口护栏)");
    println!("\n正在检索食品档案 (优先读取本地独立离线缓存库)...");

    match client.fetch_by_barcode(&normalized.standard_code, Some(&cache_storage)) {
        Ok(product) => {
            let hit_source = if product.from_cache {
                "[本地离线缓存命中 (food_cache.db)]"
            } else {
                "[OpenFoodFacts 远程 API 实时拉取 (已自动写入本地缓存)]"
            };

            println!("\n=======================================================");
            println!("  食品检索成功 {}", hit_source);
            println!("=======================================================");
            println!("* 商品名称:  {}", product.food_name);
            println!("* 生产厂商:  {}", product.brand.as_deref().unwrap_or("未标明"));
            println!(
                "* 标准份量:  {:.1} {}",
                product.serving_quantity.unwrap_or(100.0),
                product.serving_unit.as_deref().unwrap_or("g")
            );
            println!("* 数据版权:  {}", product.attribution);

            let n = &product.nutriments;
            println!("\n--- 每 100g 临床级核心宏量与微量元素明细 ---");
            println!("  * 热量:         {:.1} kcal", n.energy_kcal_100);
            println!("  * 蛋白质:       {:.1} g", n.proteins_100);
            println!(
                "  * 脂肪:         {:.1} g (饱和脂肪: {})",
                n.fat_100,
                n.saturated_fat_100.map(|v| format!("{:.1} g", v)).unwrap_or_else(|| "未标注 (保持未污染)".to_string())
            );
            println!(
                "  * 碳水化合物:   {:.1} g (糖分: {}, 膳食纤维: {})",
                n.carbohydrates_100,
                n.sugars_100.map(|v| format!("{:.1} g", v)).unwrap_or_else(|| "未标注".to_string()),
                n.fiber_100.map(|v| format!("{:.1} g", v)).unwrap_or_else(|| "未标注".to_string())
            );
            println!(
                "  * 钠 (Sodium):  {}",
                n.sodium_mg_100.map(|v| format!("{:.1} mg", v)).unwrap_or_else(|| "未标注 (保持未污染)".to_string())
            );

            // 微量元素展示
            let format_micro = |val: Option<f64>, unit: &str| {
                val.map(|v| format!("{:.1} {}", v, unit)).unwrap_or_else(|| "未标注/未知 (保持 None，避免数据稀释)".to_string())
            };
            println!("  * 钙 (Calcium):    {}", format_micro(n.calcium_mg_100, "mg"));
            println!("  * 铁 (Iron):       {}", format_micro(n.iron_mg_100, "mg"));
            println!("  * 锌 (Zinc):       {}", format_micro(n.zinc_mg_100, "mg"));
            println!("  * 镁 (Magnesium):  {}", format_micro(n.magnesium_mg_100, "mg"));
            println!("  * 钾 (Potassium):  {}", format_micro(n.potassium_mg_100, "mg"));
            println!("  * 维生素 C:        {}", format_micro(n.vitamin_c_mg_100, "mg"));
            println!("  * 维生素 D:        {}", format_micro(n.vitamin_d_ug_100, "ug"));

            // 若带 --save 参数，持久化存入用户主库
            if save_to_main {
                let main_storage = open_app_storage();
                let food_record = FoodRecord {
                    id: product.barcode.standard_code.clone(),
                    name: product.food_name.clone(),
                    brand: product.brand.clone(),
                    source: FoodSource::OpenFoodFacts,
                    serving_quantity: product.serving_quantity,
                    serving_unit: product.serving_unit.clone(),
                    image_url: None,
                };
                match main_storage.insert_food(&food_record, &product.nutriments) {
                    Ok(_) => {
                        println!("\n[已保存] 该食品已成功保存至本地用户主库 (litebalance.db)！");
                        println!("  可使用 'search {}' 进行离线全文检索，或使用 'log-food --id {}' 直接打卡记录。", product.food_name, product.barcode.standard_code);
                    }
                    Err(e) => {
                        eprintln!("\n[警告] 保存至用户主库失败: {}", e);
                    }
                }
            } else {
                println!("\n提示: 可添加 '--save' 选项将此商品直接收录至本地主库离线持久化存储。");
            }
            println!();
        }
        Err(e) => {
            eprintln!("\n[错误] 食品查询失败: {}", e);
        }
    }
}

/// 处理 `cache` 指令：管理独立外部网络缓存数据库 (`food_cache.db`)。
fn handle_cache(args: &[String]) {
    let cache_storage = open_cache_storage();
    let db_path = get_cache_db_path();

    let is_clear = args.iter().any(|a| a == "--clear");
    let is_evict = args.iter().any(|a| a == "--evict");

    if is_clear {
        match cache_storage.clear_all() {
            Ok(count) => {
                println!("\n已成功清空外部食品网络缓存数据库！");
                println!("  清理条目数:  {} 条", count);
                println!("  缓存文件:    {}", db_path.display());
                println!("  说明:        已彻底清除网络暂存数据。用户核心数据库 (litebalance.db) 完好无损。\n");
            }
            Err(e) => {
                eprintln!("\n清空缓存失败: {}", e);
            }
        }
        return;
    }

    if is_evict {
        match cache_storage.evict_expired() {
            Ok(count) => {
                println!("\n已成功执行过期缓存驱逐清理！");
                println!("  驱逐过期条目:  {} 条", count);
                println!("  缓存文件:      {}\n", db_path.display());
            }
            Err(e) => {
                eprintln!("\n驱逐过期缓存失败: {}", e);
            }
        }
        return;
    }

    // 默认展示统计指标
    match cache_storage.get_cache_stats() {
        Ok(stats) => {
            println!("\n=======================================================");
            println!("  外部食品网络缓存数据库状态 (food_cache.db)");
            println!("=======================================================");
            println!("* 缓存存储文件:    {}", db_path.display());
            println!("* 架构隔离级别:    物理独立分库 (零 ODbL 协议传染 / 不随原生备份膨胀)");
            println!("* 缓存默认有效时长: 30 天 (2,592,000 秒)");
            println!("* 缓存记录总数:    {} 条", stats.total_entries);
            println!("* 处于有效 TTL:    {} 条", stats.active_entries);
            println!("* 已过期等待驱逐:  {} 条", stats.expired_entries);
            println!("\n可用维护指令:");
            println!("  litebalance cache --evict    驱逐清理已过期的网络缓存记录");
            println!("  litebalance cache --clear    全量清空所有网络缓存 (不影响用户私有数据)\n");
        }
        Err(e) => {
            eprintln!("\n获取缓存指标失败: {}", e);
        }
    }
}

/// 获取或创建默认用户的生理档案（用于代谢能耗与代偿基准计算）。
fn get_or_create_default_profile(storage: &StorageEngine) -> UserProfile {
    let user_id = "default_user";
    if let Ok(Some(u)) = storage.get_user(user_id) {
        let gender = match u.gender.to_lowercase().as_str() {
            "female" | "f" => Gender::Female,
            _ => Gender::Male,
        };
        let act = match u.activity_level.to_lowercase().as_str() {
            "inactive" | "sedentary" => ActivityLevel::Inactive,
            "low" | "lowactive" => ActivityLevel::LowActive,
            "very" | "veryactive" => ActivityLevel::VeryActive,
            _ => ActivityLevel::Active,
        };
        UserProfile::new(30, u.height_cm, u.weight_kg, gender, act).unwrap_or_else(|_| {
            UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::Active).unwrap()
        })
    } else {
        let profile = UserProfile::new(30, 175.0, 75.0, Gender::Male, ActivityLevel::Active).unwrap();
        let _ = storage.insert_user(user_id, "User", "1994-01-01", &profile);
        profile
    }
}

/// 处理 `activity` 指令：运动与体力活动打卡 (log / list / catalog / delete)。
fn handle_activity(args: &[String]) {
    let sub = if args.is_empty() {
        "list"
    } else {
        args[0].as_str()
    };

    let storage = open_app_storage();
    let user_id = "default_user";

    match sub {
        "catalog" => {
            let cat_filter = get_arg_val(args, "--category");
            let catalog = get_standard_activity_catalog();

            println!("\n=========================================================================================");
            println!("  临床标准 MET 运动知识库 (Herrmann et al. 2024 / Compendium of Physical Activities)");
            println!("=========================================================================================");
            println!("{:<28} {:<24} {:<8} {:<18} 场景描述", "运动编码 (CODE)", "中文名称", "MET", "运动模态");
            println!("{:-<105}", "");

            for item in catalog {
                if let Some(cat) = &cat_filter
                    && !item.category.as_str().eq_ignore_ascii_case(cat)
                {
                    continue;
                }
                let modality_str = match item.modality {
                    ExerciseModality::ResistanceTraining => "抗阻力量 (代偿极低)",
                    ExerciseModality::Aerobic => "有氧耐力 (代偿约70%)",
                    ExerciseModality::HybridHiit => "间歇综合 (代偿中等)",
                    ExerciseModality::LowIntensityActive => "低强度日常 (NEAT)",
                };
                println!(
                    "{:<28} {:<24} {:<8.1} {:<18} {}",
                    item.code, item.name_zh, item.met_value, modality_str, item.description
                );
            }
            println!("\n提示: 可使用 'litebalance activity log --code <CODE> --duration <分钟>' 快速打卡。\n");
        }
        "log" => {
            let code = get_arg_val(args, "--code").or_else(|| {
                if args.len() > 1 && !args[1].starts_with("--") {
                    Some(args[1].clone())
                } else {
                    None
                }
            }).expect("缺少必填参数 --code <CODE>。示例: litebalance activity log --code running_10kph --duration 45");

            let duration: f64 = get_arg_val(args, "--duration")
                .and_then(|s| s.parse().ok())
                .unwrap_or(30.0);

            let date = get_arg_val(args, "--date")
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

            let profile = get_or_create_default_profile(&storage);

            // 查找运动标准条目
            let (name, category, met, modality) = if let Some(item) = find_activity_by_code(&code) {
                (item.name_zh.to_string(), item.category.as_str().to_string(), item.met_value, item.modality)
            } else {
                let custom_met: f64 = get_arg_val(args, "--met").and_then(|s| s.parse().ok()).unwrap_or(5.0);
                (format!("自定义运动 ({})", code), "custom".to_string(), custom_met, ExerciseModality::HybridHiit)
            };

            let state = match get_arg_val(args, "--state").as_deref() {
                Some("deficit") => NutritionState::CaloricDeficit,
                Some("surplus") => NutritionState::CaloricSurplus,
                _ => NutritionState::Maintenance,
            };

            // 结合 Pontzer-Trexler 2026 代偿模型计算
            let comp_res = ActivityEnergyCalculator::calculate_with_compensation(
                met,
                modality,
                profile.weight_kg,
                duration,
                &profile,
                None,
                state,
            );

            let record = ActivityLogRecord {
                id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.into(),
                activity_code: code.clone(),
                activity_name: name.clone(),
                category,
                duration_minutes: duration,
                met_value: met,
                gross_burned_kcal: comp_res.reported_kcal,
                net_credited_kcal: comp_res.credited_kcal,
                compensation_ratio: comp_res.effective_multiplier,
                date: date.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                external_id: None,
            };

            storage.log_activity(&record).expect("记录运动日志失败");

            println!("\n=======================================================");
            println!("  运动打卡成功 (Pontzer 2026 代偿结算完成)");
            println!("=======================================================");
            println!("* 记录日期:      {}", date);
            println!("* 运动项目:      {} ({})", name, code);
            println!("* 持续时间:      {:.0} 分钟 (标准 MET: {:.1})", duration, met);
            println!("* 名义总能量消耗: {:.1} kcal (未经折算的表面能耗)", comp_res.reported_kcal);
            println!(
                "* 科学净计入消耗: {:.1} kcal (实际计入每日摄入预算，折算率 {:.0}%)",
                comp_res.credited_kcal, comp_res.effective_multiplier * 100.0
            );
            println!(
                "* 代偿抵消挤压:   {:.1} kcal (被基础代谢与自发性活动吸收抵消)",
                comp_res.compensated_kcal
            );
            println!("* 生理代偿依据:   {}", comp_res.scientific_rationale);
            println!("\n提示: 运行 'litebalance balance --date {}' 查看今日净热量赤字/盈余闭环。\n", date);
        }
        "delete" => {
            let id = get_arg_val(args, "--id").or_else(|| {
                if args.len() > 1 && !args[1].starts_with("--") {
                    Some(args[1].clone())
                } else {
                    None
                }
            }).expect("缺少参数 --id <ID>");

            match storage.delete_activity(user_id, &id) {
                Ok(true) => println!("\n已成功删除运动打卡记录 ID: {}\n", id),
                Ok(false) => eprintln!("\n未找到指定 ID 的运动打卡记录: {}\n", id),
                Err(e) => eprintln!("\n删除运动记录失败: {}\n", e),
            }
        }
        _ => {
            let date = get_arg_val(args, "--date")
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

            let activities = storage.get_activities_for_date(user_id, &date).unwrap_or_default();

            println!("\n=========================================================================================");
            println!("  当日运动与体力活动记录列表 ({})", date);
            println!("=========================================================================================");

            if activities.is_empty() {
                println!("今日暂无运动打卡记录。使用 'activity log --code <CODE> --duration <MIN>' 添加运动。");
            } else {
                println!("{:<28} {:<10} {:<14} {:<14} {:<10}", "运动名称", "时长(分)", "名义消耗(kcal)", "净计入(kcal)", "代偿率");
                println!("{:-<80}", "");

                let mut total_gross = 0.0;
                let mut total_net = 0.0;

                for act in &activities {
                    total_gross += act.gross_burned_kcal;
                    total_net += act.net_credited_kcal;
                    println!(
                        "{:<28} {:<10.0} {:<14.1} {:<14.1} {:.0}%",
                        act.activity_name,
                        act.duration_minutes,
                        act.gross_burned_kcal,
                        act.net_credited_kcal,
                        act.compensation_ratio * 100.0
                    );
                }

                println!("{:-<80}", "");
                println!(
                    "当日合计: 名义消耗 {:.1} kcal | 净计入 {:.1} kcal | 代偿抵消扣除 {:.1} kcal",
                    total_gross, total_net, (total_gross - total_net).max(0.0)
                );
            }
            println!();
        }
    }
}

/// 处理 `balance` 指令：每日全量能量闭环仪表盘（摄入 - 基础维持 - 运动名义 + 代偿 = 调整后 TDEE 与净热量差，支持 --boundary 生理日界线）。
fn handle_balance(args: &[String]) {
    let date = get_arg_val(args, "--date")
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let boundary_offset = get_arg_val(args, "--boundary")
        .map(|b| DayBoundaryConfig::from_str_loose(&b).offset_total_minutes)
        .unwrap_or(0);

    let storage = open_app_storage();
    let user_id = "default_user";

    let profile = get_or_create_default_profile(&storage);
    let base_tdee = EnergyCalc::nasem_2023(&profile).round();

    let summary = storage
        .get_daily_summary_with_boundary(user_id, &date, boundary_offset)
        .unwrap_or_default();
    let (gross_burned, net_credited) = storage.get_daily_activity_totals(user_id, &date).unwrap_or((0.0, 0.0));

    let balance = DailyEnergyBalance::compute(
        date.clone(),
        summary.total_energy_kcal,
        base_tdee,
        gross_burned,
        net_credited,
    );

    println!("\n=======================================================");
    if boundary_offset > 0 {
        let h = boundary_offset / 60;
        let m = boundary_offset % 60;
        println!("  每日全量能量闭环仪表盘 (Daily Energy Balance) [生理日界线: {:02}:{:02}]", h, m);
    } else {
        println!("  每日全量能量闭环仪表盘 (Daily Energy Balance)");
    }
    println!("=======================================================");
    println!("* 评估核算日期:  {}", balance.date);

    println!("\n--- 1. 能量摄入与基准生理代谢 ---");
    println!("  * 实际饮食摄入总热量:    {:.0} kcal (共打卡 {} 项餐食)", balance.total_intake_kcal, summary.items_count);
    println!("  * 静息基准维持消耗 (TDEE): {:.0} kcal (遵循 NASEM 2023 临床回归方程)", balance.base_tdee_kcal);

    println!("\n--- 2. 运动体力活动与 Pontzer 2026 代偿分析 ---");
    println!("  * 运动上报名义总消耗:    {:.0} kcal", balance.gross_activity_burned_kcal);
    println!("  * 代偿吸收扣减能耗:      {:.0} kcal (人体自我节律代偿，不转为可用额度)", balance.compensated_amount_kcal);
    println!("  * 科学净有效计入消耗:    {:.0} kcal (真实有效能耗增量)", balance.net_activity_credited_kcal);

    println!("\n--- 3. 闭环能量平衡结算与体成分趋势 ---");
    println!("  * 今日调整后真实总能耗:  {:.0} kcal (Base TDEE + Net Credited)", balance.adjusted_tdee_kcal);

    let status_str = if balance.net_caloric_balance_kcal < -100.0 {
        format!("[热量赤字/减脂期: {:.0} kcal]", balance.net_caloric_balance_kcal)
    } else if balance.net_caloric_balance_kcal > 100.0 {
        format!("[热量盈余/增肌期: +{:.0} kcal]", balance.net_caloric_balance_kcal)
    } else {
        "[热量维持平衡期]".to_string()
    };
    println!("  * 今日最终净能量差:      {:+0.0} kcal  {}", balance.net_caloric_balance_kcal, status_str);
    println!(
        "  * 理论体重大致趋势:      预计每周体重变化 {:+0.2} kg/周 (基于 7700 kcal 能量守恒)",
        balance.projected_weekly_weight_change_kg
    );

    println!("\n--- 4. 代谢防护诊断 ---");
    println!("  * 临床提示: {}", balance.compensation_diagnostic);

    // 若用户配置了个性化体态目标，展示动态目标预算与摄入依从度
    if let Ok(Some(goal_rec)) = storage.get_user_goal(user_id) {
        let goal_config = GoalConfig {
            kind: GoalKind::from_str(&goal_rec.kind),
            target_weight_kg: goal_rec.target_weight_kg,
            weekly_rate_kg: goal_rec.weekly_rate_kg,
            taper_enabled: goal_rec.taper_enabled,
            adaptive_enabled: goal_rec.adaptive_enabled,
            manual_calorie_offset: goal_rec.manual_calorie_offset,
        };
        let budget_res = GoalProfileEngine::compute_adaptive_budget(&profile, &goal_config, &[]);
        println!("\n--- 5. 目标动态预算与摄入依从度 ---");
        println!("  * 设定体态目标:          {} (每周速率 {:+.2} kg/周)", goal_config.kind.display_name(), goal_config.weekly_rate_kg);
        println!("  * 今日动态摄入预算:      {:.0} kcal", budget_res.daily_budget_kcal);
        let diff = balance.total_intake_kcal - budget_res.daily_budget_kcal;
        if diff > 0.0 {
            println!("  * 预算执行状态:          超标 +{:.0} kcal (今日摄入高于自适应预算)", diff);
        } else {
            println!("  * 预算执行状态:          剩余可用额度 {:.0} kcal", diff.abs());
        }
    }
    println!();
}

/// 处理 `goal` 指令：动态卡路里预算与自适应目标调节 (set / status / delete)。
fn handle_goal(args: &[String]) {
    let sub = if args.is_empty() {
        "status"
    } else {
        args[0].as_str()
    };

    let storage = open_app_storage();
    let user_id = "default_user";
    let profile = get_or_create_default_profile(&storage);

    match sub {
        "set" => {
            let kind_str = get_arg_val(args, "--type").unwrap_or_else(|| "lose".to_string());
            let kind = GoalKind::from_str(&kind_str);

            let target_weight: Option<f64> = get_arg_val(args, "--target").and_then(|s| s.parse().ok());
            let default_rate = match kind {
                GoalKind::LoseWeight => -0.5,
                GoalKind::GainWeight => 0.25,
                GoalKind::MaintainWeight => 0.0,
            };
            let weekly_rate: f64 = get_arg_val(args, "--rate")
                .and_then(|s| s.parse().ok())
                .unwrap_or(default_rate);

            // 检查标志
            let taper_enabled = !args.iter().any(|a| a == "--no-taper");
            let adaptive_enabled = !args.iter().any(|a| a == "--no-adaptive");
            let manual_offset: f64 = get_arg_val(args, "--offset")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            let now = chrono::Utc::now().to_rfc3339();
            let goal_record = UserGoalRecord {
                id: uuid::Uuid::new_v4().to_string(),
                user_id: user_id.to_string(),
                kind: kind.as_str().to_string(),
                target_weight_kg: target_weight,
                weekly_rate_kg: weekly_rate,
                taper_enabled,
                adaptive_enabled,
                manual_calorie_offset: manual_offset,
                created_at: now.clone(),
                updated_at: now,
            };

            storage.set_user_goal(&goal_record).expect("保存用户目标配置失败");

            println!("\n=======================================================");
            println!("  体态目标与动态预算设置成功！");
            println!("=======================================================");
            println!("* 目标类型:        {}", kind.display_name());
            if let Some(t) = target_weight {
                println!("* 目标体重:        {:.1} kg (当前体重: {:.1} kg)", t, profile.weight_kg);
            }
            println!("* 目标每周速率:    {:+.2} kg/周", weekly_rate);
            println!("* 平稳着陆缓冲:    {}", if taper_enabled { "开启 (距目标 5kg~1kg 平滑衰减)" } else { "关闭" });
            println!("* OLS 自适应调节:  {}", if adaptive_enabled { "开启 (带 0.5 阻尼动态闭环微调)" } else { "关闭" });
            if manual_offset.abs() > 0.1 {
                println!("* 手动微调偏移:    {:+.0} kcal/天", manual_offset);
            }
            println!("\n提示: 运行 'litebalance goal status' 查看自适应预算与生理安全评估。\n");
        }
        "delete" | "clear" => {
            match storage.delete_user_goal(user_id) {
                Ok(true) => println!("\n已成功清除用户目标配置，恢复为自然维持代谢模式。\n"),
                Ok(false) => println!("\n当前未设定任何体态目标。\n"),
                Err(e) => eprintln!("\n删除目标配置失败: {}\n", e),
            }
        }
        _ => {
            let goal_opt = storage.get_user_goal(user_id).unwrap_or(None);
            let goal_config = match goal_opt {
                Some(ref g) => GoalConfig {
                    kind: GoalKind::from_str(&g.kind),
                    target_weight_kg: g.target_weight_kg,
                    weekly_rate_kg: g.weekly_rate_kg,
                    taper_enabled: g.taper_enabled,
                    adaptive_enabled: g.adaptive_enabled,
                    manual_calorie_offset: g.manual_calorie_offset,
                },
                None => {
                    println!("\n当前尚未配置个性化体态目标（默认维持现有体重）。");
                    println!("可运行例如: litebalance goal set --type lose --target 72.0 --rate -0.5 设定减脂目标。\n");
                    GoalConfig::default()
                }
            };

            // 获取过去 30 天体重历史记录
            let weight_points = storage.get_weight_history(user_id, 30).unwrap_or_default();

            let result = GoalProfileEngine::compute_adaptive_budget(
                &profile,
                &goal_config,
                &weight_points,
            );

            println!("\n=========================================================================================");
            println!("  轻衡 (LiteBalance) 动态卡路里预算与自适应目标调节面板");
            println!("=========================================================================================");
            println!("* 用户档案:        {:.1} kg, {:.1} cm | 生理表型: {:?} | 日常活动: {:?}",
                profile.weight_kg, profile.height_cm, profile.gender, profile.activity_level);
            println!("* 基准维持能耗:    {:.0} kcal/天 (遵循 NASEM 2023 DRI 临床多项式矩阵)", result.base_tdee_kcal);

            println!("\n--- 1. 目标设定与理论缺口/盈余 ---");
            println!("* 目标类型:        {}", goal_config.kind.display_name());
            if let Some(t) = goal_config.target_weight_kg {
                println!("* 目标体重:        {:.1} kg (距离目标差额: {:+.1} kg)", t, t - profile.weight_kg);
            }
            println!("* 目标周速率:      {:+.2} kg/周 (理论调节: {:+.0} kcal/天)", goal_config.weekly_rate_kg, result.raw_rate_adjustment_kcal);
            println!("* 平稳着陆状态:    衰减系数 {:.0}% (经缓冲后理论调节: {:+.0} kcal/天)", result.taper_factor * 100.0, result.tapered_adjustment_kcal);

            println!("\n--- 2. 近期体重 OLS 回归与自适应闭环反馈 ---");
            if let Some(actual_rate) = result.actual_ols_weekly_rate_kg {
                println!("* 过去30天称重记录: 共采样 {} 组有效数据点", weight_points.len());
                println!("* 实际每周变化速率: {:+.2} kg/周 (基于 OLS 最小二乘法线性回归斜率)", actual_rate);
                if let Some(diff) = result.rate_discrepancy_kg {
                    println!("* 速率偏离差额:    {:+.2} kg/周 (实测速率 - 目标速率)", diff);
                }
                println!("* 自适应阻尼修正:  {:+.0} kcal/天 (0.5阻尼平滑，最大限制 ±250 kcal)", result.adaptive_correction_kcal);
            } else {
                println!("* 过去30天称重记录: 有效称重样本不足 2 次 (运行 'litebalance log-weight <KG>' 积累数据启动自适应)");
                println!("* 自适应阻尼修正:  0 kcal/天 (保持理论开环计算)");
            }
            if result.manual_offset_kcal.abs() > 0.1 {
                println!("* 手动配置偏移:    {:+.0} kcal/天", result.manual_offset_kcal);
            }

            println!("\n--- 3. 临床内分泌安全红线与最终推荐预算 ---");
            println!("* 生理安全摄入底线: ≥ {:.0} kcal/天 (保护下丘脑-垂体-性腺轴与甲状腺基础代谢)", result.safety_floor_kcal);
            println!("* 未受限计算预算:  {:.0} kcal/天", result.unconstrained_budget_kcal);
            println!("* 今日最终动态预算: {:.0} kcal/天", result.daily_budget_kcal);
            if result.safety_floor_triggered {
                println!("  >>> 【注意】当前已触发安全红线强制钳制保护！已锁死在最低安全底线。");
            }

            println!("\n--- 4. 高蛋白均衡临床推荐宏量素分配 ---");
            println!(
                "  * 蛋白质: {:.0} g ({:.0} kcal, 锚定去脂体重保护肌肉防流失)",
                result.recommended_protein_g,
                result.recommended_protein_g * 4.0
            );
            println!(
                "  * 脂  肪: {:.0} g ({:.0} kcal, 保障必需脂肪酸与内分泌合成下限)",
                result.recommended_fat_g,
                result.recommended_fat_g * 9.0
            );
            println!(
                "  * 碳水化合物: {:.0} g ({:.0} kcal, 供给日常脑力与运动肌糖原储备)",
                result.recommended_carbs_g,
                result.recommended_carbs_g * 4.0
            );

            if !result.clinical_notes.is_empty() {
                println!("\n--- 5. 临床代谢诊断与指导建议 ---");
                for note in &result.clinical_notes {
                    println!("  * {}", note);
                }
            }
            println!();
        }
    }
}

/// 处理 `food-custom` 指令：管理本地自建食物（create 录入 / list 列表 / delete 删除）。
fn handle_food_custom(args: &[String]) {
    if args.is_empty() {
        println!(r#"
用法: litebalance food-custom <SUBCOMMAND> [OPTIONS]

子指令:
  create    从食品包装标签录入新自建食物 (支持每 100g 模式或单份 Serving 模式，自动精确归一化)
  list      查看所有用户本地自建食物
  delete    根据食物 ID 删除自建食物

参数 (create):
  --name <NAME>            食品名称 (必填)
  --brand <BRAND>          生产商 / 品牌 (可选)
  --barcode <BARCODE>      自定义条形码 (可选)
  --basis <100g|serving>   录入基准模式 (默认: 若提供 --serving-size 则为 serving，否则为 100g)
  --serving-size <AMOUNT>  单份重量/体积 (例如 35.0)
  --serving-unit <UNIT>    单份单位 (默认: g)
  --energy <KCAL>          能量 (必填，基于录入基准数值)
  --protein <G>            蛋白质 (必填，单位 g)
  --carbs <G>              碳水化合物 (必填，单位 g)
  --fat <G>                脂肪 (必填，单位 g)
  --fiber <G>              膳食纤维 (可选，单位 g)
  --sodium <MG>            钠 (可选，单位 mg)
  --sugars <G>             糖 (可选，单位 g)
  --saturated-fat <G>      饱和脂肪 (可选，单位 g)
  --potassium <MG>         钾 (可选，单位 mg)
  --calcium <MG>           钙 (可选，单位 mg)
  --iron <MG>              铁 (可选，单位 mg)

示例:
  litebalance food-custom create --name "高蛋白乳清曲奇" --brand "家庭烘焙" --serving-size 60 --energy 220 --protein 18 --carbs 24 --fat 5.5
  litebalance food-custom list
  litebalance food-custom delete custom_abc123
"#);
        return;
    }

    let subcmd = args[0].to_lowercase();
    let storage = open_app_storage();

    match subcmd.as_str() {
        "create" => {
            let name = match get_arg_val(args, "--name") {
                Some(n) => n,
                None => {
                    eprintln!("错误: 必须提供食品名称 (--name)。");
                    return;
                }
            };
            let brand = get_arg_val(args, "--brand");
            let barcode = get_arg_val(args, "--barcode");
            let energy: f64 = get_arg_val(args, "--energy")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let protein: f64 = get_arg_val(args, "--protein")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let carbs: f64 = get_arg_val(args, "--carbs")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            let fat: f64 = get_arg_val(args, "--fat")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);

            let fiber = get_arg_val(args, "--fiber").and_then(|v| v.parse().ok());
            let sodium = get_arg_val(args, "--sodium").and_then(|v| v.parse().ok());
            let sugars = get_arg_val(args, "--sugars").and_then(|v| v.parse().ok());
            let saturated_fat = get_arg_val(args, "--saturated-fat").and_then(|v| v.parse().ok());
            let potassium = get_arg_val(args, "--potassium").and_then(|v| v.parse().ok());
            let calcium = get_arg_val(args, "--calcium").and_then(|v| v.parse().ok());
            let iron = get_arg_val(args, "--iron").and_then(|v| v.parse().ok());

            let serving_size = get_arg_val(args, "--serving-size").and_then(|v| v.parse::<f64>().ok());
            let serving_unit = get_arg_val(args, "--serving-unit").unwrap_or_else(|| "g".to_string());
            let basis_str = get_arg_val(args, "--basis").unwrap_or_else(|| {
                if serving_size.is_some() {
                    "serving".to_string()
                } else {
                    "100g".to_string()
                }
            });

            let basis = if basis_str == "serving" || serving_size.is_some() {
                let sz = serving_size.unwrap_or(100.0);
                Some(InputBasis::PerServing {
                    serving_amount: sz,
                    unit: serving_unit,
                })
            } else {
                Some(InputBasis::Per100g)
            };

            let draft = CustomFoodDraft {
                name,
                brand,
                barcode,
                basis,
                energy_kcal: energy,
                proteins_g: protein,
                carbs_g: carbs,
                fat_g: fat,
                sugars_g: sugars,
                saturated_fat_g: saturated_fat,
                fiber_g: fiber,
                sodium_mg: sodium,
                potassium_mg: potassium,
                calcium_mg: calcium,
                iron_mg: iron,
                ..Default::default()
            };

            match CustomFoodEngine::normalize_draft(&draft) {
                Ok(norm) => {
                    match storage.insert_food(&norm.food, &norm.nutriments_100g) {
                        Ok(_) => {
                            println!("\n=======================================================");
                            println!("  [成功录入本地自建食物]");
                            println!("=======================================================");
                            println!("  * 食物 ID:       {}", norm.food.id);
                            println!("  * 食品名称:      {}", norm.food.name);
                            if let Some(b) = &norm.food.brand {
                                println!("  * 品牌/产地:     {}", b);
                            }
                            if let (Some(q), Some(u)) = (norm.food.serving_quantity, &norm.food.serving_unit) {
                                println!("  * 包装单份份量:  {} {}", q, u);
                            }
                            println!("  * 每100g标准归一化营养素:");
                            println!("      - 能量:       {:.1} kcal", norm.nutriments_100g.energy_kcal_100);
                            println!("      - 蛋白质:     {:.1} g", norm.nutriments_100g.proteins_100);
                            println!("      - 碳水化合物: {:.1} g", norm.nutriments_100g.carbohydrates_100);
                            println!("      - 脂肪:       {:.1} g", norm.nutriments_100g.fat_100);
                            if let Some(fib) = norm.nutriments_100g.fiber_100 {
                                println!("      - 膳食纤维:   {:.1} g", fib);
                            }
                            if let Some(sod) = norm.nutriments_100g.sodium_mg_100 {
                                println!("      - 钠:         {:.1} mg", sod);
                            }
                            println!("  * 全文检索索引:  已自动同步至 FTS5 全文索引，随时可使用 `search` 指令检索！");
                            println!();
                        }
                        Err(e) => eprintln!("存储自建食物失败: {}", e),
                    }
                }
                Err(e) => eprintln!("自建食物参数校验归一化失败: {}", e),
            }
        }
        "list" => {
            match storage.list_custom_foods() {
                Ok(foods) => {
                    println!("\n=======================================================");
                    println!("  用户本地自建食物列表 (共 {} 项)", foods.len());
                    println!("=======================================================");
                    if foods.is_empty() {
                        println!("暂无自建食物。使用 `litebalance food-custom create` 录入第一款食品。");
                    } else {
                        println!("{:<24} | {:<16} | {:<8} | {:<8} | {:<8} | {:<8}", "ID", "名称", "能量/100g", "蛋白", "碳水", "脂肪");
                        println!("{:-<24}-+-{:-<16}-+-{:-<8}-+-{:-<8}-+-{:-<8}-+-{:-<8}", "", "", "", "", "", "");
                        for f in foods {
                            println!(
                                "{:<24} | {:<16} | {:>6.0} k | {:>6.1}g | {:>6.1}g | {:>6.1}g",
                                f.food.id,
                                if f.food.name.chars().count() > 14 { format!("{}...", f.food.name.chars().take(12).collect::<String>()) } else { f.food.name },
                                f.nutriments.energy_kcal_100,
                                f.nutriments.proteins_100,
                                f.nutriments.carbohydrates_100,
                                f.nutriments.fat_100
                            );
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("查询自建食物失败: {}", e),
            }
        }
        "delete" => {
            if args.len() < 2 {
                eprintln!("错误: 请指定要删除的食物 ID。例如: litebalance food-custom delete custom_abc");
                return;
            }
            let food_id = &args[1];
            match storage.delete_custom_food(food_id) {
                Ok(true) => println!("\n成功删除自建食物: {}", food_id),
                Ok(false) => println!("\n未找到对应自建食物或无权限删除: {}", food_id),
                Err(e) => eprintln!("删除自建食物失败: {}", e),
            }
        }
        _ => {
            eprintln!("未知子指令: '{}'。可用子指令: create, list, delete", subcmd);
        }
    }
}

/// 处理 `unit` 指令：多单位制公英制临床级双向换算。
fn handle_unit(args: &[String]) {
    if args.is_empty() {
        println!(r#"
用法: litebalance unit [SUBCOMMAND] [OPTIONS]

选项 / 参数:
  --value <NUMBER>     待换算数值 (例如 80, 175, 2000, 100)
  --from <UNIT>        源单位 (支持: kg, lbs, st, cm, in, ft, kcal, kj, g, oz, ml, floz)

快捷格式:
  litebalance unit <VALUE> <FROM_UNIT>

示例:
  litebalance unit 80 kg
  litebalance unit 175 cm
  litebalance unit 2000 kcal
  litebalance unit 100 g
  litebalance unit 500 ml
"#);
        return;
    }

    // 解析数值与单位
    let (val, unit_raw) = if let (Some(v_str), Some(u_str)) = (get_arg_val(args, "--value"), get_arg_val(args, "--from")) {
        (v_str.parse::<f64>().ok(), Some(u_str))
    } else {
        let v = args[0].parse::<f64>().ok();
        let u = if args.len() > 1 { Some(args[1].clone()) } else { None };
        (v, u)
    };

    let value = match val {
        Some(v) => v,
        None => {
            eprintln!("错误: 无法解析数值。请检查输入，例如: litebalance unit 80 kg");
            return;
        }
    };

    let unit = match unit_raw {
        Some(u) => u.trim().to_lowercase(),
        None => {
            eprintln!("错误: 必须提供源单位 (--from 或第二个参数)，如 kg, cm, kcal, g, ml");
            return;
        }
    };

    println!("\n=======================================================");
    println!("  轻衡 (LiteBalance) 多单位制高精度换算工作台");
    println!("=======================================================");

    match unit.as_str() {
        // 体重
        "kg" | "kgs" | "公斤" | "千克" => {
            let lbs = UnitConverter::kg_to_lbs(value);
            let (st, rem_lb) = UnitConverter::kg_to_stone_lbs(value);
            println!("输入值:       {:.2} kg (公制千克)", value);
            println!("换算结果 (英美制/UK):");
            println!("  * 磅 (Pounds):      {:.2} lbs", lbs);
            println!("  * 英石 (Stone+lb):  {} st {:.1} lb", st, rem_lb);
        }
        "lb" | "lbs" | "pound" | "pounds" | "磅" => {
            let kg = UnitConverter::lbs_to_kg(value);
            let (st, rem_lb) = UnitConverter::kg_to_stone_lbs(kg);
            println!("输入值:       {:.2} lbs (英制磅)", value);
            println!("换算结果:");
            println!("  * 千克 (Kilograms): {:.2} kg", kg);
            println!("  * 英石 (Stone+lb):  {} st {:.1} lb", st, rem_lb);
        }
        "st" | "stone" | "英石" => {
            let kg = UnitConverter::stone_lbs_to_kg(value as u32, 0.0);
            let lbs = UnitConverter::kg_to_lbs(kg);
            println!("输入值:       {:.0} st (英石)", value);
            println!("换算结果:");
            println!("  * 千克 (Kilograms): {:.2} kg", kg);
            println!("  * 磅 (Pounds):      {:.2} lbs", lbs);
        }

        // 身高 / 长度
        "cm" | "厘米" | "公分" => {
            let (ft, inch) = UnitConverter::cm_to_feet_inches(value);
            let total_in = UnitConverter::cm_to_inches(value);
            println!("输入值:       {:.1} cm (公制厘米)", value);
            println!("换算结果 (英制身高):");
            println!("  * 英尺+英寸:        {}", UnitConverter::format_feet_inches(ft, inch));
            println!("  * 纯英寸 (Inches):  {:.1} in", total_in);
        }
        "in" | "inch" | "inches" | "英寸" => {
            let cm = UnitConverter::inches_to_cm(value);
            let ft = (value / 12.0).floor() as u32;
            let rem_in = (value % 12.0).round() as u32;
            println!("输入值:       {:.1} in (英寸)", value);
            println!("换算结果:");
            println!("  * 厘米 (Centimeters): {:.1} cm", cm);
            println!("  * 英尺+英寸:          {} ft {} in", ft, rem_in);
        }
        "ft" | "feet" | "英尺" => {
            let cm = UnitConverter::feet_inches_to_cm(value as u32, 0);
            println!("输入值:       {:.0} ft (英尺)", value);
            println!("换算结果:");
            println!("  * 厘米 (Centimeters): {:.1} cm", cm);
            println!("  * 纯英寸:             {:.1} in", value * 12.0);
        }

        // 能量
        "kcal" | "cal" | "大卡" | "千卡" => {
            let kj = UnitConverter::kcal_to_kj(value);
            println!("输入值:       {:.1} kcal (大卡/千卡)", value);
            println!("换算结果:");
            println!("  * 千焦 (Kilojoules): {:.1} kJ (标准系数 4.184 kJ/kcal)", kj);
        }
        "kj" | "千焦" => {
            let kcal = UnitConverter::kj_to_kcal(value);
            println!("输入值:       {:.1} kJ (千焦)", value);
            println!("换算结果:");
            println!("  * 大卡 (Kilocalories): {:.1} kcal", kcal);
        }

        // 食物质量 / 重量
        "g" | "克" => {
            let oz = UnitConverter::g_to_oz(value);
            println!("输入值:       {:.1} g (公制克)", value);
            println!("换算结果:");
            println!("  * 盎司 (Ounces):    {:.2} oz (常衡制 28.3495 g/oz)", oz);
        }
        "oz" | "ounce" | "盎司" => {
            let g = UnitConverter::oz_to_g(value);
            println!("输入值:       {:.2} oz (常衡盎司)", value);
            println!("换算结果:");
            println!("  * 克 (Grams):       {:.1} g", g);
        }

        // 食物体积 / 液体
        "ml" | "毫升" => {
            let fl_oz = UnitConverter::ml_to_fl_oz(value);
            println!("输入值:       {:.1} ml (公制毫升)", value);
            println!("换算结果:");
            println!("  * 美制液体盎司:     {:.2} fl oz (美制 29.5735 ml/fl oz)", fl_oz);
        }
        "fl oz" | "floz" | "fl_oz" | "液体盎司" => {
            let ml = UnitConverter::fl_oz_to_ml(value);
            println!("输入值:       {:.2} fl oz (液体盎司)", value);
            println!("换算结果:");
            println!("  * 毫升 (Milliliters): {:.1} ml", ml);
        }

        _ => {
            eprintln!("不支持的单位类型: '{}'。支持的单位包括: kg, lbs, st, cm, in, ft, kcal, kj, g, oz, ml, floz", unit);
        }
    }
    println!();
}

/// 处理 `user` 指令：多用户档案与关联数据管理（list 列表 / delete 级联清空）。
fn handle_user(args: &[String]) {
    if args.is_empty() {
        println!(r#"
用法: litebalance user <SUBCOMMAND> [OPTIONS]

子指令:
  list              列出所有已注册用户档案
  delete <USER_ID>  永久删除指定用户及其所有关联打卡数据（摄入、体重、饮水、断食、运动、目标等）

示例:
  litebalance user list
  litebalance user delete default_user
"#);
        return;
    }

    let storage = open_app_storage();
    let subcmd = args[0].to_lowercase();

    match subcmd.as_str() {
        "list" => {
            match storage.list_all_users() {
                Ok(users) => {
                    println!("\n=======================================================");
                    println!("  已注册用户档案列表 (共 {} 位)", users.len());
                    println!("=======================================================");
                    if users.is_empty() {
                        println!("当前数据库中无任何用户档案。");
                    } else {
                        println!("{:<28} {:<16} {:<12} {:<8} {:<12}", "ID", "姓名", "身高(cm)", "体重(kg)", "活动水平");
                        println!("{:-<80}", "");
                        for u in users {
                            println!(
                                "{:<28} {:<16} {:<12.1} {:<8.1} {}",
                                u.id, u.name, u.height_cm, u.weight_kg, u.activity_level
                            );
                        }
                    }
                    println!();
                }
                Err(e) => eprintln!("查询用户列表失败: {}", e),
            }
        }
        "delete" => {
            if args.len() < 2 {
                eprintln!("错误: 请指定要删除的用户 ID。例如: litebalance user delete default_user");
                return;
            }
            let target_user_id = &args[1];
            println!("\n警告: 此操作将永久删除用户 '{}' 及其所有关联数据（摄入、体重、饮水、断食、运动记录等），不可撤销！", target_user_id);
            match storage.delete_all_user_data(target_user_id) {
                Ok(true) => println!("已成功级联清空并删除用户: {}\n", target_user_id),
                Ok(false) => eprintln!("未找到指定用户 ID: {}\n", target_user_id),
                Err(e) => eprintln!("删除用户数据失败: {}\n", e),
            }
        }
        _ => {
            eprintln!("未知子指令: '{}'。可用子指令: list, delete", subcmd);
        }
    }
}
