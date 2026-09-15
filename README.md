# 轻衡 LiteBalance Core

临床级人体代谢动力学与高精度营养计算的 **共享 Rust 核心**（litebalance-core）。

面向原生 **Android（Jetpack Compose）** 与 **iOS（SwiftUI）**，计划经 UniFFI 桥接；本仓库同时提供无头 CLI litebalance 用于算法回测与数据校验。

## 特性概览

- **能量与代谢**：NASEM 2023 DRI、NIH Kevin Hall 动态体重 ODE、Pontzer-Trexler 运动补偿等
- **营养记录**：饮食/饮水/体重/断食/运动活动，本地 SQLite + FTS5
- **微量营养素**：NASEM DRI 雷达（RDA / AI / UL / CDRR）
- **本地优先**：用户数据存于设备侧；支持原生 JSON 备份导出/导入，以及摄入 CSV 导出
- **外部数据**：条码 GS1 校验、OpenFoodFacts 查询与本地食物缓存

## 快速开始

`ash
cargo build
cargo test
cargo run -- --help
`

常用 CLI 示意：

`ash
cargo run -- plan
cargo run -- search <关键词>
cargo run -- summary
cargo run -- dri
cargo run -- report
cargo run -- export
cargo run -- import <backup.json>
`

## 文档

- [项目介绍与架构](docs/PROJECT_INTRODUCTION.md)
- [实现走查与验证记录](docs/WALKTHROUGH.md)

## 许可证

见仓库内 LICENSE（若尚未添加则以你后续声明为准）。
