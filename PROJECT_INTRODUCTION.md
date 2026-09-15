# 轻衡 (LiteBalance) 原生移动端应用技术架构与全景说明书

> **产品定位**：面向**原生 Android 与原生 iOS 移动端**的新一代临床级人体代谢动力学与高精度精准营养健康应用。  
> **技术架构**：**跨平台统一 Rust 核心引擎 (Shared Rust Core)** + **双端原生 UI (Android Jetpack Compose + iOS SwiftUI)** + **UniFFI 强类型安全桥接**。  
> **核心标准**：NASEM 2023 DRI | NIH Kevin Hall ODE | Pontzer-Trexler 2026 | GS1 Modulo 10 | SQLite FTS5 | ODbL 物理隔离

---

## 1. 项目定位与架构愿景

### 1.1 为什么不做跨平台混合框架，而是选择“Rust 共享核心 + 双端原生 UI”？

本项目从第一天起就按「共享 Rust 计算核心 + 双端原生 UI」设计，而不是事后用跨端 UI 框架硬扛临床级代谢数值：

1. **无 GC 抖动**：Kevin Hall 动态超微方程等重计算在原生代码路径执行，避免托管运行时的停顿与发热；
2. **单一计算真相源**：iOS / Android 共用同一套 Rust 核心，代谢预测与营养预算结果一致；
3. **本地优先**：用户数据落在设备沙箱内的 SQLite，核心能力可离线使用；
4. **原生壳层**：SwiftUI / Jetpack Compose 直接对接系统能力（HealthKit、Health Connect、灵动岛、小组件等）。

**轻衡 (LiteBalance)** 采用 **「共享原生内核 + 双端独立原生壳层」** 架构。

`mermaid
graph TD
    subgraph UI_Layer [移动双端原生 UI 交互层]
        iOS[iOS 原生 App: SwiftUI<br/>- Dynamic Island 灵动岛<br/>- Live Activity 实时活动<br/>- Apple HealthKit 深度整合<br/>- 120Hz ProMotion 极度丝滑]
        Android[Android 原生 App: Jetpack Compose<br/>- Material 3 / Material You 动态取色<br/>- Glance 桌面微件<br/>- Health Connect 健康连接<br/>- 高帧率原生效能]
        CLI[开发者/无头调试工作台 (main.rs)<br/>- 算法批量回测与基准仿真<br/>- 数据导入导出检验]
    end

    subgraph Bridge_Layer [跨平台强类型桥接层 (UniFFI / Swift-Bridge)]
        iOS -->|Swift Package / XCFramework| UniFFI[UniFFI 安全绑定契约]
        Android -->|Kotlin 绑定 / AAR JNI| UniFFI
        CLI -->|直接引用| Core
        UniFFI --> Core[轻衡 LiteBalance 统一核心引擎 (Rust 2024)]
    end

    subgraph Calc_Layer [临床级代谢与营养计算层 (扁平模块)]
        Core --> Energy[NASEM 2023 能量代谢矩阵]
        Core --> ODE[NIH Kevin Hall 动态体重常微分方程]
        Core --> Pontzer[Pontzer-Trexler 2026 运动代偿模型]
        Core --> Activity[Herrmann 2024 运动与能量闭环天平]
        Core --> Goal[自适应目标预算调节器 (OLS斜率纠偏)]
        Core --> Boundary[跨午夜生理日界线算法]
        Core --> CustomFood[自建食物包装双基准归一化]
        Core --> UnitSys[多单位制公英制复合转换]
        Core --> Radar[NASEM DRI 微量元素雷达防虚警]
        Core --> Macro[去脂体重锚定宏量协议与碳循环]
        Core --> Fasting[间歇性断食无状态状态机]
        Core --> Water[临床补水生理转化]
        Core --> Report[周期性代谢偏离度报表]
    end

    subgraph Storage_Layer [物理双分库存储层 (扁平模块)]
        Core --> MainDB[(主库: litebalance.db<br/>用户沙盒目录)]
        MainDB --> Tables[用户档案/打卡/体重/自建库/目标]
        MainDB --> FTS5[FTS5 离线全文索引]
        Core --> CacheDB[(独立网络缓存库: food_cache.db)]
        CacheDB --> OFFCache[OpenFoodFacts 30天TTL缓存]
    end

    subgraph Client_Layer [外部网络服务与客户端 (扁平模块)]
        Core --> Barcode[GS1 模10条码校验与自动升格]
        Barcode --> OFFClient[OpenFoodFacts 客户端]
        OFFClient --> RateLimit[滑动窗口限速器 (14次/分)]
        OFFClient --> CacheDB
    end
```

