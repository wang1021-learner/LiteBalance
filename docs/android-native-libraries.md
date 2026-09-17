# Android 官方原生库全景参考清单 (AndroidX & Jetpack Ecosystem)

> 整理汇总 Google 官方（AndroidX / Jetpack / ML Kit / NDK）及 JetBrains 官方维护的原生库技术特性、Gradle 依赖坐标与典型应用场景，供后续技术调研与模块扩展参考。

---

## 一、 UI 界面、视觉动效与交互类库

涵盖 Google 官方在 Jetpack Compose 及底层图形系统中的全部 UI 相关扩展：

### 1. 动效、物理流变与电影感转场 (Animations & Morphing)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Graphics Shapes** | `androidx.graphics:graphics-shapes` | **多边形与液体形态平滑流变库**。<br/>支持任意几何图形之间的有机流变（Morphing），例如圆角卡片向圆形或多角形的连续平滑形变。 |
| **Shared Element** | `androidx.compose.animation:animation` | **共享元素无缝连续展开** (Compose 1.7+)。<br/>支持组件跨页面或局部区域的原地连贯拉伸展开动画。 |
| **Animated Graphics** | `androidx.compose.animation:animation-graphics` | **矢量图形动态形变 (AnimatedVectorDrawable)**。<br/>支持带物理阻尼或逐帧形变的矢量微动画（如水滴波浪充盈、状态切换矢量微动效）。 |

### 2. 底层着色器、图形绘制与色彩提取 (Graphics & Shaders)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Compose UI Graphics** | `androidx.compose.ui:ui-graphics` | **硬件着色器 (AGSL) 与底层几何绘制**。<br/>支持 AGSL (Android Graphics Shading Language) 自定义着色器、`PathMeasure`（路径描边动态流光）、图层混合模式 `BlendMode`。 |
| **Palette** | `androidx.palette:palette-ktx` | **图像色彩智能分析与主色提取**。<br/>从位图（如食物照片）中动态提取主色调、明暗色相与活力色，用于动态生成背景氛围色。 |

### 3. 高级排版与自适应布局 (Layouts & Flow)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Flow Layouts** | `androidx.compose.foundation:foundation-layout` | **自适应流式换行布局 (`FlowRow` / `FlowColumn`)**。<br/>根据子项宽度自动流式折行，支持行数限制、溢出折叠展开与对齐控制。 |
| **ConstraintLayout Compose** | `androidx.constraintlayout:constraintlayout-compose` | **复杂相对锚点约束排版**。<br/>提供 Guideline 辅助线、Barrier 屏障与 Chain 链条机制，适用于复杂的非对称多维相对锚定布局。 |

### 4. 触控手势、阻尼抽屉与反馈 (Gestures & Sheets)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **ModalBottomSheet** | `androidx.compose.material3:material3` | **自适应半屏/全屏阻尼拖拽抽屉**。<br/>支持手势惯性吸附、多停靠位状态管理与背景遮罩。 |
| **NestedScroll** | `androidx.compose.ui.input.nestedscroll:nestedscroll` | **嵌套滚动与视差视效联动**。<br/>协调父子组件滚动事件，支持视差滚动拉伸、滚动时底栏自动浮动隐藏等交互。 |
| **LocalHapticFeedback** | `androidx.compose.ui:ui` | **系统硬件级触觉微震动**。<br/>提供预置的文本选择、手柄吸附、警示碰撞等触觉反馈模式。 |
| **PullToRefresh** | `androidx.compose.material3:material3` | **M3 规范原生下拉刷新指示器**。<br/>支持容器悬浮式或嵌入式刷新手势联动。 |

### 5. 折叠屏、大屏与自适应分栏 (Adaptive & Windowing)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Adaptive Layouts** | `androidx.compose.material3.adaptive:adaptive`<br/>`androidx.compose.material3.adaptive:adaptive-navigation` | **大屏与双栏自适应脚手架**。<br/>提供 `ListDetailPaneScaffold`，自动根据窗口尺寸在单列滑动与双栏并排交互间无缝自适应切换。 |
| **WindowManager** | `androidx.window:window` | **折叠屏铰链与设备形态感知**。<br/>感知屏幕物理折叠夹角、半折悬停态（Tabletop mode）与双屏展开状态。 |

