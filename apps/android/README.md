# 轻衡 (LiteBalance) Android 客户端

基于 **Google Material Design 3 (M3 / Material You)** 规范与 **Jetpack Compose** 构建的轻衡原生 Android 客户端。

## 架构特性

- **Material Design 3 原生设计系统**：
  - **Material You 动态取色**：在 Android 12+ (API 31+) 设备上无缝适配用户系统壁纸调色板（`dynamicLightColorScheme` / `dynamicDarkColorScheme`）。
  - **临床健康基准调色板**：在非动态配色环境启用定制的临床翡翠绿（Clinical Emerald `#006C4C`）种子色体系。
  - **Tonal Elevation 色调海拔**：弃用生硬的投影，全面使用 M3 `surfaceContainerLow` / `surfaceContainerHigh` 容器层级。
  - **15 级标准排版系统**：`displaySmall` / `headlineLarge` 用于大字号代谢数值，结合 `labelSmall` 紧凑单位呈现严谨度。
- **本地优先 & 单一计算真相源**：
  - 核心代谢动力学（NASEM 2023 DRI、Kevin Hall 动态体重常微分方程、Pontzer 运动非线性代偿）100% 运行于 Rust 核心。
  - 数据持久化于 Android 沙盒内部的 SQLite（`litebalance.db` + FTS5 全文索引）。
  - `CoreSession` 单例保证所有密集计算与磁盘 I/O 严格调度于 `Dispatchers.IO`。
- **严格的安全与生命阶段防护**：
  - 界面直观集成针对未成年人（<19岁）、孕期及哺乳期女性的成人热量公式安全拦截。
  - 守护男性 1500 kcal / 女性 1200 kcal 最低生理内分泌安全热量红线。

## 工程结构

```text
apps/android/
├── build.gradle.kts                      # 根构建脚本
├── settings.gradle.kts                   # 模块配置
├── gradle.properties                     # JVM 参数与编译配置
├── app/
│   ├── build.gradle.kts                  # Compose BOM + Material 3 + JNA
│   ├── proguard-rules.pro                # JNA 与 UniFFI 混淆保护
│   └── src/main/
│       ├── AndroidManifest.xml           # 应用程序清单
│       ├── res/values/
│       │   ├── strings.xml               # 国际化文案
│       │   └── themes.xml                # 启动过渡主题
│       ├── jniLibs/                      # 存放交叉编译的 liblitebalance_core.so
│       └── java/
│           ├── uniffi/litebalance/       # UniFFI 自动生成的 Kotlin 绑定
│           └── com/litebalance/app/
│               ├── theme/                # M3 设计系统 (Color, Type, Shape, Theme)
│               ├── core/                 # UniFFI 协程安全会话管理 (CoreSession)
│               ├── ui/
│               │   ├── components/       # M3 临床组件 (EnergyBalanceCard, MacroProgressBar)
│               │   ├── navigation/       # NavigationBar 导航路由
│               │   └── dashboard/        # 代谢天平仪表盘主屏
│               └── MainActivity.kt       # 单 Activity 入口
```

## 编译与调试

1. **准备 JNI 动态链接库**：
   按 `docs/android-build.md` 指引，使用 `cargo ndk` 编译生成 `liblitebalance_core.so`，放置于 `app/src/main/jniLibs/<abi>/`。
2. **构建 APK**：
   ```bash
   ./gradlew assembleDebug
   ```