---

## 2. 核心架构优势与移动端设计考量

### 2.1 唯一计算真理源（Single Source of Truth）
* 临床级营养计算包含极其复杂的数学模型（微分方程求解器、8 分组多项式多维矩阵、Pontzer 非线性阻尼、自适应着陆平滑算法）。
* 若在 iOS (Swift) 和 Android (Kotlin) 各自手写一套，必然导致**浮点精度偏差、跨端计算结果不一致、双倍测试与长期维护噩梦**。
* 通过 Rust 统一实现核心，确保无论用户在 iPhone 还是 Android 设备上使用，所有代谢预测、卡路里预算、微量元素评估与能量闭环结算结果**100% 绝对一致**。

### 2.2 移动端极致性能与超低电池功耗
* 移动设备对 CPU 功耗和内存极度敏感。
* Rust 编译为原生机器码，**零垃圾回收（No GC）开销**，无内存泄漏风险，冷启动亚毫秒级完成。
* 内存占用极轻（核心常驻内存 $< 15\text{ MB}$），保证双端 UI 无论在 60Hz 还是 120Hz 刷新率下均如丝般顺滑，后台同步与打卡运算零发热。

### 2.3 本地优先（Local-First）与绝对隐私
* 移动设备的健康数据（体重、饮食习惯、体脂、作息规律）属于极度敏感的个人隐私。
* 轻衡坚持**全量数据保存在手机本地沙盒内部**（iOS `Application Support / Documents`，Android `context.filesDir`）。
* 内嵌 SQLite 物理双分库与 FTS5 全文索引，无网络离线状态下所有检索、打卡、图表、微分仿真均 100% 可用，无需依赖任何后端服务器。

### 2.4 零数据污染原则（Zero-Pollution Data Model）
* 食品标签中未标注的微量元素严格保留为 `None`，绝不在移动端存储或展示时盲目补 `0.0`。
* 临床微量元素雷达评估内置 50% 覆盖率门槛，未达标时显示“数据不全”而非误报缺乏症，杜绝引起用户的虚假健康焦虑。

---

## 3. 核心计算与领域能力矩阵

| 模块名称 | 核心文件 | 移动端产品核心价值 |
| :--- | :--- | :--- |
| **能量代谢引擎** | [`energy.rs`](src/energy.rs) | 基于 NASEM 2023 最新 DRI 标准，结合成人 8 分组多项式方程，准确预测每日静息维持能耗（TDEE）。 |
| **动态体重 ODE** | [`dynamic_weight.rs`](src/dynamic_weight.rs) | 美国国家卫生研究院（NIH）Kevin Hall 常微分方程模型，精准模拟人体减重平台期与瘦体重演化。 |
| **运动能量代偿** | [`workout_compensation.rs`](src/workout_compensation.rs) | Pontzer-Trexler 2026 前沿模型，区分名义毛消耗与真实净消耗，扣除代偿挤压，防止高估消耗诱发暴食反弹。 |
| **运动活动闭环** | [`activity.rs`](src/activity.rs) | 涵盖 Herrmann 2024 MET 标准运动库，打通每日“摄入 - 基础能耗 - 运动 - 代偿 = 真实净能量差”闭环天平。 |
| **自适应目标调节** | [`goal_profile.rs`](src/goal_profile.rs) | 临床自适应动态预算（-0.25~-1.0 kg/周），结合实测体重 OLS 回归斜率自动调节，平稳着陆缓冲，守护内分泌红线（男 $\ge 1500$ / 女 $\ge 1200$ kcal）。 |
| **跨午夜生理日界线** | [`day_boundary.rs`](src/day_boundary.rs) | 支持自定义日界线（如凌晨 04:00），将通宵夜班或熬夜宵夜按生理清醒周期归集至前一逻辑日，彻底根除日历漂移。 |
| **本地自建食物全流程** | [`custom_food.rs`](src/custom_food.rs) | 支持包装单份（Serving）或每 100g 模式录入，自动等比精确归一化，零污染保留未检微量，自动同步 FTS5 离线检索。 |
| **多单位制转换系统** | [`unit_system.rs`](src/unit_system.rs) | 公制（Metric）、英制（Imperial / US）、英石复合进位（UK Stone + lb）、身高（ft + in）、大卡/千焦、盎司/毫升双向高精度换算。 |
| **微量元素达标雷达** | [`micronutrient_eval.rs`](src/micronutrient_eval.rs) | 对标 NASEM DRI（RDA / AI / UL / CDRR），内置 50% 覆盖率防误报门限，精准诊断钠过量及矿物质达成度。 |
| **周期性报表分析** | [`analytics_reporting.rs`](src/analytics_reporting.rs) | 周期性代谢偏离度诊断、实际热量缺口达成率统计、三大宏量摄入饼图及断食依从率分析。 |
| **外部食品库与条形码** | [`remote_food_client.rs`](src/remote_food_client.rs) | GS1 模 10 条形码校验（EAN-13/UPC-A）、OpenFoodFacts API 客户端，合规 User-Agent 与滑动窗口防封限速。 |

