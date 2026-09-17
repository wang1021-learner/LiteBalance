# Android 端功能与 M3 设计规范接入状态

| 功能模块 | 接入状态 | 说明 |
| :--- | :--- | :--- |
| **Material 3 主题系统** | ✅ 已就绪 | 完整适配 Material You 动态取色、翡翠绿种子色、Tonal 容器层级、15 级排版与形状 |
| **UniFFI 核心桥接** | ✅ 已就绪 | `CoreSession` 全面基于 Kotlin 协程与 `Dispatchers.IO`，带异常捕获与沙箱注入 |
| **底部 NavigationBar** | ✅ 已就绪 | 仪表盘、饮食日记、动态趋势、档案目标 4 大核心 Tab |
| **今日代谢天平 (焦点)** | ✅ 已就绪 | `EnergyBalanceCard`：清晰展示摄入、NASEM 2023 TDEE、Pontzer 净运动代偿及能量差 |
| **三大宏量进度条** | ✅ 已就绪 | `MacroProgressBar`：去脂体重锚定的碳水、蛋白质、脂肪三段进度与达成率 |
| **水化与断食卡片** | ✅ 已就绪 | 快速记录饮水 (+250ml) 与 16:8 间歇断食进度可视化卡片 |
| **生命阶段安全防线** | ✅ 已就绪 | `SafetyAlertBanner`：未成年与孕哺拦截容器，男 1500 / 女 1200 kcal 安全红线 |
| **FTS5 食物打卡与条码** | 📋 待接屏 | 底层 `CoreSession.searchFoods` / `logFood` 已就绪，待接入 ModalBottomSheet UI |
| **Kevin Hall 动态体重图表**| 📋 待接屏 | 底层 `weightHistory` / `computePlan` 已就绪，待绘制 Canvas / 曲线组件 |
