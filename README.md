# 轻衡 LiteBalance

**轻衡（LiteBalance）** 是一款面向移动端的临床级营养与代谢追踪应用：用统一的 Rust 计算核心，给 Android / iOS 原生客户端提供一致、可离线的代谢与营养能力。

本仓库是共享核心库 **`litebalance-core`**（当前版本 `0.1.0`），并附带无头 CLI **`litebalance`**，方便在电脑上验证算法与数据流。

## 它解决什么问题

- 把复杂的能量代谢、体重轨迹、运动补偿、宏量/微量营养评估做成**可复用、可测试**的核心
- 保证双端计算结果一致（单一计算真相源），而不是在 Swift / Kotlin 里各写一套
- **本地优先**：用户饮食、体重等数据落在本机 SQLite，核心能力可离线使用
- 营养数据「零污染」：标签未标注的微量元素保持未知，不盲目填 `0`

## 适用范围（重要）

能量需求（NASEM / IOM）、动态体重规划与自适应热量预算**当前仅支持 ≥19 岁且非孕非哺成人**。  
未成年人、妊娠期、哺乳期会显式返回错误，避免误用成人方程。儿童/孕哺专用 EER 尚未接入。

## 技术概览

| 层级 | 说明 |
|------|------|
| 计算核心 | Rust 2024：能量（NASEM 2023）、动态体重（NIH Kevin Hall）、运动补偿、目标预算、日界、宏量/碳水循环、微量 DRI、食谱、断食、饮水、趋势与报表等 |
| 存储 | SQLite + FTS5 主库；外部食物缓存库 |
| 外部数据 | GS1 条码校验、OpenFoodFacts 查询 |
| 移动端方向 | UniFFI 门面 `LiteBalanceSession`（`src/ffi.rs`）+ Android Compose 壳（`apps/android`）；iOS 待 Mac（`docs/ios-next.md`） |
| 本机工具 | CLI：`plan` / `search` / `log-food` / `summary` / `dri` / `report` / `export` / `import` 等 |

## 快速开始

```bash
cargo build
cargo test
cargo run -- --help
```

质量校验（与 CI 一致）：

```bash
cargo test --all-targets
# 必须带 --all-targets，否则不检查 #[cfg(test)] 测试代码中的 lint
cargo clippy --all-targets -- -D warnings
```

> 构建要求：Rust **1.85+**（本项目使用 edition 2024）。

示例：

```bash
cargo run -- search 牛奶
cargo run -- summary
cargo run -- dri
cargo run -- export
```

## 文档

- [架构与模块详解](PROJECT_INTRODUCTION.md)
- [实现走查与验证记录](WALKTHROUGH.md)

## 仓库结构（简）

```text
├── README.md                 # 项目介绍（本文件）
├── PROJECT_INTRODUCTION.md   # 架构全文
├── WALKTHROUGH.md            # 子系统交付与验证笔记
├── docs/                     # android-build / ios-next 等
├── apps/
│   └── android/              # Jetpack Compose 壳（Android Studio 打开此目录）
├── Cargo.toml
├── rustfmt.toml              # max_width = 120
└── src/                      # 扁平模块结构，全部模块直接置于此
    ├── lib.rs                # 模块声明 + 统一公开 API re-export + UniFFI
    ├── ffi.rs                # 移动端窄门面 LiteBalanceSession
    ├── main.rs               # CLI
    └── ...                   # 计算 / 存储 / 外部客户端等
```

> 模块采用扁平结构而非 `calc/`、`storage/` 等子目录：模块总量适中，扁平化可避免
> `crate::calc::energy::EnergyCalc` 这类冗长路径。调用方统一从 crate 根引入
> （`use litebalance_core::EnergyCalc`），内部模块可自由调整而不影响外部引用。

## 状态

活跃开发中。当前以共享 Rust 核心与 CLI 验证为主；移动端壳层按路线图推进。