---

## 4. 移动端工程目录组织

```text
D:\nutritracker-core\
├── Cargo.toml                       # 核心库配置 (支持 rlib / cdylib)
├── src/
│   ├── lib.rs                       # 核心公共 API 导出总入口
│   ├── main.rs                      # 开发者无头测试 CLI 工作台 (非最终端产品，用于 PC 调试回测)
│   ├── models/                      # 基础生理实体模型 (UserProfile, Gender, ActivityLevel)
│   ├── calc/                        # 临床级代谢与营养计算层 (13 个纯函数计算引擎)
│   │   ├── energy.rs                # NASEM 2023 能量代谢计算
│   │   ├── dynamic_weight.rs        # NIH Kevin Hall 动态常微分方程仿真
│   │   ├── workout_compensation.rs  # Pontzer 运动代偿模型
│   │   ├── activity.rs              # 运动打卡与每日代谢天平结算
│   │   ├── goal_profile.rs          # 动态自适应目标调节器
│   │   ├── day_boundary.rs          # 跨午夜生理日界线算法
│   │   ├── custom_food.rs           # 本地自建食物归一化管理
│   │   ├── unit_system.rs           # 多单位制转换系统
│   │   ├── macro_engine.rs          # 宏量素去脂体重锚定与碳循环
│   │   ├── micronutrient_eval.rs    # NASEM DRI 微量元素达标雷达
│   │   ├── analytics_reporting.rs   # 周期性代谢偏离度报表
│   │   ├── recipe.rs                # 食谱多原料归一化与覆盖率追踪
│   │   ├── fasting.rs               # 间歇性断食无状态状态机
│   │   ├── water.rs                 # 临床补水生理模型
│   │   └── trends.rs                # 体重线性回归与营养移动均值
│   ├── storage/                     # 本地持久化层 (物理双库 + FTS5 全文索引)
│   │   ├── db.rs                    # 用户核心数据库 (litebalance.db)
│   │   ├── cache_db.rs              # 外部网络独立缓存库 (food_cache.db)
│   │   ├── schema.rs                # SQLite DDL、索引与 FTS5 触发器
│   │   ├── models.rs                # 持久化数据记录结构体
│   │   ├── seed.rs                  # 默认临床参考食物离线种子库
│   │   └── export_import.rs         # 原生 JSON 备份与老版本数据无缝迁移
│   └── client/                      # 外部数据源与条形码服务
│       ├── barcode.rs               # GS1 模 10 校验与 UPC-A 升格
│       └── remote_food_client.rs    # OpenFoodFacts API 客户端与滑动窗口限速
```

---

## 5. 开发者 CLI 工作台与移动端调试

在移动端 App 构建之前，`src/main.rs` 提供了高效的**桌面无头（Headless）开发者工作台**，方便在 PC 上直接进行算法校验、数据迁移测试与极限场景回归：