---

## 二、 健康生态、穿戴与端侧视觉库

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Health Connect<br/>(健康连接)** | `androidx.health.connect:connect-client` | **Android 官方健康数据中枢**。<br/>双向读写步数、心率、睡眠质量、运动消耗、体重体脂等数十种健康指标，统一对接主流智能手环与体脂秤。 |
| **Glance<br/>(桌面微件)** | `androidx.glance:glance-appwidget` | **声明式 Compose 桌面小组件开发套件**。<br/>用 Compose 语法开发手机桌面 AppWidget，低功耗常驻展示关键数据。 |
| **ML Kit Barcode** | `com.google.mlkit:barcode-scanning` | **离线设备端条码与二维码扫描**。<br/>毫秒级解析 EAN-13、UPC-A、QR Code 等全标准条形码，完全在设备本地完成运算，无需网络连接。 |
| **ML Kit Text OCR** | `com.google.mlkit:text-recognition` | **离线设备端文本 OCR 识别**。<br/>本地识别拍摄图像中的文字结构（如食品配料表、营养成分表）。 |
| **CameraX** | `androidx.camera:camera-camera2`<br/>`androidx.camera:camera-view` | **官方设备相机抽象套件**。<br/>消除不同 Android 厂商机型在相机对焦、曝光、分辨率适配上的碎片化问题。 |

---

## 三、 本地持久化、后台调度与架构类库

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **WorkManager** | `androidx.work:work-runtime-ktx` | **系统级后台任务调度器**。<br/>支持满足网络、充电、电量等特定硬件约束下的可靠延迟作业、周期性定时任务及锁屏状态下的唤醒执行。 |
| **DataStore** | `androidx.datastore:datastore-preferences` | **现代响应式键值存储**。<br/>基于 Kotlin Flow 与协程构建的类型安全轻量存储，替代旧版 SharedPreferences。 |
| **Paging 3** | `androidx.paging:paging-compose` | **海量数据集流式分页加载**。<br/>支持大数据集的分页分块加载、下拉刷新与滚动位置恢复。 |
| **Lifecycle & ViewModel**| `androidx.lifecycle:lifecycle-viewmodel-compose` | **生命周期与状态持久化**。<br/>管理界面配置变更（如屏幕翻转、后台重建）过程中的数据状态。 |
| **Biometric** | `androidx.biometric:biometric` | **硬件级生物指纹与面部认证**。<br/>调用系统级生物硬件对话框，支持应用入口安全锁或敏感数据验权。 |
| **Security Crypto** | `androidx.security:security-crypto` | **Keystore 硬件主密钥加密封装**。<br/>基于芯片级 KeyStore 安全隔离区提供文件与键值的高强度硬件加密。 |

---

## 四、 编译优化、启动与性能分析工具库

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Baseline Profiles** | `androidx.profileinstaller:profileinstaller` | **AOT (Ahead-of-Time) 提前编译配置文件**。<br/>在应用安装阶段直接将热点代码预编译为机器码，显著提升应用冷启动速度，降低首屏滑动丢帧率。 |
| **Macrobenchmark** | `androidx.benchmark:benchmark-macro-junit4` | **宏观性能基准压测套件**。<br/>自动化量化与监控应用启动耗时、120Hz 滚动丢帧（Jank Rate）与资源占用。 |
| **Core Splashscreen** | `androidx.core:core-splashscreen` | **Android 12+ 启动引导过渡画面规范**。<br/>平滑衔接系统桌面到首屏界面的冷启动过渡动画，消除启动闪烁。 |

---

## 五、 Kotlin 语言官方扩展库 (JetBrains)

| 库名称 | Gradle 依赖坐标 | 核心技术能力与典型场景 |
| :--- | :--- | :--- |
| **Coroutines Android** | `org.jetbrains.kotlinx:kotlinx-coroutines-android` | 官方协程 Android 调度器（`Dispatchers.Main`、`Dispatchers.IO`）。 |
| **Serialization** | `org.jetbrains.kotlinx:kotlinx-serialization-json` | 官方编译器插件级 JSON 序列化工具，无 Java 反射开销。 |
| **DateTime** | `org.jetbrains.kotlinx:kotlinx-datetime` | 官方跨平台时间日期计算库。 |
