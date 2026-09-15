# 轻衡 LiteBalance — Android

本目录是 monorepo 内的 Jetpack Compose 壳层，通过 UniFFI 调用仓库根目录的 `litebalance-core`（`LiteBalanceSession`）。

## 功能（第一期）

- 今日汇总（摄入 + TDEE）
- 代谢计划（NASEM / Kevin Hall）
- 食物搜索 + 打卡
- 目标自适应预算
- 体重 / 饮水

## 接入原生库

在**仓库根目录**操作（详见 [`docs/android-build.md`](../../docs/android-build.md)）：

```powershell
cd D:\nutritracker-core
cargo build
cargo run --bin uniffi-bindgen -- generate --library target\debug\litebalance_core.dll --language kotlin --out-dir bindings\android
# 需要 Android NDK + cargo-ndk：
cargo ndk -t x86_64 -o apps\android\app\src\main\jniLibs build --release
cargo ndk -t arm64-v8a -o apps\android\app\src\main\jniLibs build --release
```

将生成的 `litebalance.kt` 同步到：

`apps/android/app/src/main/java/uniffi/litebalance/litebalance.kt`

（本目录已包含一份与当前核心匹配的拷贝。）

用 Android Studio 打开 **`apps/android`**（不是仓库根），同步 Gradle 后运行。

## 验收

计划页默认档案应为：**NASEM ≈ 2275 kcal**（22 岁女性 / 165 cm / 63 kg / low_active）。

## 说明

- 无 `liblitebalance_core.so` 时 App 能编译，但打开会话会失败并提示错误。
- iOS 需 Mac，见 [`docs/ios-next.md`](../../docs/ios-next.md)。