```powershell
# 1. 临床代谢规划与 NIH 动态体重仿真
cargo run -- plan --age 28 --height 178 --weight 80 --gender male --activity active --target 75 --weeks 12

# 2. 离线 FTS5 全文模糊检索
cargo run -- search 鸡胸肉

# 3. GS1 条形码智能校验与 OpenFoodFacts 检索 (支持 --save 归档入库)
cargo run -- barcode 737628064502 --save

# 4. 全天能量平衡天平结算 (支持 --boundary 凌晨跨午夜生理日界线)
cargo run -- balance --boundary 04:00
cargo run -- summary --boundary 04:00

# 5. 动态卡路里预算与自适应目标调节 (支持平台期 OLS 斜率自动纠偏)
cargo run -- goal set --type lose --target 75.0 --rate -0.5
cargo run -- goal status

# 6. 本地自建食物全流程管理 (包装单份/100g 模式归一化与 FTS5 索引)
cargo run -- food-custom create --name "高蛋白低脂能量棒" --serving-size 60 --energy 210 --protein 20 --carbs 18 --fat 6
cargo run -- food-custom list

# 7. 多单位制高精度转换 (kg/lbs/st, cm/in/ft, kcal/kJ, g/oz, ml/fl_oz)
cargo run -- unit 80 kg
cargo run -- unit 175 cm

# 8. 临床 NASEM DRI 微量元素雷达评估
cargo run -- dri

# 9. 周期性代谢偏离度与依从性报表
cargo run -- report --days 14
```

---

## 6. 质量保障与测试指标

- **自动化测试套件**：全量测试套件共计 **77 项自动化集成与单元测试**，覆盖所有医学公式、ODE 微分方程、SQLite 数据库触发器与网络客户端，**100% 测试通过（耗时 0.05 秒）**。
- **静态代码严苛规范**：`cargo clippy --all-targets -- -D warnings` **零警告通过**，完全符合 Rust 2024 工业级代码规范。

  > 注意须带 `--all-targets`：不加该参数时 clippy 不会检查 `#[cfg(test)]` 测试代码，容易漏检测试中的 lint 问题。CI 已固化为带 `--all-targets` 的严格模式，详见 `.github/workflows/ci.yml`。
- **中文化注释与文档**：所有源码注释与错误诊断全量采用专业临床中文表述。

---

## 7. 后续移动端交付路线图 (Mobile Roadmap)

1. **跨语言移动端绑定导出 (UniFFI SDK Layer)**：
   - 编写 `litebalance.udl` 或 UniFFI proc-macro 接口描述；
   - 配置 `cargo-ndk` 交叉编译 Android 目标架构（`arm64-v8a`, `armeabi-v7a`, `x86_64`）并生成 Kotlin 封装库（AAR）；
   - 配置 `cargo-lipo` / `xcodebuild` 交叉编译 iOS 架构并生成 `LiteBalanceCore.xcframework` 与 Swift Package。
2. **iOS 原生应用开发 (SwiftUI)**：
   - 构建 120Hz 极致顺滑的现代极简 UI；
   - 接入 **Apple HealthKit**：双向同步体重、步数、静息消耗与运动卡路里；
   - 接入 **灵动岛 (Dynamic Island)** 与 **实时活动 (Live Activity)**：断食倒计时与补水进度实时驻留屏幕常显；
   - 接入 **锁屏小组件 (Lock Screen Widgets)**：即时查看今日摄入与净能量差。
3. **Android 原生应用开发 (Jetpack Compose)**：
   - 采用 Material 3 原生设计语言与 Material You 动态主题取色；
   - 接入 **Health Connect (健康连接)**：跨应用同步各大运动手环与体重秤数据；
   - 接入 **Glance 桌面 AppWidget**：直观展示今日热量收支天平。
4. **移动端本地 AI Agent (Personal Nutrition Agent)**：
   - 作为移动端内的“个人专属代谢与营养管家”；
   - 挂载 Rust Core 作为确定性的 **Tool Calling** 工具层，让用户通过日常自然语言或拍照就能精准完成食物记录与自适应预算调整。
