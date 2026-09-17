# Android 绑定构建说明

轻衡核心通过 UniFFI 导出 `LiteBalanceSession`（见 `src/ffi.rs`）。  
**说明：原 `apps/android` Compose 壳已删除，等待重新设计 UI。** 以下步骤用于给新的 Android 工程生成 `.so` 与 Kotlin 绑定。

## 前置

- Rust 1.85+（edition 2024）
- Android NDK（建议 r26+）与 `cargo-ndk`：`cargo install cargo-ndk`
- UniFFI 绑定生成器：本仓库自带 `uniffi-bindgen` 二进制（`Cargo.toml` 中 `uniffi` 的 `cli` feature）

## 本机（Windows）先生成 Kotlin 绑定

```powershell
cd D:\nutritracker-core
cargo build
# Debug 产物一般为 target\debug\litebalance_core.dll
cargo run --bin uniffi-bindgen -- generate --library target\debug\litebalance_core.dll --language kotlin --out-dir bindings\android
```

将 `bindings\android` 下生成的 Kotlin 源码同步到本仓库：

`apps\android\app\src\main\java\uniffi\litebalance\`

（仓库内已包含一份与当前核心匹配的拷贝。）

## 交叉编译 `.so`（模拟器 / 真机）

```powershell
rustup target add aarch64-linux-android x86_64-linux-android
# 示例：模拟器 x86_64
cargo ndk -t x86_64 -o apps\android\app\src\main\jniLibs build --release
# 真机 arm64
cargo ndk -t arm64-v8a -o apps\android\app\src\main\jniLibs build --release
```

产物路径形如：`jniLibs/x86_64/liblitebalance_core.so`。

## App 侧约定

1. 启动时用沙箱路径打开会话：

```kotlin
val db = File(filesDir, "litebalance.db").absolutePath
val cache = File(filesDir, "food_cache.db").absolutePath
val session = LiteBalanceSession.open(db, cache)
```

2. **所有**核心调用放在后台线程（`Dispatchers.IO`），尤其是未来条码联网。
3. 能量规划仅支持 **≥19 岁且非孕非哺**；错误信息直接展示给用户。

## 黄金验收

成人女性 22 岁 / 165 cm / 63 kg / `low_active` → `computeTdee` 的 NASEM 结果约 **2275 kcal**。
