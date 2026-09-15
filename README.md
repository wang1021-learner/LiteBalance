# 轻衡 LiteBalance

**轻衡（LiteBalance）** 是一款面向移动端的临床级营养与代谢追踪应用：用统一的 Rust 计算核心，给 Android / iOS 原生客户端提供一致、可离线的代谢与营养能力。

本仓库是共享核心库 **`litebalance-core`**（当前版本 `0.1.0`），并附带无头 CLI **`litebalance`**，方便在电脑上验证算法与数据流。

## 它解决什么问题

- 把复杂的能量代谢、体重轨迹、运动补偿、宏量/微量营养评估做成**可复用、可测试**的核心
- 保证双端计算结果一致（单一计算真相源），而不是在 Swift / Kotlin 里各写一套
- **本地优先**：用户饮食、体重等数据落在本机 SQLite，核心能力可离线使用
- 营养数据「零污染」：标签未标注的微量元素保持未知，不盲目填 `0`

## 技术概览

| 层级 | 说明 |
|------|------|
| 计算核心 | Rust 2024：能量（NASEM 2023）、动态体重（NIH Kevin Hall）、运动补偿、目标预算、日界、宏量/碳水循环、微量 DRI、食谱、断食、饮水、趋势与报表等 |
| 存储 | SQLite + FTS5 主库；外部食物缓存库 |
| 外部数据 | GS1 条码校验、OpenFoodFacts 查询 |
| 移动端方向 | Android Jetpack Compose + iOS SwiftUI，经 UniFFI 桥接（壳层不在本仓库） |
| 本机工具 | CLI：`plan` / `search` / `log-food` / `summary` / `dri` / `report` / `export` / `import` 等 |

## 快速开始

```bash
cargo build
cargo test
cargo run -- --help
```

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
├── Cargo.toml
└── src/
    ├── calc/                 # 代谢与营养计算
    ├── storage/              # SQLite 持久化与备份
    ├── client/               # 条码 / OpenFoodFacts
    ├── models/
    ├── lib.rs
    └── main.rs               # CLI
```

## 状态

活跃开发中。当前以共享 Rust 核心与 CLI 验证为主；移动端壳层按路线图推进。
